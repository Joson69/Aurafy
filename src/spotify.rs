use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{Rng, distributions::Alphanumeric};
use reqwest::blocking::{Client, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use url::Url;
pub mod catalog;
pub mod insights;
pub mod playback;

pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
pub const FALLBACK_REDIRECT_URI: &str = "http://127.0.0.1:43821/callback";
const API_BASE: &str = "https://api.spotify.com/v1";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const HTTP_TIMEOUT: Duration = Duration::from_secs(18);

#[derive(Clone, Debug)]
pub struct TokenState {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Instant,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Track {
    pub context_position: Option<usize>,
    pub id: String,
    pub name: String,
    pub artist: String,
    pub artist_names: Vec<String>,
    pub artist_ids: Vec<String>,
    pub album: String,
    pub album_id: String,
    pub uri: String,
    pub duration_ms: u64,
    pub artwork_url: Option<String>,
    pub explicit: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: String,
    pub image_url: Option<String>,
    pub owner: String,
    pub owner_id: String,
    pub total_items: usize,
    pub items_accessible: bool,
}

impl Playlist {
    pub fn is_owned_by(&self, user_id: &str) -> bool {
        !user_id.is_empty() && !self.owner_id.is_empty() && self.owner_id == user_id
    }
}

#[derive(Clone, Debug, Default)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub is_active: bool,
    pub volume_percent: Option<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct PlaybackState {
    pub context_uri: Option<String>,
    pub track: Option<Track>,
    pub is_playing: bool,
    pub is_idle: bool,
    pub progress_ms: u64,
    pub device_name: Option<String>,
    pub device_id: Option<String>,
    pub volume_percent: Option<u8>,
    pub shuffle_state: bool,
    pub repeat_state: String,
    pub is_ad: bool,
}

#[derive(Clone, Debug, Default)]
pub struct UserProfile {
    pub id: String,
    pub display_name: String,
    pub image_url: Option<String>,
    pub profile_url: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct SearchResponse {
    tracks: Option<TrackPage>,
}

#[derive(Deserialize)]
struct TrackPage {
    #[serde(default)]
    items: Vec<ApiTrack>,
    #[serde(default)]
    next: Option<String>,
    #[serde(default)]
    offset: usize,
}

#[derive(Deserialize)]
struct ApiTrack {
    #[serde(default, deserialize_with = "nullable_string")]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    uri: String,
    #[serde(default)]
    duration_ms: u64,
    #[serde(default)]
    explicit: bool,
    #[serde(default)]
    artists: Vec<ApiArtist>,
    album: Option<ApiAlbum>,
    is_playable: Option<bool>,
    linked_from: Option<LinkedTrack>,
}

#[derive(Deserialize)]
struct LinkedTrack {
    #[serde(default)]
    uri: String,
}

#[derive(Deserialize)]
struct ApiArtist {
    #[serde(default, deserialize_with = "nullable_string")]
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Default)]
struct ApiAlbum {
    #[serde(default, deserialize_with = "nullable_string")]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    images: Vec<ApiImage>,
}

#[derive(Deserialize)]
struct ApiImage {
    url: String,
}

#[derive(Deserialize)]
struct PlaybackResponse {
    context: Option<PlaybackContext>,
    #[serde(default)]
    is_playing: bool,
    progress_ms: Option<u64>,
    item: Option<ApiTrack>,
    device: Option<ApiDevice>,
    shuffle_state: Option<bool>,
    repeat_state: Option<String>,
    currently_playing_type: Option<String>,
}

#[derive(Deserialize)]
struct PlaybackContext {
    uri: String,
}

#[derive(Deserialize)]
struct ApiDevice {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(rename = "type", default)]
    device_type: String,
    #[serde(default)]
    is_active: bool,
    volume_percent: Option<u8>,
}

#[derive(Deserialize)]
struct DevicesResponse {
    #[serde(default)]
    devices: Vec<ApiDevice>,
}

#[derive(Deserialize)]
struct QueueResponse {
    currently_playing: Option<ApiTrack>,
    #[serde(default)]
    queue: Vec<ApiTrack>,
}

#[derive(Deserialize)]
struct SavedTrackPage {
    #[serde(default)]
    items: Vec<SavedTrackItem>,
}

#[derive(Deserialize)]
struct SavedTrackItem {
    track: Option<ApiTrack>,
}

#[derive(Deserialize)]
struct PlaylistPage {
    #[serde(default)]
    items: Vec<ApiPlaylist>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct ApiPlaylist {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default, deserialize_with = "nullable_string")]
    description: String,
    #[serde(default)]
    images: Vec<ApiImage>,
    owner: Option<ApiOwner>,
    items: Option<ApiPlaylistItems>,
    tracks: Option<ApiPlaylistItems>,
}

#[derive(Deserialize)]
struct ApiOwner {
    #[serde(default)]
    id: String,
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct ApiPlaylistItems {
    total: Option<usize>,
}

#[derive(Deserialize)]
struct RecentlyPlayedResponse {
    #[serde(default)]
    items: Vec<RecentlyPlayedItem>,
}

#[derive(Deserialize)]
struct RecentlyPlayedItem {
    track: Option<ApiTrack>,
}

#[derive(Deserialize)]
struct PlaylistItemsPage {
    #[serde(default)]
    items: Vec<ApiPlaylistItem>,
    next: Option<String>,
}

#[derive(Deserialize)]
struct ApiPlaylistItem {
    item: Option<ApiTrack>,
}

#[derive(Deserialize)]
struct ApiProfile {
    #[serde(default)]
    id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    images: Vec<ApiImage>,
    external_urls: Option<ApiExternalUrls>,
}

fn nullable_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Deserialize)]
struct ApiExternalUrls {
    spotify: Option<String>,
}

pub(crate) fn http_client() -> Result<Client, String> {
    static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            Client::builder()
                .timeout(HTTP_TIMEOUT)
                .user_agent(concat!("Aurafy/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| format!("Could not create HTTP client: {e}"))
        })
        .clone()
}

fn api_response(builder: RequestBuilder, context: &str) -> Result<Response, String> {
    let (client, request) = builder.build_split();
    let mut request = request.map_err(|e| format!("{context}: {e}"))?;
    // Spotify's front end rejects empty POST/PUT commands without an explicit
    // length, even though the endpoints do not require a request body.
    if matches!(
        request.method().as_str(),
        "POST" | "PUT" | "PATCH" | "DELETE"
    ) && request.body().is_none()
    {
        request.headers_mut().insert(
            reqwest::header::CONTENT_LENGTH,
            reqwest::header::HeaderValue::from_static("0"),
        );
    }
    let response = client
        .execute(request)
        .map_err(|e| format!("{context}: {e}"))?;
    checked_response(response, context)
}

fn checked_response(response: Response, context: &str) -> Result<Response, String> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    let body = response.text().unwrap_or_default();
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("error_description")
                .or_else(|| {
                    value
                        .get("error")
                        .map(|error| error.get("message").unwrap_or(error))
                })
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if body.trim_start().starts_with('<') {
                String::new()
            } else {
                body.trim().chars().take(240).collect()
            }
        });

    let detail = match status.as_u16() {
        401 => "Spotify authorization expired. Reconnect Spotify.".to_string(),
        403 => "Spotify denied this action. Check Premium, app access, and requested permissions."
            .to_string(),
        429 => "Spotify rate limit reached. Wait a moment and try again.".to_string(),
        404 => "Spotify could not find the requested item or active playback device.".to_string(),
        _ if message.is_empty() => format!("HTTP {status}"),
        _ => format!("HTTP {status}: {message}"),
    };
    Err(format!("{context}. {detail}"))
}

pub fn validate_client_id(client_id: &str) -> Result<(), String> {
    let client_id = client_id.trim();
    if client_id.len() != 32 || !client_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Paste the 32-character Client ID from your Spotify Developer Dashboard, then click Save and connect. Use the Client ID, not the Client Secret.".into());
    }
    Ok(())
}

pub fn authorize(
    client_id: &str,
    cancelled: &AtomicBool,
    browser_ready: impl FnOnce(String, Option<String>, bool),
) -> Result<TokenState, String> {
    validate_client_id(client_id)?;

    let (listener, redirect_uri) = bind_callback(&[8888, 43821])?;

    let verifier: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    let state: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    let mut auth_url = Url::parse("https://accounts.spotify.com/authorize")
        .map_err(|e| format!("Invalid Spotify sign-in URL: {e}"))?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", client_id.trim())
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &challenge)
        .append_pair("state", &state)
        .append_pair(
            "scope",
            "user-read-private user-read-playback-state user-read-currently-playing user-modify-playback-state user-library-read user-library-modify playlist-read-private playlist-read-collaborative playlist-modify-private playlist-modify-public user-read-recently-played user-top-read",
        );

    if cancelled.load(Ordering::Relaxed) {
        return Err("Spotify sign-in cancelled.".into());
    }
    // Keep listening even if automatic browser launch fails. The UI exposes this
    // same URL for manual opening, with its original PKCE verifier and state.
    let browser_error = crate::browser::open_url(auth_url.as_str()).err();
    browser_ready(auth_url.into(), browser_error, redirect_uri != REDIRECT_URI);
    let (code, mut stream) =
        wait_for_callback(&listener, &state, cancelled, Duration::from_secs(180))?;

    let result = exchange_code(client_id, &code, &verifier, &redirect_uri);
    if cancelled.load(Ordering::Relaxed) {
        callback_response(
            &mut stream,
            "400 Bad Request",
            "Sign-in cancelled. Return to Aurafy to try again.",
        );
        return Err("Spotify sign-in cancelled.".into());
    }
    match &result {
        Ok(_) => callback_response(
            &mut stream,
            "200 OK",
            "Spotify connected. You can close this tab and return to Aurafy.",
        ),
        Err(_) => callback_response(
            &mut stream,
            "502 Bad Gateway",
            "Spotify could not complete sign-in. Return to Aurafy to see the error and try again.",
        ),
    }
    result
}

fn bind_callback(ports: &[u16]) -> Result<(TcpListener, String), String> {
    // Use registered, explicit loopback ports: the developer dashboard may
    // reject portless URIs even though Spotify documents dynamic-port support.
    for port in ports {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", *port)) {
            let actual_port = listener
                .local_addr()
                .map_err(|e| format!("Could not read sign-in callback port: {e}"))?
                .port();
            return Ok((listener, format!("http://127.0.0.1:{actual_port}/callback")));
        }
    }
    Err(format!(
        "Could not open a local Spotify sign-in callback. Ports {ports:?} are unavailable. Close another Aurafy instance or the app using these ports, then try again."
    ))
}

fn exchange_code(
    client_id: &str,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenState, String> {
    let client = http_client()?;
    let response = api_response(
        client.post(TOKEN_URL).form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id.trim()),
            ("code_verifier", verifier),
        ]),
        "Spotify sign-in failed",
    )?;
    let token: TokenResponse = response
        .json()
        .map_err(|e| format!("Could not read Spotify token response: {e}"))?;

    Ok(TokenState {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at: Instant::now() + Duration::from_secs(token.expires_in),
    })
}

fn read_callback_request(
    stream: &mut TcpStream,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> Result<String, String> {
    // Windows may inherit the listener's nonblocking mode. Timed blocking reads
    // avoid spinning a CPU core while the browser sends a partial request.
    stream
        .set_nonblocking(false)
        .map_err(|e| format!("Could not prepare sign-in connection: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|e| format!("Could not read sign-in response: {e}"))?;
    // TCP is a byte stream: a single read is not guaranteed to contain the
    // complete HTTP request line and headers. Read until the header terminator
    // (or the small safety cap) before parsing the callback URL.
    let mut request_bytes = Vec::with_capacity(2048);
    let mut buffer = [0_u8; 1024];
    let read_deadline = deadline.min(Instant::now() + Duration::from_secs(2));
    while request_bytes.len() < 8192 && Instant::now() < read_deadline {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Spotify sign-in cancelled.".into());
        }
        let count = match stream.read(&mut buffer) {
            Ok(count) => count,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(e) => return Err(format!("Could not read Spotify callback: {e}")),
        };
        if count == 0 {
            break;
        }
        request_bytes.extend_from_slice(&buffer[..count]);
        if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(String::from_utf8_lossy(&request_bytes).into_owned());
        }
    }
    Err("Incomplete or oversized sign-in callback request.".into())
}

#[derive(Debug, PartialEq, Eq)]
enum Callback {
    Authorized(String),
    Denied(String),
    Ignore,
}

fn parse_callback(request: &str, expected_state: &str) -> Callback {
    let first_line = request.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    if parts.next() != Some("GET") {
        return Callback::Ignore;
    }
    let Some(target) = parts
        .next()
        .filter(|target| target.starts_with("/callback?"))
    else {
        return Callback::Ignore;
    };
    let Ok(callback_url) = Url::parse(&format!("http://127.0.0.1:8888{target}")) else {
        return Callback::Ignore;
    };
    if callback_url.path() != "/callback" || !matches!(parts.next(), Some("HTTP/1.1" | "HTTP/1.0"))
    {
        return Callback::Ignore;
    }

    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut oauth_error: Option<String> = None;
    for (key, value) in callback_url.query_pairs() {
        match key.as_ref() {
            "code" if code.is_none() => code = Some(value.into_owned()),
            "state" if state.is_none() => state = Some(value.into_owned()),
            "error" if oauth_error.is_none() => oauth_error = Some(value.into_owned()),
            "code" | "state" | "error" => return Callback::Ignore,
            _ => {}
        }
    }

    // Ignore unrelated requests, including a previous browser tab's callback.
    // They must not consume this sign-in attempt or report a false success.
    if state.as_deref() != Some(expected_state) {
        return Callback::Ignore;
    }
    match (code, oauth_error) {
        (None, Some(error)) if !error.is_empty() => Callback::Denied(error),
        (Some(code), None) if !code.is_empty() => Callback::Authorized(code),
        _ => Callback::Ignore,
    }
}

fn callback_response(stream: &mut TcpStream, status: &str, body: &str) {
    let page = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
    let _ = stream.write_all(page.as_bytes());
}

fn wait_for_callback(
    listener: &TcpListener,
    expected_state: &str,
    cancelled: &AtomicBool,
    timeout: Duration,
) -> Result<(String, TcpStream), String> {
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Could not prepare sign-in callback: {e}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err("Spotify sign-in cancelled.".into());
        }
        if Instant::now() >= deadline {
            return Err("Spotify sign-in timed out. Click Save and connect to try again. If Spotify showed INVALID_CLIENT or an invalid redirect URI, check your Client ID and register both http://127.0.0.1:8888/callback and http://127.0.0.1:43821/callback in the Developer Dashboard.".into());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let request = match read_callback_request(&mut stream, cancelled, deadline) {
                    Ok(request) => request,
                    Err(_) => {
                        callback_response(
                            &mut stream,
                            "400 Bad Request",
                            "Invalid sign-in request. Return to the current Spotify sign-in tab.",
                        );
                        continue;
                    }
                };
                match parse_callback(&request, expected_state) {
                    Callback::Authorized(code) => return Ok((code, stream)),
                    Callback::Denied(error) => {
                        callback_response(
                            &mut stream,
                            "400 Bad Request",
                            "Spotify sign-in was denied. Return to Aurafy to try again.",
                        );
                        return Err(format!("Spotify sign-in was cancelled or denied: {error}"));
                    }
                    Callback::Ignore => callback_response(
                        &mut stream,
                        "400 Bad Request",
                        "This is not the current Spotify sign-in request. Return to Aurafy and use the current sign-in link.",
                    ),
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50))
            }
            Err(e) => return Err(format!("Sign-in callback failed: {e}")),
        }
    }
}

pub fn refresh_if_needed(mut token: TokenState, client_id: &str) -> Result<TokenState, String> {
    if Instant::now() + Duration::from_secs(45) < token.expires_at {
        return Ok(token);
    }
    let refresh = token
        .refresh_token
        .clone()
        .ok_or_else(|| "Spotify session expired. Connect Spotify again.".to_string())?;
    let client = http_client()?;
    let response = api_response(
        client.post(TOKEN_URL).form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
            ("client_id", client_id.trim()),
        ]),
        "Could not refresh Spotify session",
    )?;
    let refreshed: TokenResponse = response
        .json()
        .map_err(|e| format!("Could not read refreshed Spotify session: {e}"))?;
    token.access_token = refreshed.access_token;
    if refreshed.refresh_token.is_some() {
        token.refresh_token = refreshed.refresh_token;
    }
    token.expires_at = Instant::now() + Duration::from_secs(refreshed.expires_in);
    Ok(token)
}

pub fn restore_session(refresh_token: &str, client_id: &str) -> Result<TokenState, String> {
    let token = TokenState {
        access_token: String::new(),
        refresh_token: Some(refresh_token.to_owned()),
        expires_at: Instant::now(),
    };
    refresh_if_needed(token, client_id)
}

pub fn search_tracks(access_token: &str, query: &str) -> Result<Vec<Track>, String> {
    let client = http_client()?;
    let mut results = Vec::new();
    // Development Mode now caps search limit at 10. Fetch two small pages instead of
    // requesting the old 30-result limit (which Spotify rejects for new apps).
    for offset in [0, 10] {
        let offset_text = offset.to_string();
        let response = api_response(
            client
                .get(format!("{API_BASE}/search"))
                .bearer_auth(access_token)
                .query(&[
                    ("q", query),
                    ("type", "track"),
                    ("limit", "10"),
                    ("offset", offset_text.as_str()),
                ]),
            "Spotify search failed",
        )?;
        let data: SearchResponse = response
            .json()
            .map_err(|e| format!("Could not read Spotify search results: {e}"))?;
        let items = data.tracks.map(|page| page.items).unwrap_or_default();
        let page_len = items.len();
        results.extend(items.into_iter().map(into_track));
        if page_len < 10 {
            break;
        }
    }
    Ok(results)
}

pub fn get_playback_state(access_token: &str) -> Result<Option<PlaybackState>, String> {
    let client = http_client()?;
    let response = client
        .get(format!("{API_BASE}/me/player"))
        .bearer_auth(access_token)
        .send()
        .map_err(|e| format!("Could not read Spotify playback state: {e}"))?;
    if response.status().as_u16() == 204 {
        return Ok(None);
    }
    let response = checked_response(response, "Could not read Spotify playback state")?;
    let data: PlaybackResponse = response
        .json()
        .map_err(|e| format!("Could not parse Spotify playback state: {e}"))?;
    let device = data.device;
    let track = data.item.map(into_track);
    let is_ad = data.currently_playing_type.as_deref() == Some("ad")
        || track
            .as_ref()
            .is_some_and(|t| t.uri.starts_with("spotify:ad:"));
    Ok(Some(PlaybackState {
        context_uri: data.context.map(|context| context.uri),
        is_idle: false,
        track,
        is_playing: data.is_playing,
        progress_ms: data.progress_ms.unwrap_or(0),
        device_name: device.as_ref().map(|item| item.name.clone()),
        device_id: device.as_ref().and_then(|item| item.id.clone()),
        volume_percent: device.and_then(|item| item.volume_percent),
        shuffle_state: data.shuffle_state.unwrap_or(false),
        repeat_state: data.repeat_state.unwrap_or_else(|| "off".into()),
        is_ad,
    }))
}

pub fn get_queue(access_token: &str) -> Result<Vec<Track>, String> {
    let client = http_client()?;
    let response = api_response(
        client
            .get(format!("{API_BASE}/me/player/queue"))
            .bearer_auth(access_token),
        "Could not read Spotify queue",
    )?;
    let data: QueueResponse = response
        .json()
        .map_err(|e| format!("Could not parse Spotify queue: {e}"))?;
    Ok(data.queue.into_iter().map(into_track).collect())
}

pub fn get_devices(access_token: &str) -> Result<Vec<Device>, String> {
    let client = http_client()?;
    let response = api_response(
        client
            .get(format!("{API_BASE}/me/player/devices"))
            .bearer_auth(access_token),
        "Could not load Spotify devices",
    )?;
    let data: DevicesResponse = response
        .json()
        .map_err(|e| format!("Could not parse Spotify devices: {e}"))?;
    Ok(data
        .devices
        .into_iter()
        .filter_map(|device| {
            Some(Device {
                id: device.id?,
                name: device.name,
                device_type: device.device_type,
                is_active: device.is_active,
                volume_percent: device.volume_percent,
            })
        })
        .collect())
}

pub fn transfer_playback(access_token: &str, device_id: &str) -> Result<(), String> {
    if device_id.trim().is_empty() {
        return Err("Choose a Spotify Connect device first.".into());
    }
    let client = http_client()?;
    api_response(
        client
            .put(format!("{API_BASE}/me/player"))
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "device_ids": [device_id] })),
        "Could not transfer Spotify playback",
    )?;
    Ok(())
}

pub fn get_saved_tracks(access_token: &str) -> Result<Vec<Track>, String> {
    let client = http_client()?;
    let response = api_response(
        client
            .get(format!("{API_BASE}/me/tracks"))
            .bearer_auth(access_token)
            .query(&[("limit", "50"), ("offset", "0")]),
        "Could not load your saved tracks",
    )?;
    let data: SavedTrackPage = response
        .json()
        .map_err(|e| format!("Could not parse your saved tracks: {e}"))?;
    Ok(data
        .items
        .into_iter()
        .filter_map(|item| item.track)
        .map(into_track)
        .collect())
}

pub fn get_playlists(access_token: &str) -> Result<Vec<Playlist>, String> {
    get_playlists_from(&http_client()?, API_BASE, access_token)
}

fn get_playlists_from(
    client: &Client,
    base: &str,
    access_token: &str,
) -> Result<Vec<Playlist>, String> {
    let mut playlists = Vec::new();
    let mut offset = 0;
    loop {
        let response = api_response(
            client
                .get(format!("{base}/me/playlists"))
                .bearer_auth(access_token)
                .query(&[("limit", "50"), ("offset", &offset.to_string())]),
            "Could not load your playlists",
        )?;
        let page: PlaylistPage = response
            .json()
            .map_err(|e| format!("Could not parse your playlists: {e}"))?;
        let count = page.items.len();
        playlists.extend(page.items.into_iter().map(into_playlist));
        if count == 0 || page.next.is_none() {
            break;
        }
        offset += count;
        if offset > 100_000 {
            return Err("Spotify's playlist pagination limit was reached.".into());
        }
    }
    Ok(playlists)
}

fn into_playlist(playlist: ApiPlaylist) -> Playlist {
    let owner_id = playlist
        .owner
        .as_ref()
        .map(|owner| owner.id.clone())
        .unwrap_or_default();
    let owner = playlist
        .owner
        .and_then(|owner| owner.display_name)
        .unwrap_or_else(|| owner_id.clone());
    let items = playlist.items.or(playlist.tracks);
    Playlist {
        id: playlist.id,
        name: playlist.name,
        description: playlist.description,
        image_url: playlist.images.first().map(|image| image.url.clone()),
        owner,
        owner_id,
        total_items: items.as_ref().and_then(|items| items.total).unwrap_or(0),
        items_accessible: items.is_some(),
    }
}

pub fn create_playlist(
    access_token: &str,
    name: &str,
    description: &str,
    is_public: bool,
) -> Result<Playlist, String> {
    let client = http_client()?;
    let response = api_response(
        client
            .post(format!("{API_BASE}/me/playlists"))
            .bearer_auth(access_token)
            .json(&serde_json::json!({
                "name": name,
                "description": description,
                "public": is_public,
                "collaborative": false
            })),
        "Could not create Spotify playlist",
    )?;
    let playlist: ApiPlaylist = response
        .json()
        .map_err(|e| format!("Could not parse the created playlist: {e}"))?;
    let mut playlist = into_playlist(playlist);
    playlist.items_accessible = true;
    Ok(playlist)
}

pub fn add_to_playlist(access_token: &str, playlist_id: &str, uri: &str) -> Result<(), String> {
    let client = http_client()?;
    api_response(
        playlist_add_request(&client, API_BASE, access_token, playlist_id, uri)?,
        "Could not add this track to the playlist",
    )?;
    Ok(())
}

pub fn add_playlist_batch(
    access_token: &str,
    playlist_id: &str,
    uris: &[String],
) -> Result<(), String> {
    if uris.is_empty() || uris.len() > 100 {
        return Err("Choose 1 to 100 tracks.".into());
    }
    let client = http_client()?;
    for uri in uris {
        validate_track_uri(uri)?;
    }
    api_response(
        playlist_items_post(&client, API_BASE, access_token, playlist_id)?
            .json(&serde_json::json!({"uris": uris})),
        "Could not save the DJ tracks",
    )?;
    Ok(())
}

fn playlist_add_request(
    client: &Client,
    base: &str,
    access_token: &str,
    playlist_id: &str,
    uri: &str,
) -> Result<RequestBuilder, String> {
    validate_track_uri(uri)?;
    Ok(
        playlist_items_post(client, base, access_token, playlist_id)?
            .json(&serde_json::json!({"uris": [uri]})),
    )
}

fn playlist_items_post(
    client: &Client,
    base: &str,
    access_token: &str,
    playlist_id: &str,
) -> Result<RequestBuilder, String> {
    if playlist_id.is_empty() || !playlist_id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("Select a valid Spotify playlist first.".into());
    }
    Ok(client
        .post(format!("{base}/playlists/{playlist_id}/items"))
        .bearer_auth(access_token))
}

pub fn remove_from_playlist(
    access_token: &str,
    playlist_id: &str,
    uri: &str,
) -> Result<(), String> {
    let client = http_client()?;
    api_response(
        playlist_remove_request(&client, API_BASE, access_token, playlist_id, uri)?,
        "Could not remove this track from the playlist",
    )?;
    Ok(())
}
fn playlist_remove_request(
    client: &Client,
    base: &str,
    access_token: &str,
    playlist_id: &str,
    uri: &str,
) -> Result<RequestBuilder, String> {
    validate_track_uri(uri)?;
    if playlist_id.is_empty() || !playlist_id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("Select a valid Spotify playlist first.".into());
    }
    Ok(client
        .delete(format!("{base}/playlists/{playlist_id}/items"))
        .bearer_auth(access_token)
        .json(&serde_json::json!({"items":[{"uri":uri}]})))
}

// Membership requires complete pagination; never treat the UI's first 150 rows as a full playlist.
pub fn get_playlist_uris(
    access_token: &str,
    playlist_id: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let client = http_client()?;
    let mut uris = std::collections::HashSet::new();
    for page in 0..2000 {
        let response = api_response(
            client
                .get(format!("{API_BASE}/playlists/{playlist_id}/items"))
                .bearer_auth(access_token)
                .query(&[
                    ("limit", "50".to_owned()),
                    ("offset", (page * 50).to_string()),
                    ("fields", "items(item(uri)),next".to_owned()),
                ]),
            "Could not check playlist membership",
        )?;
        let data: serde_json::Value = response
            .json()
            .map_err(|_| "Could not read playlist membership".to_owned())?;
        let items = data["items"]
            .as_array()
            .ok_or("Could not read playlist items")?;
        for entry in items {
            if let Some(uri) = entry["item"]["uri"].as_str() {
                uris.insert(uri.to_owned());
            }
        }
        if items.len() < 50 || data.get("next").is_some_and(|next| next.is_null()) {
            return Ok(uris);
        }
    }
    Err("Playlist membership exceeded the supported limit".into())
}

pub fn get_playlist_items(access_token: &str, playlist_id: &str) -> Result<Vec<Track>, String> {
    let client = http_client()?;
    let mut tracks = Vec::new();
    for offset in [0, 50, 100] {
        let data = playlist_items_page(&client, access_token, playlist_id, offset)?;
        let page_len = data.items.len();
        tracks.extend(playlist_page_tracks(data, offset));
        if page_len < 50 {
            return Ok(tracks);
        }
    }
    Ok(tracks)
}

fn playlist_items_page(
    client: &Client,
    access: &str,
    id: &str,
    offset: usize,
) -> Result<PlaylistItemsPage, String> {
    api_response(
        client
            .get(format!("{API_BASE}/playlists/{id}/items"))
            .bearer_auth(access)
            .query(&[("limit", 50), ("offset", offset)]),
        "Could not load playlist tracks",
    )?
    .json()
    .map_err(|error| format!("Could not parse playlist tracks: {error}"))
}

pub(crate) const PLAYLIST_REMOVAL_LIMIT: usize = 100;

pub(crate) struct PlaylistEditSnapshot {
    pub version: String,
    pub tracks: Vec<Track>,
}

#[derive(Deserialize)]
struct PlaylistVersion {
    owner: ApiOwner,
    snapshot_id: String,
}

fn playlist_edit_version(
    client: &Client,
    access: &str,
    id: &str,
    owner: &str,
) -> Result<String, String> {
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err("Select a valid Spotify playlist first.".into());
    }
    let state: PlaylistVersion = api_response(
        client
            .get(format!("{API_BASE}/playlists/{id}"))
            .bearer_auth(access)
            .query(&[("fields", "owner(id),snapshot_id")]),
        "Could not verify playlist ownership and version",
    )?
    .json()
    .map_err(|error| format!("Could not read playlist version: {error}"))?;
    if owner.is_empty() || state.owner.id != owner {
        return Err("Choose a playlist you own.".into());
    }
    if state.snapshot_id.is_empty() {
        return Err("Spotify did not supply a playlist version.".into());
    }
    Ok(state.snapshot_id)
}

pub(crate) fn load_playlist_edit_snapshot(
    access: &str,
    id: &str,
    owner: &str,
) -> Result<PlaylistEditSnapshot, String> {
    let client = http_client()?;
    let version = playlist_edit_version(&client, access, id, owner)?;
    let mut tracks = Vec::new();
    for offset in (0..100_000).step_by(50) {
        let page = playlist_items_page(&client, access, id, offset)?;
        let done = page.next.is_none();
        tracks.extend(page.items.into_iter().filter_map(|entry| {
            let mut track = entry.item?;
            if let Some(original) = track.linked_from.take() {
                track.uri = original.uri;
            }
            Some(into_track(track))
        }));
        if done {
            if version != playlist_edit_version(&client, access, id, owner)? {
                return Err(
                    "The playlist changed while loading. Reload it to review the latest tracks."
                        .into(),
                );
            }
            return Ok(PlaylistEditSnapshot { version, tracks });
        }
    }
    Err("Could not read the complete playlist.".into())
}

pub(crate) fn remove_playlist_selection(
    access: &str,
    id: &str,
    owner: &str,
    version: &str,
    uris: &[String],
) -> Result<(), String> {
    if uris.is_empty() || uris.len() > PLAYLIST_REMOVAL_LIMIT {
        return Err("Select between 1 and 100 distinct songs.".into());
    }
    for uri in uris {
        validate_track_uri(uri)?;
    }
    let client = http_client()?;
    if version != playlist_edit_version(&client, access, id, owner)? {
        return Err(
            "The playlist changed after you opened the editor. Reload it before removing tracks."
                .into(),
        );
    }
    let items: Vec<_> = uris
        .iter()
        .map(|uri| serde_json::json!({"uri":uri}))
        .collect();
    api_response(
        client
            .delete(format!("{API_BASE}/playlists/{id}/items"))
            .bearer_auth(access)
            .json(&serde_json::json!({"items": items, "snapshot_id": version})),
        "Could not remove the selected tracks",
    )?;
    Ok(())
}

fn playlist_page_tracks(page: PlaylistItemsPage, offset: usize) -> Vec<Track> {
    page.items
        .into_iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let mut track = into_track(item.item?);
            track.context_position = Some(offset + index);
            Some(track)
        })
        .collect()
}

pub fn get_profile(access_token: &str) -> Result<UserProfile, String> {
    let client = http_client()?;
    let response = api_response(
        client
            .get(format!("{API_BASE}/me"))
            .bearer_auth(access_token),
        "Could not load Spotify profile",
    )?;
    let profile: ApiProfile = response
        .json()
        .map_err(|e| format!("Could not parse Spotify profile: {e}"))?;
    Ok(UserProfile {
        id: profile.id,
        display_name: profile.display_name.unwrap_or_default(),
        image_url: profile.images.first().map(|image| image.url.clone()),
        profile_url: profile.external_urls.and_then(|urls| urls.spotify),
    })
}

pub fn get_recently_played(access_token: &str) -> Result<Vec<Track>, String> {
    let client = http_client()?;
    let response = api_response(
        client
            .get(format!("{API_BASE}/me/player/recently-played"))
            .bearer_auth(access_token)
            .query(&[("limit", "20")]),
        "Could not load recently played tracks",
    )?;
    let data: RecentlyPlayedResponse = response
        .json()
        .map_err(|e| format!("Could not parse recently played tracks: {e}"))?;
    Ok(data
        .items
        .into_iter()
        .filter_map(|item| item.track)
        .map(into_track)
        .collect())
}

pub fn play_track(access_token: &str, uri: &str) -> Result<(), String> {
    let client = http_client()?;
    api_response(
        client
            .put(format!("{API_BASE}/me/player/play"))
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "uris": [uri] })),
        "Could not start Spotify playback",
    )?;
    Ok(())
}

pub fn playback_command(access_token: &str, action: &str) -> Result<(), String> {
    let client = http_client()?;
    let (method, endpoint) = match action {
        "pause" => ("PUT", "pause"),
        "resume" => ("PUT", "play"),
        "next" => ("POST", "next"),
        "previous" => ("POST", "previous"),
        _ => return Err("Unknown playback action.".into()),
    };
    let url = format!("{API_BASE}/me/player/{endpoint}");
    let builder = match method {
        "PUT" => client.put(url),
        _ => client.post(url),
    };
    api_response(
        builder.bearer_auth(access_token),
        "Spotify playback command failed",
    )?;
    Ok(())
}

pub fn seek_to(access_token: &str, position_ms: u64) -> Result<(), String> {
    let client = http_client()?;
    api_response(
        client
            .put(format!("{API_BASE}/me/player/seek"))
            .bearer_auth(access_token)
            .query(&[("position_ms", position_ms.min(86_400_000).to_string())]),
        "Spotify could not seek",
    )?;
    Ok(())
}

pub fn set_volume(
    access_token: &str,
    volume_percent: u8,
    device_id: Option<&str>,
) -> Result<(), String> {
    let client = http_client()?;
    let mut request = client
        .put(format!("{API_BASE}/me/player/volume"))
        .bearer_auth(access_token)
        .query(&[("volume_percent", volume_percent.min(100).to_string())]);
    if let Some(id) = device_id {
        request = request.query(&[("device_id", id)]);
    }
    api_response(request, "Spotify could not set volume")?;
    Ok(())
}

pub fn set_shuffle(access_token: &str, enabled: bool) -> Result<(), String> {
    let client = http_client()?;
    api_response(
        client
            .put(format!("{API_BASE}/me/player/shuffle"))
            .bearer_auth(access_token)
            .query(&[("state", enabled.to_string())]),
        "Spotify could not change shuffle",
    )?;
    Ok(())
}

pub fn set_repeat(access_token: &str, state: &str) -> Result<(), String> {
    if !["off", "context", "track"].contains(&state) {
        return Err("Invalid repeat mode.".into());
    }
    let client = http_client()?;
    api_response(
        client
            .put(format!("{API_BASE}/me/player/repeat"))
            .bearer_auth(access_token)
            .query(&[("state", state)]),
        "Spotify could not change repeat mode",
    )?;
    Ok(())
}

pub fn add_to_queue(access_token: &str, uri: &str) -> Result<(), String> {
    let client = http_client()?;
    api_response(
        client
            .post(format!("{API_BASE}/me/player/queue"))
            .bearer_auth(access_token)
            .query(&[("uri", uri)]),
        "Could not add track to Spotify queue",
    )?;
    Ok(())
}

pub fn save_track(access_token: &str, uri: &str) -> Result<(), String> {
    library_change(access_token, reqwest::Method::PUT, uri)
}

pub fn remove_saved_track(access_token: &str, uri: &str) -> Result<(), String> {
    library_change(access_token, reqwest::Method::DELETE, uri)
}

fn library_change(access_token: &str, method: reqwest::Method, uri: &str) -> Result<(), String> {
    let client = http_client()?;
    api_response(
        library_request(&client, API_BASE, method, access_token, uri)?,
        "Could not update your Spotify library",
    )?;
    Ok(())
}

fn library_request(
    client: &Client,
    base: &str,
    method: reqwest::Method,
    access_token: &str,
    uri: &str,
) -> Result<RequestBuilder, String> {
    validate_track_uri(uri)?;
    // /me/library takes a required query string, unlike playlist /items,
    // which accepts a JSON array. A JSON body here produces "Missing ... uris".
    Ok(client
        .request(method, format!("{base}/me/library"))
        .bearer_auth(access_token)
        .query(&[("uris", uri)]))
}

pub fn validate_track_uri(uri: &str) -> Result<(), String> {
    if uri
        .strip_prefix("spotify:track:")
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()))
    {
        Ok(())
    } else {
        Err("This item isn't a Spotify catalogue song, so it can't be saved or added to a playlist.".into())
    }
}

fn into_track(item: ApiTrack) -> Track {
    let album = item.album.unwrap_or_default();
    let artist_ids = item
        .artists
        .iter()
        .map(|artist| artist.id.clone())
        .collect();
    let artist_names: Vec<String> = item.artists.into_iter().map(|artist| artist.name).collect();
    Track {
        context_position: None,
        id: if item.id.is_empty() {
            item.uri.clone()
        } else {
            item.id
        },
        name: item.name,
        artist: artist_names.join(", "),
        artist_names,
        artist_ids,
        album: album.name,
        album_id: album.id,
        uri: item.uri,
        duration_ms: item.duration_ms,
        artwork_url: album.images.first().map(|image| image.url.clone()),
        explicit: item.explicit,
    }
}

#[cfg(target_os = "windows")]
pub fn save_refresh_token(refresh_token: &str) -> Result<(), String> {
    let entry = keyring::Entry::new("Auralis", "spotify-refresh-token")
        .map_err(|e| format!("Could not open Windows Credential Manager: {e}"))?;
    entry
        .set_password(refresh_token)
        .map_err(|e| format!("Could not store Spotify sign-in securely: {e}"))
}

#[cfg(target_os = "windows")]
pub fn load_refresh_token() -> Option<String> {
    keyring::Entry::new("Auralis", "spotify-refresh-token")
        .ok()
        .and_then(|entry| entry.get_password().ok())
}

#[cfg(target_os = "windows")]
pub fn clear_refresh_token() {
    if let Ok(entry) = keyring::Entry::new("Auralis", "spotify-refresh-token") {
        let _ = entry.delete_credential();
    }
}

#[cfg(not(target_os = "windows"))]
pub fn save_refresh_token(_refresh_token: &str) -> Result<(), String> {
    Err("Secure session storage is currently enabled only on Windows.".into())
}

#[cfg(not(target_os = "windows"))]
pub fn load_refresh_token() -> Option<String> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn clear_refresh_token() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn empty_playback_requests_include_zero_content_length() {
        for method in [
            reqwest::Method::POST,
            reqwest::Method::PUT,
            reqwest::Method::DELETE,
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let worker = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_callback_request(
                    &mut stream,
                    &AtomicBool::new(false),
                    Instant::now() + Duration::from_secs(5),
                )
                .unwrap();
                callback_response(&mut stream, "204 No Content", "");
                request
            });
            api_response(
                http_client()
                    .unwrap()
                    .request(method.clone(), format!("http://{address}/playback")),
                "playback test",
            )
            .unwrap();
            let request = worker.join().unwrap().to_lowercase();
            assert!(request.starts_with(&method.as_str().to_lowercase()));
            assert!(
                request.contains("\r\ncontent-length: 0\r\n"),
                "missing explicit empty body length: {request}"
            );
        }
    }

    #[test]
    fn json_request_body_keeps_its_actual_content_length() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_callback_request(
                &mut stream,
                &AtomicBool::new(false),
                Instant::now() + Duration::from_secs(5),
            )
            .unwrap();
            callback_response(&mut stream, "204 No Content", "");
            request
        });
        let body = serde_json::json!({"device_ids":["phone"]});
        api_response(
            http_client()
                .unwrap()
                .put(format!("http://{address}/player"))
                .json(&body),
            "transfer test",
        )
        .unwrap();
        let request = worker.join().unwrap().to_lowercase();
        assert!(request.contains(&format!("content-length: {}", body.to_string().len())));
        assert!(!request.contains("content-length: 0\r\n"));
    }

    #[test]
    fn library_and_playlist_writes_send_their_distinct_required_uri_formats() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                let end = loop {
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                        break end + 4;
                    }
                };
                let headers = String::from_utf8(bytes[..end].to_vec()).unwrap();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                while bytes.len() < end + length {
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                }
                requests.push((
                    headers,
                    String::from_utf8(bytes[end..end + length].to_vec()).unwrap(),
                ));
                callback_response(&mut stream, "200 OK", "");
            }
            requests
        });
        let client = http_client().unwrap();
        let uri = "spotify:track:ExampleTrack123";
        for method in [reqwest::Method::PUT, reqwest::Method::DELETE] {
            api_response(
                library_request(&client, &base, method, "test-only", uri).unwrap(),
                "library test",
            )
            .unwrap();
        }
        api_response(
            playlist_add_request(&client, &base, "test-only", "Editable123", uri).unwrap(),
            "playlist test",
        )
        .unwrap();
        let requests = worker.join().unwrap();
        for (i, method) in ["PUT", "DELETE"].iter().enumerate() {
            let (headers, body) = &requests[i];
            let line = headers.lines().next().unwrap();
            assert!(line.starts_with(method));
            let target = line.split_whitespace().nth(1).unwrap();
            let url = url::Url::parse(&format!("{base}{target}")).unwrap();
            assert_eq!(url.path(), "/me/library");
            assert_eq!(
                url.query_pairs().find(|(key, _)| key == "uris").unwrap().1,
                uri
            );
            assert!(body.is_empty());
            assert!(headers.to_ascii_lowercase().contains("content-length: 0"));
        }
        assert!(
            requests[2]
                .0
                .starts_with("POST /playlists/Editable123/items HTTP/1.1")
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&requests[2].1).unwrap(),
            serde_json::json!({"uris":[uri]})
        );
        assert!(library_request(&client, &base, reqwest::Method::PUT, "test-only", "").is_err());
        assert!(
            playlist_add_request(
                &client,
                &base,
                "test-only",
                "Editable123",
                "spotify:local:artist:album:song"
            )
            .is_err()
        );
    }

    #[test]
    fn ownership_uses_spotify_ids_and_accepts_nullable_owner_metadata() {
        let api: ApiPlaylist=serde_json::from_str(r#"{"id":"Owned123","name":"Mine","description":null,"owner":{"id":"Owner123","display_name":null},"tracks":{"total":470}}"#).unwrap();
        let playlist = into_playlist(api);
        assert!(playlist.is_owned_by("Owner123"));
        assert!(!playlist.is_owned_by("SomeoneElse"));
        assert!(!playlist.is_owned_by(""));
        assert_eq!(playlist.total_items, 470);
        assert_eq!(playlist.owner, "Owner123");
        let profile: ApiProfile = serde_json::from_str(
            r#"{"id":"Owner123","display_name":"Identical Name","images":[]}"#,
        )
        .unwrap();
        let other = Playlist {
            owner_id: "SomeoneElse".into(),
            owner: profile.display_name.clone().unwrap(),
            ..Default::default()
        };
        assert!(!other.is_owned_by(&profile.id));
        let unknown = Playlist {
            owner: "Identical Name".into(),
            ..Default::default()
        };
        assert!(!unknown.is_owned_by(&profile.id));
    }

    #[test]
    fn playlist_pagination_keeps_owned_playlists_beyond_the_first_page() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let worker = thread::spawn(move || {
            for (offset, body) in [
                (
                    "0",
                    r#"{"items":[{"id":"Followed1","name":"Followed","owner":{"id":"Other","display_name":"Same"}}],"next":"https://api.spotify.com/v1/me/playlists?offset=1"}"#,
                ),
                (
                    "1",
                    r#"{"items":[{"id":"Owned2","name":"Created by me","owner":{"id":"Me","display_name":"Same"},"items":{"total":10}}],"next":null}"#,
                ),
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_callback_request(
                    &mut stream,
                    &AtomicBool::new(false),
                    Instant::now() + Duration::from_secs(5),
                )
                .unwrap();
                assert!(request.contains(&format!("offset={offset}")));
                callback_response(&mut stream, "200 OK", body);
            }
        });
        let playlists = get_playlists_from(&http_client().unwrap(), &base, "test-only").unwrap();
        worker.join().unwrap();
        assert_eq!(playlists.len(), 2);
        assert_eq!(
            playlists
                .iter()
                .filter(|p| p.is_owned_by("Me"))
                .map(|p| p.id.as_str())
                .collect::<Vec<_>>(),
            ["Owned2"]
        );
    }

    #[test]
    fn profile_and_local_track_nullable_fields_are_accepted() {
        let profile: ApiProfile =
            serde_json::from_str(r#"{"display_name":null,"images":[]}"#).unwrap();
        assert!(profile.display_name.is_none());
        let page: SavedTrackPage = serde_json::from_str(r#"{"items":[{"track":{"id":null,"name":"Local song","uri":"spotify:local:artist:album:song","artists":[],"album":null}}]}"#).unwrap();
        assert_eq!(page.items.len(), 1);
        let track: ApiTrack = serde_json::from_str(r#"{"id":"credits","name":"Song","artists":[{"name":"Tyler, The Creator"},{"name":"Guest Artist"}],"album":null}"#).unwrap();
        assert_eq!(
            into_track(track).artist_names,
            vec!["Tyler, The Creator", "Guest Artist"]
        );
    }

    fn request(target: &str) -> String {
        format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
    }

    #[test]
    fn busy_callback_port_falls_back_to_available_loopback_port() {
        let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
        let preferred_port = occupied.local_addr().unwrap().port();
        let (listener, redirect) = bind_callback(&[preferred_port, 0]).unwrap();
        let actual_port = listener.local_addr().unwrap().port();
        assert_ne!(actual_port, preferred_port);
        assert_eq!(redirect, format!("http://127.0.0.1:{actual_port}/callback"));
        assert!(listener.local_addr().unwrap().ip().is_loopback());
    }

    #[test]
    fn unavailable_callback_ports_return_actionable_error() {
        let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = occupied.local_addr().unwrap().port();
        let error = bind_callback(&[port]).unwrap_err();
        assert!(error.contains("unavailable"));
        assert!(error.contains(&port.to_string()));
    }

    #[test]
    fn validates_client_id_before_opening_browser() {
        for invalid in [
            "",
            "abc",
            "not-a-spotify-client-id",
            "0123456789abcdef0123456789abcdeg",
        ] {
            assert!(validate_client_id(invalid).is_err());
        }
        assert!(validate_client_id(" 0123456789abcdef0123456789abcdef ").is_ok());
    }

    #[test]
    fn callback_decodes_code_and_checks_state() {
        assert_eq!(
            parse_callback(
                &request("/callback?code=abc%2B123&state=current"),
                "current"
            ),
            Callback::Authorized("abc+123".into())
        );
        for target in [
            "/favicon.ico",
            "/callback?code=abc&state=old",
            "/callback?code=abc",
            "/callback?code=&state=current",
            "/callback?state=current",
            "/callback?code=abc&state=current&state=current",
            "/callback?code=abc&code=def&state=current",
            "/callback?code=abc&error=access_denied&state=current",
        ] {
            assert_eq!(
                parse_callback(&request(target), "current"),
                Callback::Ignore,
                "{target}"
            );
        }
        assert_eq!(
            parse_callback(
                "POST /callback?code=abc&state=current HTTP/1.1\r\n\r\n",
                "current"
            ),
            Callback::Ignore
        );
    }

    #[test]
    fn denial_requires_matching_state() {
        assert_eq!(
            parse_callback(
                &request("/callback?error=access_denied&state=current"),
                "current"
            ),
            Callback::Denied("access_denied".into())
        );
        assert_eq!(
            parse_callback(
                &request("/callback?error=access_denied&state=old"),
                "current"
            ),
            Callback::Ignore
        );
    }

    #[test]
    fn listener_survives_stray_requests_and_reads_fragmented_callback() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (code, mut stream) = wait_for_callback(
                &listener,
                "current",
                &AtomicBool::new(false),
                Duration::from_secs(5),
            )
            .unwrap();
            callback_response(&mut stream, "200 OK", "Complete");
            code
        });

        for target in [
            "/favicon.ico",
            "/callback?code=stale&state=old",
            "/callback?state=current",
        ] {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream.write_all(request(target).as_bytes()).unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 400"));
            assert!(!response.contains("Spotify connected"));
        }

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(b"GET /callback?code=valid&").unwrap();
        thread::sleep(Duration::from_millis(150));
        stream
            .write_all(b"state=current HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert_eq!(worker.join().unwrap(), "valid");
    }

    #[test]
    fn denial_finishes_listener_with_error() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            wait_for_callback(
                &listener,
                "current",
                &AtomicBool::new(false),
                Duration::from_secs(5),
            )
            .unwrap_err()
        });
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
            .write_all(request("/callback?error=access_denied&state=current").as_bytes())
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 400"));
        assert!(worker.join().unwrap().contains("access_denied"));
    }

    #[test]
    fn cancellation_releases_listener_even_during_incomplete_request() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancel = cancelled.clone();
        let worker = thread::spawn(move || {
            wait_for_callback(&listener, "current", &worker_cancel, Duration::from_secs(5))
                .unwrap_err()
        });
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(b"GET /callback?").unwrap();
        cancelled.store(true, Ordering::Relaxed);
        assert!(worker.join().unwrap().contains("cancelled"));
        drop(stream);
        assert!(TcpListener::bind(address).is_ok());
    }

    #[test]
    fn sign_in_timeout_is_bounded() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let error = wait_for_callback(
            &listener,
            "current",
            &AtomicBool::new(false),
            Duration::ZERO,
        )
        .unwrap_err();
        assert!(error.contains("timed out"));
    }

    #[test]
    fn callback_rejects_oversized_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            read_callback_request(
                &mut stream,
                &AtomicBool::new(false),
                Instant::now() + Duration::from_secs(5),
            )
        });
        let mut stream = TcpStream::connect(address).unwrap();
        stream.write_all(&[b'x'; 8192]).unwrap();
        assert!(worker.join().unwrap().is_err());
    }

    #[test]
    fn oauth_errors_include_spotify_description() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = read_callback_request(
                &mut stream,
                &AtomicBool::new(false),
                Instant::now() + Duration::from_secs(5),
            );
            callback_response(
                &mut stream,
                "400 Bad Request",
                r#"{"error":"invalid_grant","error_description":"Code verifier was invalid"}"#,
            );
        });
        let response = http_client()
            .unwrap()
            .get(format!("http://{address}"))
            .send()
            .unwrap();
        let error = checked_response(response, "Spotify sign-in failed").unwrap_err();
        assert!(error.contains("Code verifier was invalid"));
        worker.join().unwrap();
    }
}

#[cfg(test)]
mod context_position_tests {
    use super::*;
    #[test]
    fn unavailable_playlist_rows_and_duplicate_songs_keep_original_positions() {
        let page: PlaylistItemsPage = serde_json::from_value(serde_json::json!({"items":[
            {"item":null},
            {"item":{"id":"same","uri":"spotify:track:same","name":"Repeated","artists":[]}},
            {"item":{"id":"same","uri":"spotify:track:same","name":"Repeated","artists":[]}}
        ]}))
        .unwrap();
        let tracks = playlist_page_tracks(page, 50);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].context_position, Some(51));
        assert_eq!(tracks[1].context_position, Some(52));
        assert_eq!(tracks[0].uri, tracks[1].uri);
    }
}

#[cfg(test)]
mod playlist_removal_tests {
    use super::*;
    #[test]
    fn removal_uses_current_items_endpoint_and_object_body() {
        let request = playlist_remove_request(
            &Client::new(),
            "https://example.test/v1",
            "test",
            "Playlist123",
            "spotify:track:Song123",
        )
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(request.method(), reqwest::Method::DELETE);
        assert_eq!(request.url().path(), "/v1/playlists/Playlist123/items");
        let body: serde_json::Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"items":[{"uri":"spotify:track:Song123"}]})
        );
        assert!(
            playlist_remove_request(
                &Client::new(),
                "https://example.test",
                "test",
                "../invalid",
                "spotify:track:Song123"
            )
            .is_err()
        );
    }
}
