use super::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Range {
    #[default]
    FourWeeks,
    SixMonths,
    Year,
}
impl Range {
    pub fn label(self) -> &'static str {
        match self {
            Self::FourWeeks => "Last 4 weeks",
            Self::SixMonths => "Last 6 months",
            Self::Year => "Last year",
        }
    }
    fn parameter(self) -> &'static str {
        match self {
            Self::FourWeeks => "short_term",
            Self::SixMonths => "medium_term",
            Self::Year => "long_term",
        }
    }
}
#[derive(Clone, Default, Debug)]
pub struct Data {
    pub tracks: Vec<Track>,
    pub artists: Vec<catalog::Artist>,
}
#[derive(Clone, Debug)]
pub enum Error {
    Permission,
    Other(String),
}

#[derive(Default)]
pub struct State {
    pub range: Range,
    pub cache: HashMap<Range, Data>,
    pub loading: bool,
    pub attempted: bool,
    pub permission_needed: bool,
    pub error: Option<String>,
    pub generation: u64,
    pub artists: bool,
    pub expanded: bool,
}
impl State {
    pub fn reset(&mut self) {
        let generation = self.generation.wrapping_add(1);
        let range = self.range;
        *self = Self {
            generation,
            range,
            ..Default::default()
        };
    }
    pub fn finish(&mut self, generation: u64, range: Range, result: Result<Data, Error>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        match result {
            Ok(data) => {
                self.cache.insert(range, data);
                self.permission_needed = false;
                self.error = None;
            }
            Err(Error::Permission) => {
                self.permission_needed = true;
                self.error = None;
            }
            Err(Error::Other(error)) => self.error = Some(error),
        }
        true
    }
}

#[derive(Deserialize)]
struct ArtistRecord {
    id: String,
    name: String,
    #[serde(default)]
    images: Vec<ApiImage>,
}
#[derive(Deserialize)]
struct Page<T> {
    items: Vec<T>,
}
fn page<T: serde::de::DeserializeOwned>(
    client: &Client,
    base: &str,
    token: &str,
    kind: &str,
    range: Range,
) -> Result<Page<T>, Error> {
    let response = client
        .get(format!("{base}/me/top/{kind}"))
        .bearer_auth(token)
        .query(&[("time_range", range.parameter()), ("limit", "20")])
        .send()
        .map_err(|_| Error::Other("Could not connect to Spotify. Try again.".into()))?;
    if response.status() == reqwest::StatusCode::FORBIDDEN {
        let body = response.json::<serde_json::Value>().unwrap_or_default();
        if body["error"]["message"]
            .as_str()
            .is_some_and(|message| message.to_ascii_lowercase().contains("scope"))
        {
            return Err(Error::Permission);
        }
        return Err(Error::Other(
            "Spotify has not allowed this app to read your top music.".into(),
        ));
    }
    checked_response(response, "Could not load your top music")
        .map_err(Error::Other)?
        .json()
        .map_err(|_| Error::Other("Could not read your top music.".into()))
}
fn fetch_from(client: &Client, base: &str, token: &str, range: Range) -> Result<Data, Error> {
    let tracks: Page<ApiTrack> = page(client, base, token, "tracks", range)?;
    let artists: Page<ArtistRecord> = page(client, base, token, "artists", range)?;
    Ok(Data {
        tracks: tracks.items.into_iter().map(into_track).collect(),
        artists: artists
            .items
            .into_iter()
            .map(|artist| catalog::Artist {
                id: artist.id,
                name: artist.name,
                image_url: artist.images.first().map(|image| image.url.clone()),
            })
            .collect(),
    })
}
pub fn fetch(token: &str, range: Range) -> Result<Data, Error> {
    fetch_from(
        &http_client().map_err(Error::Other)?,
        API_BASE,
        token,
        range,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn top_music_uses_official_ranges_and_keeps_spotify_order() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for kind in ["tracks", "artists"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0; 4096];
                let n = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);
                assert!(
                    request.contains(&format!("/me/top/{kind}?time_range=short_term&limit=20"))
                );
                let body = if kind == "tracks" {
                    r#"{"items":[{"id":"first","name":"First","uri":"spotify:track:first","artists":[],"album":null},{"id":"second","name":"Second","uri":"spotify:track:second","artists":[],"album":null}]}"#
                } else {
                    r#"{"items":[{"id":"artist","name":"Artist","images":[{"url":"https://example.test/cover"}]}]}"#
                };
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let data = fetch_from(
            &Client::new(),
            &format!("http://{address}"),
            "test-token",
            Range::FourWeeks,
        )
        .unwrap();
        assert_eq!(data.tracks[0].id, "first");
        assert_eq!(data.tracks[1].id, "second");
        assert_eq!(data.artists[0].id, "artist");
        server.join().unwrap();
        assert_eq!(Range::SixMonths.parameter(), "medium_term");
        assert_eq!(Range::Year.parameter(), "long_term");
    }
    #[test]
    fn insufficient_scope_is_distinct_from_provider_denial() {
        for (message, permission) in [
            ("Insufficient client scope", true),
            ("User not allowed", false),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0; 4096];
                let _ = stream.read(&mut buffer).unwrap();
                let body = serde_json::json!({"error":{"message":message}}).to_string();
                write!(stream,"HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            });
            let result = fetch_from(
                &Client::new(),
                &format!("http://{address}"),
                "test-token",
                Range::FourWeeks,
            );
            assert!(if permission {
                matches!(result, Err(Error::Permission))
            } else {
                matches!(result, Err(Error::Other(_)))
            });
            server.join().unwrap();
        }
    }
    #[test]
    fn account_reset_rejects_old_rankings_and_clears_private_data() {
        let mut state = State::default();
        state.cache.insert(Range::FourWeeks, Data::default());
        let generation = state.generation;
        state.reset();
        assert!(state.cache.is_empty());
        assert!(!state.finish(generation, Range::FourWeeks, Ok(Data::default())));
        assert!(state.finish(state.generation, Range::FourWeeks, Err(Error::Permission)));
        assert!(state.permission_needed);
    }
}
