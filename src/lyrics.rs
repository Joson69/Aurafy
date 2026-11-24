//! On-demand lyrics from LRCLIB's documented public API. Spotify credentials
//! never leave the Spotify integration; this client sends track metadata only.
use crate::spotify::Track;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::{
    collections::VecDeque,
    io::Read,
    time::{Duration, Instant},
};

const BASE: &str = "https://lrclib.net";
const CACHE_SIZE: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackKey {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub seconds: u64,
}

impl TrackKey {
    pub fn for_track(track: &Track) -> Option<Self> {
        if track.uri.is_empty() || track.name.trim().is_empty() || track.artist.trim().is_empty() {
            return None;
        }
        Some(Self {
            title: track.name.clone(),
            artist: track.artist_names.first().cloned().unwrap_or_else(|| {
                track
                    .artist
                    .split(", ")
                    .next()
                    .unwrap_or(&track.artist)
                    .to_owned()
            }),
            album: track.album.clone(),
            seconds: (track.duration_ms + 500) / 1000,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Line {
    pub time_ms: u64,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct Lyrics {
    pub lines: Vec<Line>,
    pub plain: String,
    pub instrumental: bool,
}

impl Lyrics {
    pub fn active_line(&self, position_ms: u64) -> Option<usize> {
        self.lines
            .partition_point(|line| line.time_ms <= position_ms)
            .checked_sub(1)
    }
}

#[derive(Clone, Debug)]
pub struct FetchError {
    pub message: String,
    pub retry_after: Option<Duration>,
}

impl From<&str> for FetchError {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
            retry_after: None,
        }
    }
}

pub type Outcome = Result<Option<Lyrics>, FetchError>;

/// A bounded, memory-only cache. Only one fetch may run at a time, even when
/// skipping tracks quickly. Results are keyed, so old requests cannot replace
/// the currently displayed song. Failed requests require an explicit retry.
#[derive(Default)]
pub struct Library {
    entries: VecDeque<(TrackKey, Outcome)>,
    pub pending: Option<TrackKey>,
    cooldown: Option<Instant>,
}

impl Library {
    pub fn get(&self, key: &TrackKey) -> Option<&Outcome> {
        self.entries
            .iter()
            .find(|(saved, _)| saved == key)
            .map(|(_, value)| value)
    }

    pub fn wait_seconds(&self) -> u64 {
        self.cooldown
            .map(|until| {
                until
                    .saturating_duration_since(Instant::now())
                    .as_millis()
                    .div_ceil(1000) as u64
            })
            .unwrap_or(0)
    }

    pub fn begin(&mut self, key: &TrackKey) -> bool {
        if self.pending.is_some() || self.get(key).is_some() || self.wait_seconds() > 0 {
            return false;
        }
        self.pending = Some(key.clone());
        true
    }

    pub fn finish(&mut self, key: TrackKey, result: Outcome) {
        if self.pending.as_ref() == Some(&key) {
            self.pending = None;
        }
        // Space normal requests as well as honoring server Retry-After.
        let delay = result
            .as_ref()
            .err()
            .and_then(|e| e.retry_after)
            .unwrap_or(Duration::from_millis(350));
        self.cooldown = Some(Instant::now() + delay);
        self.entries.retain(|(saved, _)| saved != &key);
        self.entries.push_back((key, result));
        while self.entries.len() > CACHE_SIZE {
            self.entries.pop_front();
        }
    }

    pub fn retry(&mut self, key: &TrackKey) {
        if self.wait_seconds() == 0 && self.pending.is_none() {
            self.entries.retain(|(saved, _)| saved != key);
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Record {
    track_name: String,
    artist_name: String,
    #[serde(deserialize_with = "null_as_default")]
    album_name: String,
    #[serde(deserialize_with = "null_as_default")]
    duration: f64,
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

impl Record {
    fn into_lyrics(self) -> Option<Lyrics> {
        let lines = parse_lrc(self.synced_lyrics.as_deref().unwrap_or_default());
        let plain = self.plain_lyrics.unwrap_or_default().trim().to_owned();
        (self.instrumental || !plain.is_empty() || !lines.is_empty()).then_some(Lyrics {
            lines,
            plain,
            instrumental: self.instrumental,
        })
    }
}

pub fn fetch(key: &TrackKey) -> Outcome {
    let client = Client::builder()
        .user_agent(concat!(
            "Aurafy/",
            env!("CARGO_PKG_VERSION"),
            " (personal desktop music client)"
        ))
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|_| FetchError::from("Could not start the lyrics connection."))?;
    fetch_from(&client, BASE, key)
}

fn fetch_from(client: &Client, base: &str, key: &TrackKey) -> Outcome {
    let mut query = vec![
        ("track_name", key.title.clone()),
        ("artist_name", key.artist.clone()),
    ];
    if !key.album.is_empty() {
        query.push(("album_name", key.album.clone()));
    }
    if (1..=3600).contains(&key.seconds) {
        query.push(("duration", key.seconds.to_string()));
    }
    if let Some(bytes) = request(client, base, "get", &query)? {
        let record: Record = serde_json::from_slice(&bytes).map_err(|_| {
            FetchError::from("The lyrics service sent an unreadable response. Try again later.")
        })?;
        if let Some(lyrics) = record.into_lyrics() {
            return Ok(Some(lyrics));
        }
    }
    // Album/featured-credit spelling often differs. Search once, but require
    // matching title, primary artist and duration; never take the first hit.
    std::thread::sleep(Duration::from_millis(350));
    let query = vec![
        ("track_name", title_without_features(&key.title).to_owned()),
        ("artist_name", key.artist.clone()),
    ];
    let Some(bytes) = request(client, base, "search", &query)? else {
        return Ok(None);
    };
    let records: Vec<Record> = serde_json::from_slice(&bytes).map_err(|_| {
        FetchError::from("The lyrics service sent an unreadable response. Try again later.")
    })?;
    Ok(best_match(records, key).and_then(Record::into_lyrics))
}

fn request(
    client: &Client,
    base: &str,
    endpoint: &str,
    query: &[(&str, String)],
) -> Result<Option<Vec<u8>>, FetchError> {
    let response = client
        .get(format!("{base}/api/{endpoint}"))
        .query(query)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            "Lrclib-Client",
            concat!("Aurafy/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .map_err(|_| {
            FetchError::from("Couldn't load lyrics. Check your connection and try again.")
        })?;
    let status = response.status();
    if status.as_u16() == 404 {
        return Ok(None);
    }
    if matches!(status.as_u16(), 429 | 503) {
        let seconds = response
            .headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60)
            .min(86400);
        return Err(FetchError {
            message: "The lyrics service is busy. Please try again shortly.".into(),
            retry_after: Some(
                Duration::from_secs(seconds)
                    + Duration::from_millis(rand::random::<u8>() as u64 + 250),
            ),
        });
    }
    if !status.is_success() {
        return Err("Lyrics are temporarily unavailable. Please try again later.".into());
    }
    let mut bytes = Vec::new();
    response
        .take(1_000_001)
        .read_to_end(&mut bytes)
        .map_err(|_| FetchError::from("Couldn't finish loading the lyrics. Please try again."))?;
    if bytes.len() > 1_000_000 {
        return Err("This lyrics response is too large to display.".into());
    }
    // LRCLIB's edge may return a non-JSON error page even for a successful
    // status. Validate the body instead of trusting an intermediary's header.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }
    let first = bytes
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace());
    if !matches!(first, Some(b'{') | Some(b'[')) {
        return Err("LRCLIB returned an unexpected response. Lyrics may still be available; try again later.".into());
    }
    Ok(Some(bytes))
}

fn canonical(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn title_without_features(title: &str) -> &str {
    let lower = title.to_ascii_lowercase();
    [" (feat.", " (ft.", " (featuring ", " [feat.", " [ft."]
        .iter()
        .filter_map(|marker| lower.find(marker))
        .min()
        .map(|i| title[..i].trim())
        .unwrap_or(title)
}

fn best_match(records: Vec<Record>, key: &TrackKey) -> Option<Record> {
    records
        .into_iter()
        .filter(|r| {
            canonical(title_without_features(&r.track_name))
                == canonical(title_without_features(&key.title))
                && (canonical(&r.artist_name) == canonical(&key.artist)
                    || r.artist_name
                        .split([',', ';'])
                        .any(|a| canonical(a) == canonical(&key.artist)))
                && (key.seconds == 0 || (r.duration - key.seconds as f64).abs() <= 2.0)
                && (r.instrumental
                    || r.plain_lyrics
                        .as_ref()
                        .is_some_and(|s| !s.trim().is_empty())
                    || r.synced_lyrics
                        .as_ref()
                        .is_some_and(|s| !s.trim().is_empty()))
        })
        .max_by_key(|r| {
            (
                canonical(&r.album_name) == canonical(&key.album),
                r.synced_lyrics
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty()),
            )
        })
}

fn timestamp(tag: &str) -> Option<u64> {
    let (minutes, rest) = tag.split_once(':')?;
    let minutes = minutes.parse::<u64>().ok().filter(|m| *m <= 1440)?;
    let (seconds, fraction) = rest.split_once('.').unwrap_or((rest, ""));
    let seconds = seconds.parse::<u64>().ok().filter(|s| *s < 60)?;
    if !fraction.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let fraction = fraction
        .bytes()
        .take(3)
        .fold(0_u64, |n, b| n * 10 + u64::from(b - b'0'));
    let digits = rest
        .split_once('.')
        .map(|(_, f)| f.len().min(3))
        .unwrap_or(0);
    Some((minutes * 60 + seconds) * 1000 + fraction * 10_u64.pow((3 - digits) as u32))
}

fn parse_lrc(value: &str) -> Vec<Line> {
    let offset = value
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("[offset:")?
                .strip_suffix(']')?
                .trim()
                .parse::<i64>()
                .ok()
        })
        .unwrap_or(0)
        .clamp(-86_400_000, 86_400_000);
    let mut lines = Vec::new();
    for raw in value.lines() {
        let mut rest = raw.trim();
        let mut times = Vec::new();
        while let Some(tagged) = rest.strip_prefix('[') {
            let Some((tag, text)) = tagged.split_once(']') else {
                break;
            };
            let Some(time) = timestamp(tag) else { break };
            times.push(time.saturating_add_signed(-offset));
            rest = text;
        }
        for time_ms in times {
            lines.push(Line {
                time_ms,
                text: rest.trim().to_owned(),
            });
        }
    }
    lines.sort_by_key(|line| line.time_ms);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener, thread};

    fn server(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (String, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let mut requests = Vec::new();
            for (headers, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                while !bytes.ends_with(b"\r\n\r\n") {
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                }
                requests.push(String::from_utf8(bytes).unwrap());
                write!(stream, "HTTP/1.1 {headers}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (base, worker)
    }

    #[test]
    fn http_fallback_uses_metadata_without_spotify_credentials() {
        let (base, worker) = server(vec![
            ("404 Not Found", "{}"),
            (
                "200 OK",
                r#"[{"trackName":"Song","artistName":"Example Artist","duration":180,"syncedLyrics":"[00:01.00]An original test line"}]"#,
            ),
        ]);
        let lyrics = fetch_from(&Client::new(), &base, &key("Song (feat. Guest)"))
            .unwrap()
            .unwrap();
        assert_eq!(lyrics.lines[0].time_ms, 1000);
        let requests = worker.join().unwrap();
        assert!(requests[0].starts_with("GET /api/get?"));
        assert!(requests[1].starts_with("GET /api/search?"));
        assert!(requests[1].contains("artist_name=Example+Artist"));
        assert!(
            !requests
                .iter()
                .any(|request| request.to_ascii_lowercase().contains("authorization:"))
        );
    }

    #[test]
    fn server_backoff_stops_the_search_fallback_and_plain_instrumental_records_work() {
        let (base, worker) = server(vec![("503 Service Unavailable\r\nRetry-After: 10", "{}")]);
        let error = fetch_from(&Client::new(), &base, &key("Song")).unwrap_err();
        assert!(error.retry_after.unwrap() >= Duration::from_secs(10));
        assert_eq!(worker.join().unwrap().len(), 1);
        let record: Record =
            serde_json::from_str(r#"{"plainLyrics":"An original test line"}"#).unwrap();
        assert_eq!(record.into_lyrics().unwrap().plain, "An original test line");
        let record: Record = serde_json::from_str(r#"{"instrumental":true}"#).unwrap();
        assert!(record.into_lyrics().unwrap().instrumental);
    }

    #[test]
    #[ignore = "Contacts the real LRCLIB service; prints counts only"]
    fn live_lyrics_smoke() {
        let lyrics = fetch(&TrackKey {
            title: "Good Morning".into(),
            artist: "Kanye West".into(),
            album: "Graduation".into(),
            seconds: 195,
        })
        .unwrap()
        .expect("Expected a lyrics record");
        assert!(!lyrics.lines.is_empty() || !lyrics.plain.is_empty());
        println!(
            "LRCLIB smoke: {} timed lines, {} plain characters",
            lyrics.lines.len(),
            lyrics.plain.len()
        );
    }

    #[test]
    fn lrc_handles_metadata_repeated_tags_fractions_and_instrumental_gaps() {
        let lines = parse_lrc(
            "[ar:Example]\n[offset:100]\n[00:01.50][00:03.005]First\n[00:02.1234]Second\n[00:04.00]\n[00:99.00]Invalid\n[no:time]Ignored",
        );
        assert_eq!(
            lines.iter().map(|l| l.time_ms).collect::<Vec<_>>(),
            [1400, 2023, 2905, 3900]
        );
        assert_eq!(lines[2].text, "First");
        assert!(lines[3].text.is_empty());
        let lyrics = Lyrics {
            lines,
            ..Default::default()
        };
        assert_eq!(lyrics.active_line(0), None);
        assert_eq!(lyrics.active_line(2023), Some(1));
        assert_eq!(lyrics.active_line(3900), Some(3));
        assert_eq!(lyrics.active_line(1500), Some(0)); // Seek backwards.
    }

    fn key(title: &str) -> TrackKey {
        TrackKey {
            title: title.into(),
            artist: "Example Artist".into(),
            album: "Album".into(),
            seconds: 180,
        }
    }

    #[test]
    fn fallback_rejects_wrong_artist_duration_and_remix() {
        let record = |title: &str, artist: &str, duration: f64| Record {
            track_name: title.into(),
            artist_name: artist.into(),
            duration,
            plain_lyrics: Some("Test lyric".into()),
            ..Default::default()
        };
        let selected = best_match(
            vec![
                record("Song", "Other Artist", 180.0),
                record("Song - Remix", "Example Artist", 180.0),
                record("Song", "Example Artist", 230.0),
                record("Song", "Example Artist", 181.0),
            ],
            &key("Song (feat. Guest)"),
        );
        assert_eq!(selected.unwrap().duration, 181.0);
    }

    #[test]
    fn cache_deduplicates_requests_and_keeps_failures_until_explicit_retry() {
        let mut library = Library::default();
        assert!(library.begin(&key("One")));
        assert!(!library.begin(&key("Two")));
        library.finish(
            key("One"),
            Err(FetchError {
                message: "Busy".into(),
                retry_after: Some(Duration::from_secs(30)),
            }),
        );
        assert!(!library.begin(&key("Two")));
        library.retry(&key("One"));
        assert!(library.get(&key("One")).is_some());
        library.cooldown = None;
        assert!(!library.begin(&key("One")));
        library.retry(&key("One"));
        assert!(library.begin(&key("One")));
        for i in 0..CACHE_SIZE + 2 {
            library.finish(key(&i.to_string()), Ok(None));
        }
        assert_eq!(library.entries.len(), CACHE_SIZE);
        assert!(library.get(&key("0")).is_none());
    }
}
