#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    process::Command,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use crate::Track;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const YT_DLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

#[derive(Clone, Debug)]
pub enum FreeCommand {
    Play(Track),
    Pause,
    Resume,
    Seek(u64),
    SetVolume(f64),
    Stop,
}

#[derive(Clone, Debug)]
pub enum FreeEvent {
    Status(String),
    State {
        uri: String,
        is_playing: bool,
        position_ms: u64,
        duration_ms: u64,
    },
    Ended(String),
    Error(String),
}

struct CachedStream {
    url: String,
    fetched_at: Instant,
}

pub struct FreePlayer {
    cmd_tx: Sender<FreeCommand>,
    evt_rx: Receiver<FreeEvent>,
    pub active: bool,
    pub current_uri: Option<String>,
    pub is_playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub pending_end_uri: Option<String>,
}

impl FreePlayer {
    pub fn start(ctx: eframe::egui::Context) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (evt_tx, evt_rx) = mpsc::channel();
        thread::spawn(move || run_worker(cmd_rx, evt_tx, ctx));
        Self {
            cmd_tx,
            evt_rx,
            active: false,
            current_uri: None,
            is_playing: false,
            position_ms: 0,
            duration_ms: 0,
            pending_end_uri: None,
        }
    }

    pub fn send(&self, cmd: FreeCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn poll(&mut self) -> Vec<FreeEvent> {
        let mut events = Vec::new();
        while let Ok(evt) = self.evt_rx.try_recv() {
            match &evt {
                FreeEvent::State {
                    uri,
                    is_playing,
                    position_ms,
                    duration_ms,
                } => {
                    self.active = true;
                    self.current_uri = Some(uri.clone());
                    self.is_playing = *is_playing;
                    self.position_ms = *position_ms;
                    self.duration_ms = *duration_ms;
                }
                FreeEvent::Ended(uri) => {
                    self.is_playing = false;
                    self.pending_end_uri = Some(uri.clone());
                }
                _ => {}
            }
            events.push(evt);
        }
        events
    }
}

pub fn yt_dlp_path() -> Option<PathBuf> {
    if let Ok(output) = Command::new("where.exe")
        .arg("yt-dlp")
        .creation_flags_win()
        .output()
        && output.status.success()
    {
        let first = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();
        if !first.is_empty() {
            return Some(PathBuf::from(first));
        }
    }
    let local_bin = dirs::data_local_dir()?
        .join("Aurafy")
        .join("bin")
        .join("yt-dlp.exe");
    if local_bin.exists() {
        Some(local_bin)
    } else {
        None
    }
}

pub fn ensure_yt_dlp(
    evt_tx: &Sender<FreeEvent>,
    ctx: &eframe::egui::Context,
) -> Result<PathBuf, String> {
    if let Some(path) = yt_dlp_path() {
        return Ok(path);
    }
    let _ = evt_tx.send(FreeEvent::Status(
        "Downloading yt-dlp audio resolver (one-time setup)…".into(),
    ));
    ctx.request_repaint();

    let dir = dirs::data_local_dir()
        .ok_or_else(|| "Could not find LocalAppData directory".to_owned())?
        .join("Aurafy")
        .join("bin");
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create bin folder: {e}"))?;
    let target = dir.join("yt-dlp.exe");
    let tmp = dir.join("yt-dlp.exe.part");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;
    let mut resp = client
        .get(YT_DLP_URL)
        .send()
        .map_err(|e| format!("Failed to download yt-dlp: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("yt-dlp download returned HTTP {}", resp.status()));
    }
    let mut file = fs::File::create(&tmp).map_err(|e| format!("Could not create file: {e}"))?;
    std::io::copy(&mut resp, &mut file).map_err(|e| format!("Failed writing yt-dlp: {e}"))?;
    drop(file);
    fs::rename(&tmp, &target).map_err(|e| format!("Could not finalize yt-dlp.exe: {e}"))?;
    Ok(target)
}

pub fn resolve_audio_url(yt_dlp: &PathBuf, track: &Track) -> Result<String, String> {
    let artist = track
        .artist_names
        .first()
        .map(|s| s.as_str())
        .unwrap_or_else(|| track.artist.split(',').next().unwrap_or("").trim());
    let query = format!("ytsearch5:{artist} - {} topic", track.name);
    let output = Command::new(yt_dlp)
        .args([
            "--no-playlist",
            "--no-warnings",
            "--dump-json",
            "-f",
            "bestaudio[ext=m4a]/bestaudio",
            &query,
        ])
        .creation_flags_win()
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp search failed: {}", err.trim()));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let target_secs = track.duration_ms as f64 / 1000.0;
    let orig_lower = track.name.to_ascii_lowercase();

    let mut best: Option<(i32, String)> = None;
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let Some(url) = v["url"].as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        let title = v["title"].as_str().unwrap_or("").to_ascii_lowercase();
        let channel = v["channel"].as_str().unwrap_or("").to_ascii_lowercase();
        let duration = v["duration"].as_f64().unwrap_or(0.0);

        let mut score: i32 = 0;
        if channel.ends_with("- topic") || channel.contains("topic") {
            score += 50;
        }
        if title.contains("official audio") || title.contains("audio") {
            score += 25;
        }
        for bad in [
            "live",
            "cover",
            "slowed",
            "reverb",
            "nightcore",
            "8d",
            "karaoke",
            "reaction",
            "sped up",
            "instrumental",
        ] {
            if title.contains(bad) && !orig_lower.contains(bad) {
                score -= 80;
            }
        }
        if target_secs > 0.0 && duration > 0.0 {
            let diff = (duration - target_secs).abs();
            if diff <= 5.0 {
                score += 40;
            } else if diff <= 15.0 {
                score += 15;
            } else if diff > 45.0 {
                score -= 100;
            }
        }
        if best.as_ref().is_none_or(|(b_score, _)| score > *b_score) {
            best = Some((score, url.to_owned()));
        }
    }
    best.map(|(_, url)| url)
        .ok_or_else(|| format!("No playable audio stream found for {}", track.name))
}

trait CmdExtWin {
    fn creation_flags_win(&mut self) -> &mut Self;
}

impl CmdExtWin for Command {
    #[cfg(target_os = "windows")]
    fn creation_flags_win(&mut self) -> &mut Self {
        self.creation_flags(CREATE_NO_WINDOW)
    }
    #[cfg(not(target_os = "windows"))]
    fn creation_flags_win(&mut self) -> &mut Self {
        self
    }
}

#[cfg(target_os = "windows")]
fn run_worker(
    cmd_rx: Receiver<FreeCommand>,
    evt_tx: Sender<FreeEvent>,
    ctx: eframe::egui::Context,
) {
    use windows::{
        Foundation::Uri,
        Media::{
            Core::MediaSource,
            Playback::{MediaPlaybackState, MediaPlayer},
        },
        Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
        core::HSTRING,
    };

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let player = match MediaPlayer::new() {
        Ok(p) => p,
        Err(e) => {
            let _ = evt_tx.send(FreeEvent::Error(format!(
                "Could not initialize Windows MediaPlayer: {e}"
            )));
            return;
        }
    };

    let mut cache: HashMap<String, CachedStream> = HashMap::new();
    let mut current_uri: Option<String> = None;
    let mut current_duration_ms: u64 = 0;
    let mut ended_reported = false;
    let mut user_paused = false;
    let mut last_tick = Instant::now();

    loop {
        loop {
            let cmd = match cmd_rx.try_recv() {
                Ok(cmd) => cmd,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    let _ = player.Pause();
                    return;
                }
            };
            match cmd {
                FreeCommand::Play(track) => {
                    let uri_key = track.uri.clone();
                    let _ = player.Pause();
                    current_uri = None;

                    let cached = cache
                        .get(&uri_key)
                        .filter(|c| c.fetched_at.elapsed() < Duration::from_secs(7200))
                        .map(|c| c.url.clone());

                    let stream_url = if let Some(u) = cached {
                        Ok(u)
                    } else {
                        let _ = evt_tx.send(FreeEvent::Status(format!(
                            "Finding audio for {}…",
                            track.name
                        )));
                        ctx.request_repaint();
                        ensure_yt_dlp(&evt_tx, &ctx).and_then(|bin| resolve_audio_url(&bin, &track))
                    };

                    match stream_url {
                        Ok(url) => {
                            cache.insert(
                                uri_key.clone(),
                                CachedStream {
                                    url: url.clone(),
                                    fetched_at: Instant::now(),
                                },
                            );
                            if let Ok(win_uri) = Uri::CreateUri(&HSTRING::from(&url))
                                && let Ok(source) = MediaSource::CreateFromUri(&win_uri)
                            {
                                if let Err(err) =
                                    player.SetSource(&source).and_then(|_| player.Play())
                                {
                                    let _ = evt_tx.send(FreeEvent::Error(format!(
                                        "Could not start resolved audio: {err}"
                                    )));
                                    ctx.request_repaint();
                                    continue;
                                }
                                current_uri = Some(uri_key.clone());
                                current_duration_ms = track.duration_ms.max(1);
                                ended_reported = false;
                                user_paused = false;
                                let _ = evt_tx.send(FreeEvent::State {
                                    uri: uri_key,
                                    is_playing: true,
                                    position_ms: 0,
                                    duration_ms: current_duration_ms,
                                });
                                ctx.request_repaint();
                            } else {
                                let _ = evt_tx.send(FreeEvent::Error(
                                    "Could not load resolved audio stream.".into(),
                                ));
                            }
                        }
                        Err(err) => {
                            let _ = evt_tx.send(FreeEvent::Error(err));
                            ctx.request_repaint();
                        }
                    }
                }
                FreeCommand::Pause => {
                    user_paused = true;
                    let _ = player.Pause();
                }
                FreeCommand::Resume => {
                    user_paused = false;
                    ended_reported = false;
                    let _ = player.Play();
                }
                FreeCommand::Seek(ms) => {
                    ended_reported = false;
                    if let Ok(session) = player.PlaybackSession() {
                        let ticks = (ms as i64) * 10_000;
                        let _ =
                            session.SetPosition(windows::Foundation::TimeSpan { Duration: ticks });
                    }
                }
                FreeCommand::SetVolume(vol) => {
                    let _ = player.SetVolume(vol.clamp(0.0, 1.0));
                }
                FreeCommand::Stop => {
                    let _ = player.Pause();
                    current_uri = None;
                }
            }
        }

        if let Some(uri) = &current_uri
            && last_tick.elapsed() >= Duration::from_millis(250)
        {
            last_tick = Instant::now();
            if let Ok(session) = player.PlaybackSession() {
                let state = session.PlaybackState().unwrap_or(MediaPlaybackState::None);
                let pos_ms = session
                    .Position()
                    .map(|ts| (ts.Duration.max(0) / 10_000) as u64)
                    .unwrap_or(0);
                let dur_ms = session
                    .NaturalDuration()
                    .map(|ts| (ts.Duration.max(0) / 10_000) as u64)
                    .ok()
                    .filter(|duration| *duration > 0)
                    .unwrap_or(current_duration_ms);
                let is_playing = state == MediaPlaybackState::Playing
                    || state == MediaPlaybackState::Buffering
                    || state == MediaPlaybackState::Opening;

                if !ended_reported
                    && !user_paused
                    && dur_ms > 1500
                    && state == MediaPlaybackState::Paused
                    && pos_ms.saturating_add(800) >= dur_ms
                {
                    ended_reported = true;
                    let _ = evt_tx.send(FreeEvent::Ended(uri.clone()));
                    ctx.request_repaint();
                } else {
                    let _ = evt_tx.send(FreeEvent::State {
                        uri: uri.clone(),
                        is_playing,
                        position_ms: pos_ms,
                        duration_ms: dur_ms,
                    });
                    ctx.request_repaint();
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(target_os = "windows"))]
fn run_worker(
    cmd_rx: Receiver<FreeCommand>,
    evt_tx: Sender<FreeEvent>,
    _ctx: eframe::egui::Context,
) {
    while let Ok(cmd) = cmd_rx.recv() {
        if let FreeCommand::Play(_) = cmd {
            let _ = evt_tx.send(FreeEvent::Error(
                "Free built-in player is currently Windows-only.".into(),
            ));
        }
    }
}
