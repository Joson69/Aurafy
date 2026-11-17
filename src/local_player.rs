//! Spotify's official native Connect receiver, controlled over a loopback-only
//! WebSocket. No browser engine or polling subprocess is needed for playback.
use crate::spotify::{self, PlaybackState, Track};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket};

pub const DEVICE_NAME: &str = "Aurafy";
pub const KEY_PAGE: &str = "https://developer.spotify.com/dashboard/soloist";
const DOWNLOAD: &str = "https://soloist-builds.spotifycdn.com/soloist_release_windows_x86_64.zip";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Status {
    pub running: bool,
    pub connecting: bool,
    pub paired: bool,
    pub active: bool,
    pub message: String,
}

pub enum Event {
    Status(Status),
    Playback(PlaybackState),
    Queue(Vec<Track>),
    CommandFinished {
        action: String,
        result: Result<(), String>,
    },
}

pub struct Player {
    tx: Sender<Job>,
    stop: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

#[derive(Clone, Default)]
pub struct Options {
    pub initial_volume: u8,
    pub cache_mb: u32,
    pub audio: crate::audio_prefs::Settings,
    pub device_name: String,
}

impl Options {
    pub fn effective_device_name(&self) -> &str {
        let trimmed = self.device_name.trim();
        if trimmed.is_empty() {
            DEVICE_NAME
        } else {
            trimmed
        }
    }
}

pub(crate) struct Job {
    pub(crate) action: String,
    pub(crate) requests: Vec<Value>,
}

impl Player {
    #[cfg(test)]
    pub(crate) fn test_channel() -> (Self, Receiver<Job>) {
        let (tx, rx) = mpsc::channel();
        (
            Self {
                tx,
                stop: Arc::new(AtomicBool::new(false)),
                child: Arc::new(Mutex::new(None)),
            },
            rx,
        )
    }

    pub fn start(
        key: String,
        reset_pairing: bool,
        options: Options,
        notify: impl Fn(Event) + Send + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let worker_stop = stop.clone();
        let worker_child = child.clone();
        thread::spawn(move || {
            if let Err(error) = run(
                key,
                reset_pairing,
                options,
                rx,
                &worker_stop,
                &worker_child,
                &notify,
            ) {
                if !worker_stop.load(Ordering::Relaxed) {
                    notify(Event::Status(Status {
                        message: error,
                        ..Default::default()
                    }));
                }
            }
            stop_child(&worker_child);
        });
        Self { tx, stop, child }
    }

    pub fn command(&self, action: &str, requests: Vec<Value>) -> Result<(), String> {
        self.tx
            .send(Job {
                action: action.into(),
                requests,
            })
            .map_err(|_| "Local playback stopped. Restart it in Settings → Playback.".into())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        stop_child(&self.child);
    }
}

fn stop_child(child: &Mutex<Option<Child>>) {
    if let Ok(mut slot) = child.lock()
        && let Some(mut process) = slot.take()
    {
        let _ = process.kill();
        let _ = process.wait();
    }
}

pub fn request(command: &str) -> Value {
    json!({ "type": "command", "command": command })
}

pub fn directory() -> PathBuf {
    // The Aurafy rename changes the advertised name, never the saved device data.
    // MSIX hosts may redirect AppData writes into their own LocalCache. Keep
    // the native account/session outside AppData so desktop and hosted starts
    // always use the same physical files and Spotify device identity.
    #[cfg(windows)]
    {
        dirs::home_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join(".auralis")
            .join("player")
    }
    #[cfg(not(windows))]
    {
        legacy_directory()
    }
}

fn legacy_directory() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Auralis")
        .join("player")
}

fn hidden_command(path: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW for the headless helper.
    }
    command
}

fn engine_path() -> Result<PathBuf, String> {
    let installed = directory().join("bin/soloist.exe");
    if installed.is_file() {
        return Ok(installed);
    }
    let legacy = legacy_directory().join("bin/soloist.exe");
    if legacy.is_file() {
        return Ok(legacy);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        for candidate in [
            parent.join("soloist/soloist.exe"),
            parent.join("../tools/soloist/soloist.exe"),
        ] {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    install_engine(&installed)?;
    Ok(installed)
}

#[cfg(windows)]
fn install_engine(path: &std::path::Path) -> Result<(), String> {
    let destination = path
        .parent()
        .ok_or("Could not locate the playback engine folder")?;
    fs::create_dir_all(destination)
        .map_err(|e| format!("Could not create playback engine folder: {e}"))?;
    let archive = directory().join("soloist-download.zip");
    let response = spotify::http_client()?.get(DOWNLOAD).send()
        .map_err(|_| "Could not download Spotify's playback engine. Check your internet connection and try again.".to_string())?
        .error_for_status().map_err(|e| format!("Playback engine download failed: {e}"))?;
    let mut bytes = Vec::new();
    response
        .take(64_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Could not read playback engine download: {e}"))?;
    if bytes.len() > 64_000_000 {
        return Err("Spotify's playback engine download exceeded its size limit.".into());
    }
    fs::write(&archive, bytes)
        .map_err(|e| format!("Could not save playback engine download: {e}"))?;
    let result = hidden_command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Expand-Archive -LiteralPath $env:AURALIS_SOLOIST_ARCHIVE -DestinationPath $env:AURALIS_SOLOIST_DEST -Force"])
        .env("AURALIS_SOLOIST_ARCHIVE", &archive).env("AURALIS_SOLOIST_DEST", destination)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
        .map_err(|e| format!("Could not extract playback engine: {e}"));
    let _ = fs::remove_file(archive);
    if !result?.success() || !path.is_file() {
        return Err(
            "Could not install Spotify's playback engine. Try again in Playback settings.".into(),
        );
    }
    Ok(())
}

#[cfg(not(windows))]
fn install_engine(_: &std::path::Path) -> Result<(), String> {
    Err("Local playback is currently supported on Windows.".into())
}

fn run(
    key: String,
    reset_pairing: bool,
    options: Options,
    jobs: Receiver<Job>,
    stop: &AtomicBool,
    child: &Mutex<Option<Child>>,
    notify: &impl Fn(Event),
) -> Result<(), String> {
    let executable = engine_path()?;
    let root = directory();
    let data = root.join("data");
    migrate_session_data(&legacy_directory().join("data"), &data)?;
    fs::create_dir_all(&data)
        .map_err(|e| format!("Could not create local playback data folder: {e}"))?;
    let diagnostics = root.join("startup-status.log");
    if fs::metadata(&diagnostics).is_ok_and(|metadata| metadata.len() > 16384) {
        let _ = fs::write(&diagnostics, b"");
    }
    if stop.load(Ordering::Relaxed) {
        return Ok(());
    }
    let device_name = options.effective_device_name().to_owned();
    crate::player_process::reclaim(&executable, &data)?;
    options.audio.write(&data)?;
    if reset_pairing {
        // Only an explicit reset may clear Soloist's saved connection. Normal
        // startup lets the native receiver restore and manage its own session.
        notify(Event::Status(Status {
            running: true,
            message: format!(
                "Finish pairing: choose {device_name} in Spotify on your phone. Your connection will be saved for future launches."
            ),
            ..Default::default()
        }));
        let mut command = engine_command(&executable, &key, &root, &options);
        command.arg("--pair");
        let process = spawn_engine(command, "pair")
            .map_err(|e| format!("Could not start Spotify pairing: {e}"))?;
        {
            let mut slot = child
                .lock()
                .map_err(|_| "Could not manage pairing process")?;
            *slot = Some(process);
        }
        loop {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            let exit = {
                let mut slot = child
                    .lock()
                    .map_err(|_| "Could not check Spotify pairing")?;
                let Some(process) = slot.as_mut() else {
                    return Ok(());
                };
                process.try_wait().map_err(|e| e.to_string())?
            };
            if let Some(exit) = exit {
                if !exit.success() {
                    return Err(format!(
                        "Spotify pairing did not finish. Restart local playback and select {device_name} in Spotify's device picker."
                    ));
                }
                notify(Event::Status(Status {
                    running: true,
                    connecting: true,
                    message: format!("Pairing saved. Reconnecting {device_name}…"),
                    ..Default::default()
                }));
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    if stop.load(Ordering::Relaxed) {
        return Ok(());
    }
    // Pairing can create a per-user store; update it before the receiver starts.
    options.audio.write(&data)?;
    let mut process = spawn_engine(
        engine_command(&executable, &key, &root, &options),
        "receiver",
    )
    .map_err(|e| format!("Could not start local Spotify playback: {e}"))?;
    drop(key);
    let process_id = process.id();
    {
        let mut slot = child
            .lock()
            .map_err(|_| "Could not manage playback process")?;
        if stop.load(Ordering::Relaxed) {
            let _ = process.kill();
            let _ = process.wait();
            return Ok(());
        }
        *slot = Some(process);
    }
    observe(process_id, &data, device_name, jobs, stop, child, notify)
}

fn migrate_session_data(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    if destination.exists() || !source.is_dir() {
        return Ok(());
    }
    let parent = destination.parent().ok_or("Missing player directory")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not prepare player state: {error}"))?;
    let staging = parent.join(format!("session-migrating-{:016x}", rand::random::<u64>()));
    fs::create_dir(&staging)
        .map_err(|error| format!("Could not prepare session migration: {error}"))?;
    copy_session_tree(source, &staging)
        .map_err(|error| format!("Could not preserve the saved player session: {error}"))?;
    fs::rename(&staging, destination)
        .map_err(|error| format!("Could not finish player session migration: {error}"))?;
    Ok(())
}

fn copy_session_tree(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        if matches!(
            name.to_str(),
            Some("soloist.pid" | "ws.addr" | "ws.port" | ".lock")
        ) {
            continue;
        }
        let target = destination.join(name);
        if kind.is_dir() {
            fs::create_dir(&target)?;
            copy_session_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn engine_command(
    executable: &std::path::Path,
    key: &str,
    root: &std::path::Path,
    options: &Options,
) -> Command {
    // The key is passed directly as a process argument, never via a shell or a
    // log. Its persistent copy is held by Windows Credential Manager.
    let mut command = hidden_command(executable);
    command
        .args([
            "--device-name",
            options.effective_device_name(),
            "--api-key",
        ])
        .arg(key)
        .arg("--data-dir")
        .arg(root.join("data"))
        .arg("--cache-dir")
        .arg(root.join("cache"))
        .arg("--cache-size")
        .arg(options.cache_mb.max(100).to_string())
        .arg("--initial-volume")
        .arg(options.initial_volume.min(100).to_string())
        .args(["--ws", "127.0.0.1:0"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn startup_category(line: &str) -> Option<&'static str> {
    let line = line.to_lowercase();
    [
        ("restoring session", "restoring_session"),
        ("waiting for login", "waiting_for_login"),
        (
            "session creation from stored credentials failed",
            "stored_login_failed",
        ),
        ("login failed", "login_failed"),
        ("logged in as", "logged_in"),
        ("logged out", "logged_out"),
        ("persisting updated client token", "saved_client_token"),
        ("restoring persisted client token", "restoring_client_token"),
        ("pairing complete", "pairing_complete"),
        ("cleared existing session", "pairing_cleared_session"),
        ("shutting down", "shutting_down"),
    ]
    .into_iter()
    .find_map(|(needle, category)| line.contains(needle).then_some(category))
}

fn spawn_engine(mut command: Command, stage: &'static str) -> std::io::Result<Child> {
    let mut process = command.spawn()?;
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory().join("startup-status.log"))
    {
        let _ = writeln!(file, "{stage}: started");
    }
    let outputs: Vec<Box<dyn std::io::Read + Send>> = vec![
        Box::new(process.stdout.take().unwrap()),
        Box::new(process.stderr.take().unwrap()),
    ];
    for output in outputs {
        thread::spawn(move || {
            let mut count = 0;
            for line in std::io::BufReader::new(output)
                .lines()
                .map_while(Result::ok)
            {
                // Only fixed categories are recorded: never SDK log text, account
                // names, keys, tokens, or process arguments.
                if count < 40
                    && let Some(category) = startup_category(&line)
                {
                    if let Ok(mut file) = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(directory().join("startup-status.log"))
                    {
                        let _ = writeln!(file, "{stage}: {category}");
                    }
                    count += 1;
                }
            }
        });
    }
    Ok(process)
}

fn observe(
    process_id: u32,
    data: &std::path::Path,
    device_name: String,
    jobs: Receiver<Job>,
    stop: &AtomicBool,
    child: &Mutex<Option<Child>>,
    notify: &impl Fn(Event),
) -> Result<(), String> {
    let mut state = State {
        device_name,
        ..Default::default()
    };
    let mut first = true;
    while !stop.load(Ordering::Relaxed) {
        let deadline = Instant::now() + Duration::from_secs(if first { 20 } else { 8 });
        let mut socket = loop {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            {
                let mut slot = child
                    .lock()
                    .map_err(|_| "Could not check playback process")?;
                let Some(process) = slot.as_mut() else {
                    return Ok(());
                };
                if let Some(exit) = process.try_wait().map_err(|e| e.to_string())? {
                    return Err(if exit.code() == Some(10) {
                        "Spotify's playback engine expired. Update the engine from the official Soloist downloads page, then restart local playback.".into()
                    } else {
                        format!(
                            "Spotify's playback engine exited (code {}). Check the Soloist API key and try enabling local playback again.",
                            exit.code().unwrap_or(-1)
                        )
                    });
                }
            }
            if let Ok(text) = fs::read_to_string(data.join("soloist.pid"))
                && text.trim().parse::<u32>().ok() == Some(process_id)
            {
                if let Ok(port) = fs::read_to_string(data.join("ws.port"))
                    .and_then(|s| s.trim().parse::<u16>().map_err(std::io::Error::other))
                {
                    if let Ok(socket) = connect(port) {
                        break socket;
                    }
                }
            }
            if Instant::now() >= deadline {
                return Err("Local playback did not start in time. Check your Soloist API key, then retry in Settings → Playback.".into());
            }
            thread::sleep(Duration::from_millis(100));
        };
        first = false;
        state.status.running = true;
        state.status.connecting = true;
        state.auth_started = Instant::now();
        state.status.message = "Restoring the saved player connection…".into();
        notify(Event::Status(state.status.clone()));
        let result = socket_loop(&mut socket, &jobs, stop, &mut state, notify);
        if stop.load(Ordering::Relaxed) {
            let _ = socket.close(None);
            return Ok(());
        }
        if result.is_err() {
            state.status.active = false;
            state.status.message = "Reconnecting to local playback…".into();
            notify(Event::Status(state.status.clone()));
            thread::sleep(Duration::from_millis(250));
        }
    }
    Ok(())
}

fn connect(port: u16) -> Result<WebSocket<TcpStream>, String> {
    if port == 0 {
        return Err("Invalid local playback port".into());
    }
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let stream = TcpStream::connect_timeout(&address, Duration::from_millis(500))
        .map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    tungstenite::client(format!("ws://127.0.0.1:{port}"), stream)
        .map(|(socket, _)| socket)
        .map_err(|e| e.to_string())
}

fn read(socket: &mut WebSocket<TcpStream>) -> Result<Option<Value>, String> {
    match socket.read() {
        Ok(Message::Text(text)) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|_| "Invalid local playback response".into()),
        Ok(Message::Close(_)) => Err("Local playback connection closed".into()),
        Ok(_) => Ok(None),
        Err(tungstenite::Error::Io(e))
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e.to_string()),
    }
}

fn send(socket: &mut WebSocket<TcpStream>, value: &Value) -> Result<(), String> {
    socket
        .send(Message::Text(value.to_string().into()))
        .map_err(|e| e.to_string())
}

fn socket_loop(
    socket: &mut WebSocket<TcpStream>,
    jobs: &Receiver<Job>,
    stop: &AtomicBool,
    state: &mut State,
    notify: &impl Fn(Event),
) -> Result<(), String> {
    // Managed-session verification follows Spotifast's Soloist lifecycle.
    // See THIRD_PARTY_NOTICES.md for the pinned reference and MIT notice.
    let mut last_auth_check = Instant::now() - Duration::from_secs(30);
    let mut last_queue_refresh = Instant::now() - Duration::from_secs(5);
    while !stop.load(Ordering::Relaxed) {
        state.check_auth_timeout(notify);
        let interval = Duration::from_secs(if state.status.paired { 30 } else { 2 });
        if last_auth_check.elapsed() >= interval {
            send(socket, &request("get_auth_state"))?;
            if state.status.paired {
                send(socket, &request("get_state"))?;
            }
            last_auth_check = Instant::now();
        }
        if state.status.paired
            && let Ok(job) = jobs.try_recv()
        {
            let mut result = Ok(());
            for request in job.requests {
                send(socket, &request)?;
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    if stop.load(Ordering::Relaxed) {
                        return Ok(());
                    }
                    if Instant::now() >= deadline {
                        result =
                            Err("Spotify did not confirm the playback command. Try again.".into());
                        break;
                    }
                    if let Some(event) = read(socket)? {
                        match event["type"].as_str() {
                            Some("command_result") if event["command"] == request["command"] => {
                                break;
                            }
                            Some("error") => {
                                result = Err(event["message"]
                                    .as_str()
                                    .unwrap_or("Local playback command failed")
                                    .chars()
                                    .take(240)
                                    .collect());
                                break;
                            }
                            _ => {
                                if state.apply(&event, notify) && state.status.paired {
                                    send(socket, &self::request("get_state"))?;
                                }
                            }
                        }
                    }
                }
                if result.is_err() {
                    break;
                }
            }
            let refresh_queue = matches!(
                job.action.as_str(),
                "play_track"
                    | "resume"
                    | "listen_here"
                    | "select_local"
                    | "next"
                    | "previous"
                    | "queue"
                    | "refresh_context"
            );
            notify(Event::CommandFinished {
                action: job.action,
                result,
            });
            if state.status.paired {
                send(socket, &request("get_state"))?;
                if state.status.active && refresh_queue {
                    state.queue_dirty = true;
                }
            }
        }
        if let Some(event) = read(socket)?
            && state.apply(&event, notify)
            && state.status.paired
        {
            send(socket, &request("get_state"))?;
            if state.status.active {
                state.queue_dirty = true;
            }
        }
        if state.status.active
            && state.queue_dirty
            && last_queue_refresh.elapsed() >= Duration::from_millis(300)
        {
            last_queue_refresh = Instant::now();
            let port = socket
                .get_ref()
                .peer_addr()
                .map_err(|e| e.to_string())?
                .port();
            match full_queue(port, &state.playback.repeat_state) {
                Ok(queue) => {
                    state.queue_dirty = false;
                    notify(Event::Queue(queue));
                }
                Err(_) => {
                    // Preserve the last good list. Retry on the next native change,
                    // rather than polling or replacing it with a ten-item broadcast.
                    state.queue_dirty = false;
                }
            }
        }
    }
    Ok(())
}

fn full_queue(port: u16, repeat: &str) -> Result<Vec<Track>, String> {
    // Soloist broadcasts only ten entries. Query all on a separate socket so
    // broadcast events on the subscription cannot overwrite the full snapshot.
    // This is the same documented RPC mechanism used by Spotifast.
    let mut query = connect(port)?;
    send(
        &mut query,
        &json!({"type":"command","command":"get_queue","limit":0}),
    )?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if let Some(event) = read(&mut query)? {
            match event["type"].as_str() {
                Some("queue_changed") => {
                    return event["upcoming"]
                        .as_array()
                        .map(|entries| queue_tracks(entries, repeat))
                        .ok_or_else(|| "Invalid native queue snapshot".into());
                }
                Some("error") => return Err("Could not read the native queue".into()),
                _ => {}
            }
        }
    }
    Err("The native queue request timed out".into())
}

fn queue_tracks(entries: &[Value], repeat: &str) -> Vec<Track> {
    let mut occurrences = std::collections::HashSet::new();
    entries
        .iter()
        .filter_map(|entry| {
            let track = entity_track(&entry["item"])?;
            // Soloist expands a looping context into multiple future cycles. The
            // same stable occurrence UID identifies those cycles; the song URI
            // does not, because a playlist can intentionally contain a song twice.
            if repeat != "off"
                && entry["source"] == "context"
                && let Some(uid) = entry["uid"]
                    .as_str()
                    .filter(|uid| !uid.is_empty() && *uid != track.uri)
                && !occurrences.insert(uid.to_owned())
            {
                return None;
            }
            Some(track)
        })
        .collect()
}

pub fn repeat_requests(mode: &str) -> Vec<Value> {
    if mode == "track" {
        vec![
            json!({"type":"command","command":"set_repeat_context","enabled":false}),
            json!({"type":"command","command":"set_repeat_track","enabled":true}),
        ]
    } else {
        vec![
            json!({"type":"command","command":"set_repeat_track","enabled":false}),
            json!({"type":"command","command":"set_repeat_context","enabled":mode == "context"}),
        ]
    }
}

struct State {
    device_name: String,
    status: Status,
    playback: PlaybackState,
    progress_anchor: Instant,
    auth_started: Instant,
    queue_dirty: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            device_name: DEVICE_NAME.into(),
            status: Status::default(),
            playback: PlaybackState::default(),
            progress_anchor: Instant::now(),
            auth_started: Instant::now(),
            queue_dirty: true,
        }
    }
}

impl State {
    fn check_auth_timeout(&mut self, notify: &impl Fn(Event)) {
        if self.status.connecting
            && !self.status.paired
            && self.auth_started.elapsed() >= Duration::from_secs(15)
        {
            self.status.connecting = false;
            self.status.message = format!(
                "Player is not signed in. Reconnect the player, or select {} in Spotify to connect this computer.",
                self.device_name
            );
            notify(Event::Status(self.status.clone()));
        }
    }

    fn apply(&mut self, event: &Value, notify: &impl Fn(Event)) -> bool {
        if self.status.active && self.playback.is_playing {
            self.playback.progress_ms = self
                .playback
                .progress_ms
                .saturating_add(self.progress_anchor.elapsed().as_millis() as u64);
        }
        self.progress_anchor = Instant::now();
        let before = self.status.clone();
        let mut playback = false;
        let mut refresh = false;
        match event["type"].as_str().unwrap_or("") {
            "auth_state" => {
                self.status.paired = event["logged_in"].as_bool().unwrap_or(false);
                if self.status.paired {
                    self.status.connecting = false;
                }
                self.status.active = event["is_active"].as_bool().unwrap_or(false);
                refresh = self.status.paired;
            }
            "device_changed" => {
                self.status.active = event["is_active"].as_bool().unwrap_or(false);
                refresh = self.status.active;
            }
            "playback_state" => {
                self.status.paired = true;
                self.status.connecting = false;
                self.status.active = event["is_active"].as_bool().unwrap_or(false);
                self.playback = PlaybackState {
                    context_uri: event["context"]["uri"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned),
                    track: entity_track(&event["item"]),
                    is_playing: event["status"] == "playing",
                    is_idle: event["status"] == "idle",
                    progress_ms: anchored_position(&event["position"]),
                    device_name: Some(self.device_name.clone()),
                    device_id: None,
                    volume_percent: event["volume"].as_u64().map(|n| n.min(100) as u8),
                    shuffle_state: event["options"]["shuffle"].as_bool().unwrap_or(false),
                    repeat_state: event["options"]["repeat"].as_str().unwrap_or("off").into(),
                    is_ad: false,
                };
                playback = true;
            }
            "track_changed" => {
                let track = entity_track(&event["item"]);
                if self.playback.track.as_ref().map(|track| &track.uri)
                    != track.as_ref().map(|track| &track.uri)
                {
                    self.playback.progress_ms = 0;
                }
                self.playback.track = track;
                playback = true;
                self.queue_dirty = true;
            }
            "context_changed" => {
                self.playback.context_uri = event["context"]["uri"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned);
                playback = true;
                refresh = true;
            }
            "playback_changed" => {
                self.playback.is_playing = event["status"] == "playing";
                self.playback.is_idle = event["status"] == "idle";
                playback = true;
                refresh = true;
            }
            "position_sync" => {
                self.playback.progress_ms = anchored_position(&event["position"]);
                playback = true;
            }
            "volume_changed" => {
                self.playback.volume_percent = event["volume"].as_u64().map(|n| n.min(100) as u8);
                playback = true;
            }
            "options_changed" => {
                self.playback.shuffle_state =
                    event["options"]["shuffle"].as_bool().unwrap_or(false);
                self.playback.repeat_state =
                    event["options"]["repeat"].as_str().unwrap_or("off").into();
                playback = true;
                self.queue_dirty = true;
            }
            "queue_changed" => {
                // Notifications are invalidations, not complete queue snapshots.
                self.queue_dirty = self.status.active;
            }
            _ => {}
        }
        self.status.message = if self.status.active {
            "Aurafy is the active playback device"
        } else if self.status.paired {
            "Ready to play on this computer"
        } else if self.status.connecting {
            "Restoring the saved player connection…"
        } else {
            "Local player is signed out. Reconnect the player to retry your saved session."
        }
        .into();
        if before != self.status {
            notify(Event::Status(self.status.clone()));
        }
        if playback {
            notify(Event::Playback(self.playback.clone()));
        }
        refresh
    }
}

fn anchored_position(position: &Value) -> u64 {
    let base = position["position_ms"].as_u64().unwrap_or(0);
    let speed = position["speed"].as_f64().unwrap_or(0.0).clamp(0.0, 4.0);
    let Some(timestamp) = position["timestamp_ms"].as_u64() else {
        return base;
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    base.saturating_add((now.saturating_sub(timestamp).min(86_400_000) as f64 * speed) as u64)
}

fn entity_track(entity: &Value) -> Option<Track> {
    let uri = entity["uri"].as_str()?.to_string();
    if uri.is_empty() {
        return None;
    }
    let decoration = &entity["decorations"];
    Some(Track {
        id: uri.rsplit(':').next().unwrap_or(&uri).into(),
        name: decoration["identity"]["name"]
            .as_str()
            .unwrap_or("Loading track…")
            .into(),
        artist: decoration["creators"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|creator| creator["entity"]["decorations"]["identity"]["name"].as_str())
            .collect::<Vec<_>>()
            .join(", "),
        artist_names: decoration["creators"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|creator| {
                creator["entity"]["decorations"]["identity"]["name"]
                    .as_str()
                    .map(str::to_owned)
            })
            .collect(),
        artist_ids: decoration["creators"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|creator| creator["entity"]["decorations"]["identity"]["name"].is_string())
            .map(|creator| {
                creator["entity"]["uri"]
                    .as_str()
                    .and_then(|uri| uri.strip_prefix("spotify:artist:"))
                    .unwrap_or("")
                    .to_string()
            })
            .collect(),
        album: decoration["parent"]["entity"]["decorations"]["identity"]["name"]
            .as_str()
            .unwrap_or("")
            .into(),
        album_id: decoration["parent"]["entity"]["uri"]
            .as_str()
            .and_then(|uri| uri.strip_prefix("spotify:album:"))
            .unwrap_or("")
            .into(),
        duration_ms: decoration["playback"]["duration_ms"].as_u64().unwrap_or(0),
        artwork_url: decoration["visual_identity"]["cover"]
            .as_array()
            .and_then(|covers| {
                covers
                    .iter()
                    .find_map(|cover| cover["url"].as_str().map(str::to_owned))
            }),
        explicit: false,
        context_position: None,
        uri,
    })
}

#[cfg(windows)]
pub fn save_key(key: &str) -> Result<(), String> {
    keyring::Entry::new("Auralis", "soloist-api-key")
        .and_then(|entry| entry.set_password(key))
        .map_err(|_| "Could not save the playback key in Windows Credential Manager.".into())
}
#[cfg(windows)]
pub fn load_key() -> Result<Option<String>, String> {
    match keyring::Entry::new("Auralis", "soloist-api-key").and_then(|entry| entry.get_password()) {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Could not read the saved local-playback key.".into()),
    }
}

#[cfg(not(windows))]
pub fn save_key(_: &str) -> Result<(), String> {
    Err("Local playback requires Windows.".into())
}
#[cfg(not(windows))]
pub fn load_key() -> Result<Option<String>, String> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_queue_requests_all_entries_and_preserves_repeated_songs() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            send(
                &mut socket,
                &json!({"type":"auth_state","logged_in":true,"is_active":true}),
            )
            .unwrap();
            let Message::Text(text) = socket.read().unwrap() else {
                panic!("expected query")
            };
            let command: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(
                command,
                json!({"type":"command","command":"get_queue","limit":0})
            );
            let entries: Vec<_> = (0..137).map(|i| json!({"item":{"uri":format!("spotify:track:{}", i % 40),"decorations":{"identity":{"name":format!("Song {i}")}}}})).collect();
            send(
                &mut socket,
                &json!({"type":"queue_changed","upcoming":entries}),
            )
            .unwrap();
        });
        let queue = full_queue(port, "off").unwrap();
        assert_eq!(queue.len(), 137);
        assert_eq!(queue[0].uri, queue[40].uri);
        assert_eq!(queue[136].name, "Song 136");
        server.join().unwrap();
    }

    #[test]
    #[ignore = "reads the running native receiver; no playback changes"]
    fn live_full_queue_read_only() {
        let port = fs::read_to_string(directory().join("data/ws.port"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let queue = full_queue(port, "off").unwrap();
        let displayed = full_queue(port, "context").unwrap();
        println!(
            "Native upcoming entries: {}; single-cycle preview: {}",
            queue.len(),
            displayed.len()
        );
        fs::write(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/full-queue-count.json"),
            json!({"native_upcoming":queue.len(),"single_cycle_preview":displayed.len()})
                .to_string(),
        )
        .unwrap();
    }

    #[test]
    fn shortened_queue_broadcasts_only_invalidate_the_full_snapshot() {
        let mut state = State::default();
        state.status.active = true;
        state.queue_dirty = false;
        state.apply(&json!({"type":"queue_changed","upcoming":[]}), &|event| {
            assert!(
                !matches!(event, Event::Queue(_)),
                "a broadcast must not truncate the full list"
            );
        });
        assert!(state.queue_dirty);
        state.status.active = false;
        state.apply(&json!({"type":"queue_changed","upcoming":[]}), &|_| {});
        assert!(
            !state.queue_dirty,
            "do not fetch this receiver's queue after phone handoff"
        );
    }
    #[test]
    fn loop_preview_collapses_occurrences_not_song_uris_or_manual_queue_entries() {
        let entry = |uid: &str, source: &str| json!({"uid":uid,"source":source,"item":{"uri":"spotify:track:same","decorations":{"identity":{"name":uid}}}});
        let entries = vec![
            entry("first-occurrence", "context"),
            entry("second-occurrence", "context"),
            entry("first-occurrence", "context"),
            entry("second-occurrence", "context"),
            entry("manual", "queue"),
            entry("manual", "queue"),
            entry("spotify:track:same", "context"),
            entry("spotify:track:same", "context"),
        ];
        let queue = queue_tracks(&entries, "context");
        assert_eq!(queue.len(), 6);
        assert_eq!(queue[0].name, "first-occurrence");
        assert_eq!(queue[1].name, "second-occurrence");
        assert_eq!(queue_tracks(&entries, "off").len(), 8);
        assert_eq!(queue_tracks(&entries, "track").len(), 6);
        assert!(queue_tracks(&[], "context").is_empty());
    }

    #[test]
    fn repeat_one_disables_context_before_enabling_track_repeat() {
        let track = repeat_requests("track");
        assert_eq!(track[0]["command"], "set_repeat_context");
        assert_eq!(track[0]["enabled"], false);
        assert_eq!(track[1]["command"], "set_repeat_track");
        assert_eq!(track[1]["enabled"], true);
        let context = repeat_requests("context");
        assert_eq!(context[0]["command"], "set_repeat_track");
        assert_eq!(context[0]["enabled"], false);
        assert_eq!(context[1]["enabled"], true);
        assert!(
            repeat_requests("off")
                .iter()
                .all(|value| value["enabled"] == false)
        );
    }

    #[test]
    fn launch_preferences_reach_the_native_engine_arguments() {
        let command = engine_command(
            std::path::Path::new("soloist.exe"),
            "test-key",
            std::path::Path::new("test-state"),
            &Options {
                initial_volume: 63,
                cache_mb: 500,
                audio: crate::audio_prefs::Settings::default(),
                device_name: "Studio PC".into(),
            },
        );
        let args: Vec<_> = command
            .get_args()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        let value = |name: &str| args[args.iter().position(|arg| arg == name).unwrap() + 1].clone();
        assert_eq!(value("--device-name"), "Studio PC");
        assert_eq!(value("--initial-volume"), "63");
        assert_eq!(value("--cache-size"), "500");
        assert!(!args.iter().any(|arg| arg == "--pair"));
    }

    #[test]
    fn restored_login_is_rechecked_before_dispatching_playback() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            send(
                &mut socket,
                &json!({"type":"auth_state","logged_in":false,"is_active":false}),
            )
            .unwrap();
            let mut checks = 0;
            let mut controls = Vec::new();
            while let Ok(Message::Text(text)) = socket.read() {
                let message: Value = serde_json::from_str(&text).unwrap();
                let command = message["command"].as_str().unwrap();
                match command {
                    "get_auth_state" => {
                        checks += 1;
                        send(
                            &mut socket,
                            &json!({"type":"auth_state","logged_in":checks >= 2,"is_active":false}),
                        )
                        .unwrap();
                    }
                    "get_state" => {
                        send(&mut socket, &json!({"type":"playback_state","is_active":!controls.is_empty(),"status":if controls.len() == 2 {"playing"} else {"paused"}})).unwrap();
                    }
                    "activate" | "play" => {
                        assert!(checks >= 2, "controls must wait for verified native login");
                        assert!(
                            message.get("uri").is_none(),
                            "resume must retain the current context"
                        );
                        controls.push(command.to_owned());
                        send(
                            &mut socket,
                            &json!({"type":"command_result","command":command}),
                        )
                        .unwrap();
                    }
                    _ => {}
                }
            }
            controls
        });
        let mut socket = connect(port).unwrap();
        let (tx, rx) = mpsc::channel();
        tx.send(Job {
            action: "resume".into(),
            requests: vec![request("activate"), request("play")],
        })
        .unwrap();
        let stop = AtomicBool::new(false);
        let mut state = State::default();
        state.status.running = true;
        state.status.connecting = true;
        socket_loop(&mut socket, &rx, &stop, &mut state, &|event| {
            if matches!(
                event,
                Event::Playback(PlaybackState {
                    is_playing: true,
                    ..
                })
            ) {
                stop.store(true, Ordering::Relaxed);
            }
        })
        .unwrap();
        assert!(state.status.paired && !state.status.connecting);
        let _ = socket.close(None);
        drop(socket);
        assert_eq!(server.join().unwrap(), vec!["activate", "play"]);
    }

    #[test]
    fn readiness_wait_has_a_bound_and_recovers_without_resetting_pairing() {
        let mut state = State::default();
        state.status.running = true;
        state.status.connecting = true;
        state.apply(&json!({"type":"auth_state","logged_in":false}), &|_| {});
        assert!(state.status.connecting);
        state.auth_started = Instant::now() - Duration::from_secs(16);
        state.check_auth_timeout(&|_| {});
        assert!(!state.status.connecting && !state.status.paired);
        state.apply(&json!({"type":"auth_state","logged_in":true}), &|_| {});
        assert!(state.status.paired && !state.status.connecting);
    }

    #[test]
    fn startup_diagnostics_only_record_fixed_categories() {
        assert_eq!(
            startup_category("logged in as private-account"),
            Some("logged_in")
        );
        assert_eq!(
            startup_category("login failed token=private-value"),
            Some("login_failed")
        );
        assert_eq!(startup_category("api-key private-value"), None);
    }

    #[test]
    fn session_migration_preserves_credentials_but_not_old_runtime_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/session-tests")
            .join(format!("migration-{}", rand::random::<u64>()));
        let source = root.join("old");
        let destination = root.join("shared/data");
        fs::create_dir_all(source.join("cache")).unwrap();
        fs::write(source.join("cache/client_token"), b"synthetic-session").unwrap();
        fs::write(source.join(".device_id"), b"synthetic-device").unwrap();
        fs::write(source.join("soloist.pid"), b"old-runtime").unwrap();
        fs::write(source.join("ws.port"), b"12345").unwrap();
        migrate_session_data(&source, &destination).unwrap();
        assert_eq!(
            fs::read(destination.join("cache/client_token")).unwrap(),
            b"synthetic-session"
        );
        assert_eq!(
            fs::read(destination.join(".device_id")).unwrap(),
            b"synthetic-device"
        );
        assert!(!destination.join("soloist.pid").exists() && !destination.join("ws.port").exists());
        assert!(
            source.join("cache/client_token").exists(),
            "legacy data must remain available"
        );
        fs::write(destination.join("cache/client_token"), b"newer-session").unwrap();
        migrate_session_data(&source, &destination).unwrap();
        assert_eq!(
            fs::read(destination.join("cache/client_token")).unwrap(),
            b"newer-session"
        );
    }

    #[test]
    fn position_uses_the_server_time_anchor() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let position =
            anchored_position(&json!({"position_ms":42000,"timestamp_ms":now-3000,"speed":1.0}));
        assert!((45000..46000).contains(&position));
        assert_eq!(
            anchored_position(&json!({"position_ms":42000,"timestamp_ms":now-3000,"speed":0.0})),
            42000
        );
    }

    #[test]
    fn websocket_commands_and_phone_handoff_round_trip() {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut socket = tungstenite::accept(stream).unwrap();
            send(
                &mut socket,
                &json!({"type":"auth_state","logged_in":true,"is_active":false}),
            )
            .unwrap();
            let mut activations = 0;
            let mut handed_off = false;
            while let Ok(Message::Text(text)) = socket.read() {
                let command: Value = serde_json::from_str(&text).unwrap();
                match command["command"].as_str().unwrap_or("") {
                    "activate" => {
                        activations += 1;
                        send(
                            &mut socket,
                            &json!({"type":"command_result","command":"activate"}),
                        )
                        .unwrap();
                    }
                    "get_state" => {
                        if handed_off {
                            continue;
                        }
                        send(&mut socket,&json!({"type":"playback_state","is_active":true,"status":"playing","position":{"position_ms":42000},"item":{"uri":"spotify:track:abc","decorations":{"identity":{"name":"Song"},"playback":{"duration_ms":180000}}}})).unwrap();
                        send(
                            &mut socket,
                            &json!({"type":"device_changed","is_active":false}),
                        )
                        .unwrap();
                        handed_off = true;
                    }
                    _ => {}
                }
            }
            activations
        });
        let mut socket = connect(port).unwrap();
        let (tx, rx) = mpsc::channel();
        tx.send(Job {
            action: "listen_here".into(),
            requests: vec![request("activate")],
        })
        .unwrap();
        let stop = AtomicBool::new(false);
        let active_seen = AtomicBool::new(false);
        let notify = |event| {
            if let Event::Status(status) = event {
                if status.active {
                    active_seen.store(true, Ordering::Relaxed);
                } else if active_seen.load(Ordering::Relaxed) {
                    stop.store(true, Ordering::Relaxed);
                }
            }
        };
        let mut state = State::default();
        socket_loop(&mut socket, &rx, &stop, &mut state, &notify).unwrap();
        let _ = socket.close(None);
        drop(socket);
        assert!(active_seen.load(Ordering::Relaxed));
        assert!(!state.status.active);
        assert_eq!(
            server.join().unwrap(),
            1,
            "handoff must not auto-activate Aurafy again"
        );
    }
    #[test]
    fn phone_handoff_releases_local_active_state() {
        let mut state = State::default();
        state.apply(
            &json!({"type":"auth_state","logged_in":true,"is_active":true}),
            &|_| {},
        );
        assert!(state.status.active);
        state.apply(
            &json!({"type":"device_changed","is_active":false,"device_name":"Aurafy"}),
            &|_| {},
        );
        assert!(!state.status.active);
        assert!(state.status.paired);
    }
    #[test]
    fn native_snapshot_includes_track_position_and_volume() {
        let mut state = State::default();
        state.apply(&json!({"type":"playback_state","is_active":true,"status":"playing","position":{"position_ms":42000},"volume":31,"item":{"uri":"spotify:track:abc","decorations":{"identity":{"name":"Song"},"playback":{"duration_ms":180000},"creators":[{"entity":{"decorations":{"identity":{"name":"Artist"}}}}]}}}),&|_|{});
        assert!(state.status.active);
        assert!(state.playback.is_playing);
        assert_eq!(state.playback.progress_ms, 42000);
        assert_eq!(state.playback.volume_percent, Some(31));
        let track = state.playback.track.unwrap();
        assert_eq!(track.name, "Song");
        assert_eq!(track.artist, "Artist");
    }

    #[test]
    fn native_metadata_preserves_album_and_individual_artist_navigation_ids() {
        let track = entity_track(&json!({"uri":"spotify:track:track1","decorations":{"creators":[{"entity":{"uri":"spotify:artist:artist1","decorations":{"identity":{"name":"Artist"}}}},{"entity":{"uri":"spotify:artist:missing"}},{"entity":{"uri":"spotify:artist:guest1","decorations":{"identity":{"name":"Guest"}}}}],"parent":{"entity":{"uri":"spotify:album:album1","decorations":{"identity":{"name":"Album"}}}}}})).unwrap();
        assert_eq!(track.artist_names, ["Artist", "Guest"]);
        assert_eq!(track.artist_ids, ["artist1", "guest1"]);
        assert_eq!(track.album_id, "album1");
    }

    #[test]
    fn metadata_refresh_keeps_position_but_a_new_track_resets_it() {
        let mut state = State::default();
        state.apply(&json!({"type":"playback_state","is_active":false,"status":"paused","position":{"position_ms":37000},"item":{"uri":"spotify:track:abc","decorations":{"identity":{"name":"Song"},"playback":{"duration_ms":180000}}}}), &|_| {});
        state.apply(&json!({"type":"track_changed","item":{"uri":"spotify:track:abc","decorations":{"identity":{"name":"Updated title"},"playback":{"duration_ms":180000}}}}), &|_| {});
        assert_eq!(state.playback.progress_ms, 37000);
        assert_eq!(state.playback.track.as_ref().unwrap().name, "Updated title");
        state.apply(
            &json!({"type":"track_changed","item":{"uri":"spotify:track:def"}}),
            &|_| {},
        );
        assert_eq!(state.playback.progress_ms, 0);
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn idle_is_distinct_from_pause_and_same_uri_can_start_again() {
        let mut state = State::default();
        state.apply(&json!({"type":"playback_state","is_active":true,"status":"playing","item":{"uri":"spotify:track:repeat"}}), &|_| {});
        assert!(!state.playback.is_idle);
        state.apply(&json!({"type":"playback_changed","status":"idle"}), &|_| {});
        assert!(state.playback.is_idle && !state.playback.is_playing);
        state.apply(
            &json!({"type":"playback_changed","status":"playing"}),
            &|_| {},
        );
        assert!(state.playback.is_playing && !state.playback.is_idle);
        state.apply(
            &json!({"type":"playback_changed","status":"paused"}),
            &|_| {},
        );
        assert!(!state.playback.is_playing && !state.playback.is_idle);
    }
}

// Only regular clock movement can wait for the UI's passive refresh deadline.
pub fn position_only(old: &PlaybackState, next: &PlaybackState) -> bool {
    old.track.as_ref().map(|t| &t.uri) == next.track.as_ref().map(|t| &t.uri)
        && old.is_playing == next.is_playing
        && old.is_idle == next.is_idle
        && old.context_uri == next.context_uri
        && old.device_id == next.device_id
        && old.device_name == next.device_name
        && old.volume_percent == next.volume_percent
        && old.shuffle_state == next.shuffle_state
        && old.repeat_state == next.repeat_state
        && next.progress_ms >= old.progress_ms
        && next.progress_ms - old.progress_ms <= 2000
}
#[cfg(test)]
mod repaint_tests {
    use super::*;
    #[test]
    fn state_changes_and_seeks_wake_immediately_but_clock_ticks_can_wait() {
        let old = PlaybackState {
            track: Some(Track {
                uri: "spotify:track:test".into(),
                ..Default::default()
            }),
            is_playing: true,
            progress_ms: 10000,
            ..Default::default()
        };
        let mut next = old.clone();
        next.progress_ms += 500;
        assert!(position_only(&old, &next));
        next.is_playing = false;
        assert!(!position_only(&old, &next));
        next = old.clone();
        next.progress_ms = 0;
        assert!(!position_only(&old, &next));
        next = old.clone();
        next.track.as_mut().unwrap().uri = "spotify:track:next".into();
        assert!(!position_only(&old, &next));
        next = old.clone();
        next.volume_percent = Some(60);
        assert!(!position_only(&old, &next));
    }
}
