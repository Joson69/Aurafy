mod actions;
mod events;
mod library;
mod playback;
mod playlist_editor;

pub(crate) use events::{demo_tracks, format_time, paint_cover_rounded};

use crate::*;
pub(crate) use eframe::egui::{
    self, Align, Align2, CentralPanel, Color32, CornerRadius, FontId, Frame, Id, Key, Layout,
    Margin, Order, Pos2, Rect, RichText, ScrollArea, Sense, SidePanel, Stroke, TextEdit,
    TopBottomPanel, Ui, Vec2,
};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use spotify::{Device, PlaybackState, Playlist, TokenState, Track, catalog};
pub(crate) use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

pub(crate) const MAX_ARTWORK_CACHE: usize = 72;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum TrackSortColumn {
    #[default]
    Default,
    Title,
    Album,
    Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrackSort {
    pub(crate) column: TrackSortColumn,
    pub(crate) ascending: bool,
}

impl Default for TrackSort {
    fn default() -> Self {
        Self {
            column: TrackSortColumn::Default,
            ascending: true,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Page {
    #[default]
    Home,
    Search,
    Library,
    Liked,
    Recent,
    Profile,
    Artist,
    Album,
    Lyrics,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SettingsTab {
    #[default]
    General,
    Listening,
    Appearance,
    Privacy,
    Account,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SearchFilter {
    #[default]
    All,
    Songs,
    Artists,
    Albums,
    Playlists,
}

#[derive(Clone, PartialEq)]
pub(crate) enum NavTarget {
    Page(Page),
    Playlist(String),
    Artist { name: String, id: String },
    Album(Track),
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub(crate) struct AppConfig {
    pub(crate) client_id: String,
    pub(crate) recent_searches: Vec<String>,
    pub(crate) compact_rows: bool,
    pub(crate) smooth_progress: bool,
    pub(crate) queue_open: bool,
    pub(crate) local_playback: bool,
    pub(crate) pause_on_start: bool,
    pub(crate) quit_on_close: bool,
    pub(crate) last_page: Page,
    pub(crate) last_query: String,
    pub(crate) last_artist: String,
    pub(crate) last_artist_id: String,
    pub(crate) last_album: Option<Track>,
    pub(crate) window_placement: Option<desktop::WindowPlacement>,
    pub(crate) last_playback: Option<PlaybackBookmark>,
    pub(crate) recent_search_tracks: Vec<Track>,
    pub(crate) search_history_migrated: bool,
    pub(crate) start_page: preferences::StartPage,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) private_search: bool,
    pub(crate) hide_artwork: bool,
    pub(crate) hide_album_column: bool,
    pub(crate) reduce_motion: bool,
    pub(crate) last_local_volume: Option<u8>,
    pub(crate) startup_volume: Option<u8>,
    pub(crate) cache_limit_mb: u32,
    pub(crate) startup_in_tray: bool,
    pub(crate) audio: audio_prefs::Settings,
    pub(crate) display: appearance::Settings,
    pub(crate) top_music_range: spotify::insights::Range,
    pub(crate) lyrics_font_size: u8,
    pub(crate) lyrics_manual_by_default: bool,
    pub(crate) seek_step_seconds: u8,
    pub(crate) volume_step_percent: u8,
    pub(crate) connect_device_name: String,
    pub(crate) dj_model: String,
    pub(crate) language: i18n::Language,
    #[serde(default = "default_true")]
    pub(crate) ad_blocker: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PlaybackBookmark {
    pub(crate) track: Track,
    pub(crate) position_ms: u64,
}

pub(crate) struct PendingHandoff {
    pub(crate) bookmark: PlaybackBookmark,
    pub(crate) captured: Instant,
    pub(crate) was_playing: bool,
}

impl PendingHandoff {
    pub(crate) fn correction(&self, state: &PlaybackState) -> Option<u64> {
        let track = state.track.as_ref()?;
        if self.captured.elapsed() > Duration::from_secs(10)
            || track.uri != self.bookmark.track.uri
            || track.duration_ms == 0
        {
            return None;
        }
        let elapsed = if self.was_playing {
            self.captured.elapsed().as_millis() as u64
        } else {
            0
        };
        let expected = self
            .bookmark
            .position_ms
            .saturating_add(elapsed)
            .min(track.duration_ms.saturating_sub(250));
        (state.progress_ms < 5000 && expected > state.progress_ms.saturating_add(2500))
            .then_some(expected)
    }
}

impl AppConfig {
    pub(crate) fn lyric_size(&self, narrow: bool) -> f32 {
        let size = match self.lyrics_font_size {
            24..=44 => self.lyrics_font_size,
            _ => 32,
        };
        f32::from(size) * if narrow { 0.875 } else { 1.0 }
    }
    pub(crate) fn seek_step(&self) -> u64 {
        match self.seek_step_seconds {
            5 | 10 | 15 | 30 => self.seek_step_seconds as u64,
            _ => 10,
        }
    }
    pub(crate) fn volume_step(&self) -> f32 {
        f32::from(match self.volume_step_percent {
            1 | 2 | 5 | 10 => self.volume_step_percent,
            _ => 5,
        }) / 100.0
    }

    pub(crate) fn cache_mb(&self) -> u32 {
        match self.cache_limit_mb {
            100 | 250 | 500 | 1000 => self.cache_limit_mb,
            _ => 100,
        }
    }

    pub(crate) fn sanitize_device_name(raw: &str) -> String {
        let cleaned: String = raw
            .chars()
            .filter(|c| !c.is_control())
            .collect::<String>()
            .trim()
            .chars()
            .take(48)
            .collect();
        if cleaned.is_empty() {
            local_player::DEVICE_NAME.into()
        } else {
            cleaned
        }
    }

    pub(crate) fn effective_device_name(&self) -> String {
        Self::sanitize_device_name(&self.connect_device_name)
    }
    pub(crate) fn path() -> PathBuf {
        // Retain the pre-Aurafy location to preserve settings and saved playback.
        dirs::config_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("Auralis")
            .join("config.json")
    }

    pub(crate) fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub(crate) fn save(&self) -> Result<(), String> {
        // UI tests must never overwrite the user's real settings or session.
        #[cfg(test)]
        {
            Ok(())
        }
        #[cfg(not(test))]
        {
            self.save_to(&Self::path())
        }
    }

    pub(crate) fn save_to(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Could not create Aurafy config folder: {e}"))?;
        }
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Could not serialize Aurafy settings: {e}"))?;
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temporary, data).map_err(|e| format!("Could not save Aurafy settings: {e}"))?;
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(temporary);
            return Err(format!("Could not replace Aurafy settings: {error}"));
        }
        Ok(())
    }
}

pub(crate) enum AppEvent {
    LyricsLoaded {
        key: lyrics::TrackKey,
        result: lyrics::Outcome,
    },
    ArtistLoaded {
        generation: u64,
        result: Result<catalog::ArtistOverview, String>,
        token: Option<TokenState>,
    },
    ArtistSongsLoaded {
        generation: u64,
        result: Result<catalog::SongPage, String>,
        token: Option<TokenState>,
    },
    ArtistAlbumsLoaded {
        generation: u64,
        result: Result<catalog::Page<catalog::Album>, String>,
        token: Option<TokenState>,
    },
    AlbumLoaded {
        generation: u64,
        result: Result<catalog::AlbumDetails, String>,
        token: Option<TokenState>,
    },
    Desktop(desktop::Action),
    LocalPlayer {
        generation: u64,
        event: local_player::Event,
    },
    AutoplayLoaded {
        generation: u64,
        result: Result<Vec<Track>, String>,
        token: Option<TokenState>,
    },
    Connected(Result<TokenState, String>),
    AuthBrowserReady {
        url: String,
        error: Option<String>,
        uses_fallback_port: bool,
    },
    SearchFinished {
        query: String,
        result: Result<Vec<Track>, String>,
        token: Option<TokenState>,
    },
    SearchCatalogFinished {
        query: String,
        result: Result<catalog::SearchCatalog, String>,
        token: Option<TokenState>,
    },
    ActionFinished {
        action: String,
        result: Result<(), String>,
        token: Option<TokenState>,
    },
    PlayerSynced {
        playback: Result<Option<PlaybackState>, String>,
        queue: Option<Result<Vec<Track>, String>>,
        token: Option<TokenState>,
    },
    LibraryLoaded {
        revision: u64,
        saved_tracks: Result<Vec<Track>, String>,
        playlists: Result<Vec<Playlist>, String>,
        token: Option<TokenState>,
    },
    PlaylistLoaded {
        playlist_id: String,
        tracks: Result<Vec<Track>, String>,
        token: Option<TokenState>,
    },
    CollectionPlayLoaded {
        generation: u64,
        start: spotify::playback::ContextStart,
        track: Option<Track>,
        result: Result<Vec<Track>, String>,
        token: Option<TokenState>,
    },
    PlaylistCreated {
        result: Result<Playlist, String>,
        token: Option<TokenState>,
    },
    ArtworkLoaded {
        url: String,
        bytes: Result<Vec<u8>, String>,
    },
    DevicesLoaded {
        devices: Result<Vec<Device>, String>,
        token: Option<TokenState>,
    },
    MembershipLoaded {
        generation: u64,
        playlist_id: String,
        uris: Result<HashSet<String>, String>,
        token: Option<TokenState>,
        done: bool,
    },
    InsightsLoaded {
        generation: u64,
        range: spotify::insights::Range,
        result: Result<spotify::insights::Data, spotify::insights::Error>,
        token: Option<TokenState>,
    },
    HomeDataLoaded {
        recent_tracks: Result<Vec<Track>, String>,
        profile: Result<spotify::UserProfile, String>,
        token: Option<TokenState>,
    },
    UpdateChecked(Result<updates::CheckOutcome, String>),
}

pub(crate) struct AurafyApp {
    pub(crate) ui_frames: u64,
    pub(crate) ui_started: Instant,
    pub(crate) ui_build_ms: f64,
    pub(crate) ui_repaint_causes: Vec<String>,
    pub(crate) config: AppConfig,
    pub(crate) logo_texture: egui::TextureHandle,
    pub(crate) tx: Sender<AppEvent>,
    pub(crate) rx: Receiver<AppEvent>,
    pub(crate) spotify_tx: Sender<AppEvent>,
    spotify_rx: Receiver<AppEvent>,
    pub(crate) token: Option<TokenState>,

    pub(crate) page: Page,
    pub(crate) search_query: String,
    pub(crate) search_rect: Option<Rect>,
    pub(crate) search_popup_rect: Option<Rect>,
    pub(crate) search_popup_open: bool,
    pub(crate) search_results: Vec<Track>,
    pub(crate) search_catalog: Option<catalog::SearchCatalog>,
    pub(crate) search_filter: SearchFilter,
    pub(crate) search_loading: bool,
    pub(crate) search_error: Option<String>,
    pub(crate) nav_history: Vec<NavTarget>,
    pub(crate) nav_forward: Vec<NavTarget>,
    pub(crate) nav_skip_record: bool,
    pub(crate) artist_name: String,
    pub(crate) artist_tracks: Vec<Track>,
    pub(crate) artist_loading: bool,
    pub(crate) artist_error: Option<String>,
    pub(crate) browse: browse::State,
    pub(crate) last_sent_query: String,
    pub(crate) search_edited_at: Option<Instant>,
    pub(crate) library_filter: String,
    pub(crate) playlist_track_filter: String,
    playlist_editor: Option<playlist_editor::Editor>,
    pub(crate) track_sort: TrackSort,
    pub(crate) track_sort_context: String,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) insights: spotify::insights::State,
    pub(crate) sleep_timer: Option<preferences::SleepTimer>,
    pub(crate) sleep_minutes: u32,
    pub(crate) startup_launch_handled: bool,
    pub(crate) startup_enabled: bool,
    pub(crate) startup_error: Option<String>,
    pub(crate) lyrics: lyrics::Library,
    pub(crate) lyrics_return_page: Page,
    pub(crate) lyrics_follow: bool,
    pub(crate) lyrics_last_line: Option<(lyrics::TrackKey, Option<usize>)>,

    pub(crate) demo_tracks: Vec<Track>,
    pub(crate) current_track: Track,
    pub(crate) player_layout: Option<player_layout::Geometry>,
    #[cfg(target_os = "windows")]
    pub(crate) player_capture: Option<player_capture::Session>,
    pub(crate) priority_queue: Vec<Track>,
    pub(crate) manual_queue_len: usize,
    pub(crate) priority_dispatched: Option<(String, Instant)>,
    pub(crate) priority_auto_armed: bool,
    pub(crate) last_manual_skip: Option<Instant>,
    pub(crate) suppress_upstream_queue: bool,
    pub(crate) queue_cleared_by_user: bool,
    pub(crate) autoplay_pending: bool,
    pub(crate) autoplay_generation: u64,
    pub(crate) active_collection_uri: Option<String>,
    pub(crate) collection_play_generation: u64,
    pub(crate) queue: Vec<Track>,
    pub(crate) is_playing: bool,
    pub(crate) playback_idle: bool,
    pub(crate) playback_context_uri: Option<String>,
    pub(crate) progress: f32,
    pub(crate) progress_ms: u64,
    pub(crate) duration_ms: u64,
    pub(crate) progress_anchor: Instant,
    pub(crate) seek_interacting: bool,
    pub(crate) volume: f32,
    pub(crate) unmuted_volume: f32,
    pub(crate) pre_ad_volume: Option<u8>,
    pub(crate) last_ad_skip: Option<Instant>,
    pub(crate) volume_control: volume::Control,
    pub(crate) shuffle_enabled: bool,
    pub(crate) repeat_state: String,
    pub(crate) device_name: Option<String>,
    pub(crate) device_name_draft: String,
    pub(crate) device_id: Option<String>,
    pub(crate) devices: Vec<Device>,
    pub(crate) devices_loading: bool,
    pub(crate) device_picker_open: bool,
    pub(crate) device_popup_rect: Option<Rect>,
    pub(crate) device_button_rect: Option<Rect>,
    pub(crate) devices_error: Option<String>,
    pub(crate) next_devices_poll: Instant,
    pub(crate) local_player: Option<local_player::Player>,
    pub(crate) free_player: Option<free_player::FreePlayer>,
    pub(crate) local_player_generation: u64,
    pub(crate) select_local_on_ready: bool,
    pub(crate) resume_on_local_ready: bool,
    pub(crate) local_status: local_player::Status,
    pub(crate) local_starting: bool,
    pub(crate) local_key_input: String,
    pub(crate) playback_error: Option<String>,
    pub(crate) pending_handoff: Option<PendingHandoff>,
    pub(crate) follow_remote_device: bool,
    pub(crate) requested_track_uri: Option<String>,
    pub(crate) restoring_playback: bool,
    pub(crate) show_queue: bool,
    pub(crate) queue_show_recent: bool,

    pub(crate) saved_tracks: Vec<Track>,
    pub(crate) recent_tracks: Vec<Track>,
    pub(crate) recent_loading: bool,
    pub(crate) recent_error: Option<String>,
    pub(crate) last_home_load: Option<Instant>,
    pub(crate) user_name: Option<String>,
    pub(crate) user_profile: Option<spotify::UserProfile>,
    pub(crate) profile_error: Option<String>,
    pub(crate) saved_track_ids: HashSet<String>,
    pub(crate) playlist_membership: HashMap<String, HashSet<String>>,
    pub(crate) membership_loading: bool,
    pub(crate) membership_attempted: HashSet<String>,
    pub(crate) membership_generation: u64,
    pub(crate) library_changes: HashMap<String, (Track, bool)>,
    pub(crate) library_revision: u64,
    pub(crate) playlists: Vec<Playlist>,
    pub(crate) selected_playlist: Option<Playlist>,
    pub(crate) playlist_tracks: Vec<Track>,
    pub(crate) playlist_error: Option<String>,
    pub(crate) show_create_playlist: bool,
    pub(crate) new_playlist_name: String,
    pub(crate) new_playlist_description: String,
    pub(crate) new_playlist_public: bool,
    pub(crate) show_add_to_playlist: bool,
    pub(crate) playlist_add_pending: Option<String>,
    pub(crate) playlist_remove_target: Option<(String, String)>,
    pub(crate) playlist_add_error: Option<String>,
    pub(crate) playlist_picker_filter: String,
    pub(crate) playlist_picker_anchor: Option<Rect>,
    pub(crate) playlist_picker_rect: Option<Rect>,
    pub(crate) current_track_menu_rect: Option<Rect>,
    pub(crate) playlist_picker_track_uri: String,
    pub(crate) playlist_picker_added: HashSet<String>,
    pub(crate) track_to_add_to_playlist: Option<Track>,
    pub(crate) playlist_creation_in_progress: bool,
    pub(crate) library_loading: bool,
    pub(crate) library_refresh_needed: bool,
    pub(crate) library_error: Option<String>,
    pub(crate) playlist_loading: bool,

    pub(crate) artwork_cache: artwork::Cache,
    pub(crate) artwork_pending: HashSet<String>,
    pub(crate) artwork_failed: HashSet<String>,

    pub(crate) show_settings: bool,
    pub(crate) auth_in_progress: bool,
    pub(crate) auth_error: Option<String>,
    pub(crate) auth_url: Option<String>,
    pub(crate) auth_uses_fallback_port: bool,
    pub(crate) auth_cancel: Option<Arc<AtomicBool>>,
    pub(crate) player_poll_in_progress: bool,
    pub(crate) last_player_poll: Option<Instant>,
    pub(crate) last_queue_poll: Option<Instant>,
    pub(crate) last_library_poll: Option<Instant>,
    pub(crate) status_message: String,
    pub(crate) status_until: Instant,
    pub(crate) last_session_save: Instant,
    pub(crate) tray: Option<desktop::Tray>,
    pub(crate) tray_error: Option<String>,
    pub(crate) window_state: desktop::WindowState,
    pub(crate) window_changed_at: Option<Instant>,
    pub(crate) window_restore_pending: Option<desktop::WindowPlacement>,
    #[cfg(target_os = "windows")]
    pub(crate) window_geometry: Option<desktop::Geometry>,
    #[cfg(target_os = "windows")]
    pub(crate) display_bounds: Option<[i32; 8]>,
    #[cfg(target_os = "windows")]
    pub(crate) display_changed_at: Option<Instant>,
    #[cfg(target_os = "windows")]
    pub(crate) desktop_instance: Option<desktop::Instance>,
    #[cfg(target_os = "windows")]
    pub(crate) smtc: Option<smtc::Controls>,
    pub(crate) updates: updates::State,
}

impl AurafyApp {
    pub(crate) fn clear_search_history(&mut self) {
        self.config.recent_searches.clear();
        self.config.recent_search_tracks.clear();
        self.config.search_history_migrated = true;
        self.config.last_query.clear();
        self.search_query.clear();
        self.last_sent_query.clear();
        self.search_results.clear();
        self.search_catalog = None;
        self.search_loading = false;
        self.search_edited_at = None;
        self.search_popup_open = false;
        self.save_session();
    }

    pub(crate) fn set_search_history_enabled(&mut self, enabled: bool) {
        self.config.private_search = !enabled;
        if !enabled {
            self.clear_search_history();
        } else {
            self.save_session();
        }
    }

    pub(crate) fn set_artwork_visible(&mut self, visible: bool) {
        self.config.hide_artwork = !visible;
        if !visible {
            let avatar = self
                .user_profile
                .as_ref()
                .and_then(|p| p.image_url.as_deref());
            self.artwork_cache
                .retain(|url, _| Some(url.as_str()) == avatar);
        }
        self.save_session();
    }

    pub(crate) fn tick_sleep_timer(&mut self, ctx: &egui::Context) {
        let position = self.current_progress_ms();
        let key = if self.current_track.uri.is_empty() {
            &self.current_track.id
        } else {
            &self.current_track.uri
        };
        let expires = self
            .sleep_timer
            .as_mut()
            .is_some_and(|timer| timer.expired(Instant::now(), key, position));
        if expires {
            self.sleep_timer = None;
            self.select_local_on_ready = false;
            if self.is_playing {
                // A timer must never activate this computer just to pause it.
                if self.local_status.active && self.local_player.is_some() {
                    self.local_command("pause", vec![local_player::request("pause")]);
                } else if self.is_connected() {
                    self.start_spotify_action(
                        "pause".into(),
                        |access| spotify::playback_command(access, "pause"),
                        ctx,
                    );
                } else if self.local_player.is_some() {
                    self.local_command("pause", vec![local_player::request("pause")]);
                } else if self.is_demo_mode() {
                    self.send_playback_action("pause", ctx);
                }
            }
            self.set_status("Sleep timer finished");
        }
    }
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::build(&cc.egui_ctx, AppConfig::load());
        #[cfg(target_os = "windows")]
        audio_session::start();
        #[cfg(target_os = "windows")]
        {
            app.window_geometry = desktop::Geometry::new(cc).ok();
            if app.window_geometry.is_some() {
                app.window_restore_pending = app.config.window_placement;
            }
        }
        #[cfg(target_os = "windows")]
        match desktop::Tray::new(
            cc,
            app.tx.clone(),
            0xe8f27dc0_c2cb_43eb_a2f5_4832d18a2bb9,
            app.config.display.tray_style,
        ) {
            Ok(tray) => app.tray = Some(tray),
            Err(error) => app.tray_error = Some(format!("Could not create the tray icon: {error}")),
        }
        #[cfg(target_os = "windows")]
        {
            app.smtc = smtc::Controls::new(cc, app.tx.clone()).ok();
        }
        // Restore the refresh token from Windows Credential Manager. The refresh
        // token itself is never written into config.json or the project folder.
        if !app.config.client_id.trim().is_empty() {
            if let Some(refresh_token) = spotify::load_refresh_token() {
                app.auth_in_progress = true;
                app.auth_cancel = Some(Arc::new(AtomicBool::new(false)));
                let tx = app.spotify_tx.clone();
                let client_id = app.config.client_id.clone();
                let ctx = cc.egui_ctx.clone();
                thread::spawn(move || {
                    let result = spotify::restore_session(&refresh_token, &client_id);
                    let _ = tx.send(AppEvent::Connected(result));
                    ctx.request_repaint();
                });
                app.set_status("Restoring your Spotify session…");
            }
        }
        if app.config.local_playback {
            app.enable_local_playback(&cc.egui_ctx);
        }
        app.startup_launch_handled = !(std::env::args().any(|arg| arg == "--windows-startup")
            && std::env::args().any(|arg| arg == "--start-in-tray"));
        match startup::enabled() {
            Ok(enabled) => app.startup_enabled = enabled,
            Err(error) => app.startup_error = Some(error),
        }
        app.publish_runtime_status();
        app
    }

    pub(crate) fn build(ctx: &egui::Context, config: AppConfig) -> Self {
        appearance::store(
            ctx,
            appearance::resolve(config.display, ctx.system_theme(), None),
        );
        let initial_palette = appearance::palette(ctx);
        ctx.set_theme(match config.display.theme {
            appearance::Theme::System => egui::ThemePreference::System,
            _ if initial_palette.dark => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::Light,
        });
        ui::configure(ctx);
        ctx.options_mut(|options| options.zoom_with_keyboard = false);
        ctx.set_zoom_factor(config.display.zoom());

        let (tx, rx) = mpsc::channel();
        let (spotify_tx, spotify_rx) = mpsc::channel();
        let demo_tracks = demo_tracks();
        let configured = !config.client_id.trim().is_empty() || config.local_playback;
        let current_track = if configured {
            config
                .last_playback
                .as_ref()
                .map(|saved| saved.track.clone())
                .unwrap_or_else(|| Track {
                    name: "Restoring playback…".into(),
                    artist: "Connecting to your Spotify session".into(),
                    ..Default::default()
                })
        } else {
            demo_tracks[0].clone()
        };
        let position_ms = if configured {
            config
                .last_playback
                .as_ref()
                .map(|saved| saved.position_ms)
                .unwrap_or(0)
                .min(current_track.duration_ms)
        } else {
            0
        };
        let duration_ms = current_track.duration_ms.max(1);
        let queue = if configured {
            Vec::new()
        } else {
            demo_tracks.iter().skip(1).take(7).cloned().collect()
        };
        let now = Instant::now();

        let show_queue = config.queue_open;
        let page = match config.start_page {
            preferences::StartPage::LastView => config.last_page,
            preferences::StartPage::Home => Page::Home,
            preferences::StartPage::Search => Page::Search,
            preferences::StartPage::Library => Page::Library,
        };
        let settings_tab = config.settings_tab;
        let initial_volume = config
            .startup_volume
            .or(config.last_local_volume)
            .unwrap_or(40)
            .min(100);
        ctx.style_mut(|style| style.animation_time = if config.reduce_motion { 0.0 } else { 0.10 });
        let search_query =
            if config.private_search || config.start_page != preferences::StartPage::LastView {
                String::new()
            } else {
                config.last_query.clone()
            };
        let artist_name = config.last_artist.clone();
        let browse = browse::State::restore(&config);
        let restore_search = configured && page == Page::Search && !search_query.trim().is_empty();
        let select_local_on_ready = config.local_playback;
        let lyrics_follow = !config.lyrics_manual_by_default;
        let top_music_range = config.top_music_range;
        let device_name_draft = config.effective_device_name();
        let app = Self {
            ui_frames: 0,
            ui_started: Instant::now(),
            ui_build_ms: 0.0,
            ui_repaint_causes: Vec::new(),
            config,
            logo_texture: desktop::logo_texture(ctx),
            tx,
            rx,
            spotify_tx,
            spotify_rx,
            token: None,
            page,
            search_query: search_query.clone(),
            search_rect: None,
            search_popup_rect: None,
            search_popup_open: false,
            search_results: Vec::new(),
            search_catalog: None,
            search_filter: SearchFilter::All,
            search_loading: restore_search,
            search_error: None,
            nav_history: Vec::new(),
            nav_forward: Vec::new(),
            nav_skip_record: false,
            artist_name,
            artist_tracks: Vec::new(),
            artist_loading: page == Page::Artist,
            artist_error: None,
            browse,
            last_sent_query: if restore_search {
                search_query
            } else {
                String::new()
            },
            search_edited_at: None,
            library_filter: String::new(),
            playlist_track_filter: String::new(),
            playlist_editor: None,
            track_sort: TrackSort::default(),
            track_sort_context: String::new(),
            settings_tab,
            insights: spotify::insights::State {
                range: top_music_range,
                ..Default::default()
            },
            sleep_timer: None,
            sleep_minutes: 30,
            startup_launch_handled: true,
            startup_enabled: false,
            startup_error: None,
            lyrics: lyrics::Library::default(),
            lyrics_return_page: Page::Home,
            lyrics_follow,
            lyrics_last_line: None,
            demo_tracks,
            current_track,
            player_layout: None,
            #[cfg(target_os = "windows")]
            player_capture: player_capture::Session::from_env(),
            priority_queue: Vec::new(),
            manual_queue_len: 0,
            priority_dispatched: None,
            priority_auto_armed: false,
            last_manual_skip: None,
            suppress_upstream_queue: false,
            queue_cleared_by_user: false,
            autoplay_pending: false,
            autoplay_generation: 0,
            active_collection_uri: None,
            collection_play_generation: 0,
            queue,
            is_playing: false,
            playback_idle: false,
            playback_context_uri: None,
            progress: position_ms as f32 / duration_ms as f32,
            progress_ms: position_ms,
            duration_ms,
            progress_anchor: now,
            seek_interacting: false,
            volume: f32::from(initial_volume) / 100.0,
            unmuted_volume: f32::from(if initial_volume == 0 {
                40
            } else {
                initial_volume
            }) / 100.0,
            pre_ad_volume: None,
            last_ad_skip: None,
            volume_control: volume::Control::default(),
            shuffle_enabled: false,
            repeat_state: "off".into(),
            device_name: None,
            device_name_draft,
            device_id: None,
            devices: Vec::new(),
            devices_loading: false,
            device_picker_open: false,
            device_popup_rect: None,
            device_button_rect: None,
            devices_error: None,
            next_devices_poll: now + Duration::from_secs(20),
            local_player: None,
            free_player: None,
            local_player_generation: 0,
            select_local_on_ready,
            resume_on_local_ready: false,
            local_status: local_player::Status::default(),
            local_starting: false,
            local_key_input: String::new(),
            playback_error: None,
            pending_handoff: None,
            follow_remote_device: false,
            requested_track_uri: None,
            restoring_playback: configured,
            show_queue,
            saved_tracks: Vec::new(),
            recent_tracks: Vec::new(),
            recent_loading: false,
            recent_error: None,
            last_home_load: None,
            queue_show_recent: false,
            user_name: None,
            user_profile: None,
            profile_error: None,
            saved_track_ids: HashSet::new(),
            playlist_membership: HashMap::new(),
            membership_loading: false,
            membership_attempted: HashSet::new(),
            membership_generation: 0,
            library_changes: HashMap::new(),
            library_revision: 0,
            playlists: Vec::new(),
            selected_playlist: None,
            playlist_tracks: Vec::new(),
            playlist_error: None,
            show_create_playlist: false,
            new_playlist_name: String::new(),
            new_playlist_description: String::new(),
            new_playlist_public: false,
            show_add_to_playlist: false,
            playlist_add_pending: None,
            playlist_remove_target: None,
            playlist_add_error: None,
            playlist_picker_filter: String::new(),
            playlist_picker_anchor: None,
            playlist_picker_rect: None,
            current_track_menu_rect: None,
            playlist_picker_track_uri: String::new(),
            playlist_picker_added: HashSet::new(),
            track_to_add_to_playlist: None,
            playlist_creation_in_progress: false,
            library_loading: false,
            library_refresh_needed: false,
            library_error: None,
            playlist_loading: false,
            artwork_cache: artwork::Cache::default(),
            artwork_pending: HashSet::new(),
            artwork_failed: HashSet::new(),
            show_settings: false,
            auth_in_progress: false,
            auth_error: None,
            auth_url: None,
            auth_uses_fallback_port: false,
            auth_cancel: None,
            player_poll_in_progress: false,
            last_player_poll: None,
            last_queue_poll: None,
            last_library_poll: None,
            status_message: if configured {
                "Restoring your music…".into()
            } else {
                "Demo mode · Connect Spotify to load your music".into()
            },
            status_until: now + Duration::from_secs(8),
            last_session_save: now,
            tray: None,
            tray_error: None,
            window_state: desktop::WindowState::default(),
            window_changed_at: None,
            window_restore_pending: None,
            #[cfg(target_os = "windows")]
            window_geometry: None,
            #[cfg(target_os = "windows")]
            display_bounds: None,
            #[cfg(target_os = "windows")]
            display_changed_at: None,
            #[cfg(target_os = "windows")]
            desktop_instance: None,
            #[cfg(target_os = "windows")]
            smtc: None,
            updates: updates::State::default(),
        };

        app
    }

    pub(crate) fn check_for_updates(&mut self, ctx: &egui::Context) {
        if self.updates.checking {
            return;
        }
        self.updates.checking = true;
        self.updates.error = None;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = updates::check_latest(env!("CARGO_PKG_VERSION"));
            let _ = tx.send(AppEvent::UpdateChecked(result));
            ctx.request_repaint();
        });
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.token.is_some()
    }

    pub(crate) fn publish_runtime_status(&self) {
        #[cfg(not(test))]
        {
            let root = local_player::directory();
            let log_error = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(root.join("startup-status.log"))
                .err()
                .and_then(|error| error.raw_os_error());
            let status = serde_json::json!({
                "version":env!("CARGO_PKG_VERSION"), "app_pid":std::process::id(),
                "ui_frames":self.ui_frames,"ui_elapsed_seconds":self.ui_started.elapsed().as_secs_f64(),"ui_build_ms":self.ui_build_ms,"ui_repaint_causes":self.ui_repaint_causes,
                "executable":std::env::current_exe().ok(), "working_directory":std::env::current_dir().ok(),
                "player_directory":root, "local_app_data_env":std::env::var_os("LOCALAPPDATA"),
                "config_path":AppConfig::path(), "diagnostic_write_error":log_error,
                "native_pid":fs::read_to_string(root.join("data/soloist.pid")).ok(),
                "running":self.local_status.running, "connecting":self.local_status.connecting,
                "queue_items":self.priority_queue.len() + self.queue.len(), "playback_idle":self.playback_idle,
                "is_playing":self.is_playing,
                "paired":self.local_status.paired, "active":self.local_status.active,
                "auto_select_pending":self.select_local_on_ready,
                "tray_registered":self.tray.as_ref().is_some_and(desktop::Tray::registered),
                "window_hidden":self.window_state.hidden, "tray_error":self.tray_error,
                "artwork_cached":self.artwork_cache.len(),
                "artwork_visible":self.artwork_cache.visible_len(),
                "artwork_pending":self.artwork_pending.len(),
                "profile_verified":self.user_profile.as_ref().is_some_and(|profile|!profile.id.is_empty()),
                "playlists_loaded":self.playlists.len(),
                "top_tracks_loaded":self.insights.cache.get(&self.insights.range).map_or(0,|data|data.tracks.len()),
                "top_artists_loaded":self.insights.cache.get(&self.insights.range).map_or(0,|data|data.artists.len()),
                "top_permission_needed":self.insights.permission_needed,
                "owned_playlists":self.playlists.iter().filter(|playlist|self.user_profile.as_ref().is_some_and(|profile|playlist.is_owned_by(&profile.id))).count(),
                "window_placement":self.config.window_placement
            });
            if let Some(parent) = AppConfig::path().parent() {
                let _ = fs::create_dir_all(parent);
                if let Ok(text) = serde_json::to_string_pretty(&status) {
                    let _ = fs::write(parent.join("runtime-status.json"), text);
                }
            }
        }
    }

    pub(crate) fn is_demo_mode(&self) -> bool {
        !self.is_connected()
            && self.config.client_id.trim().is_empty()
            && !self.config.local_playback
    }

    pub(crate) fn playback_bookmark(&self) -> Option<PlaybackBookmark> {
        if !self.current_track.uri.starts_with("spotify:") {
            return None;
        }
        Some(PlaybackBookmark {
            track: self.current_track.clone(),
            position_ms: self.current_progress_ms().min(self.duration_ms),
        })
    }

    pub(crate) fn remember_window(&mut self, _ctx: Option<&egui::Context>) {
        #[cfg(target_os = "windows")]
        if !self.window_state.hidden {
            if let Some(geometry) = &self.window_geometry {
                if let Some(ctx) = _ctx
                    && let Some(bounds) = geometry.display_bounds()
                    && self
                        .display_bounds
                        .replace(bounds)
                        .is_some_and(|previous| previous != bounds)
                {
                    self.display_changed_at = Some(Instant::now());
                    geometry.invalidate();
                    ctx.request_repaint();
                }
                if let Some(changed) = self.display_changed_at {
                    let Some(ctx) = _ctx else {
                        return;
                    };
                    if changed.elapsed() < Duration::from_millis(750) {
                        ctx.request_repaint_after(Duration::from_millis(60));
                        return;
                    }
                    self.display_changed_at = None;
                    if geometry.collapsed()
                        && let Some(placement) = self.config.window_placement
                    {
                        geometry.restore(placement);
                    }
                    geometry.invalidate();
                    ctx.request_repaint();
                    return;
                }
                if geometry.collapsed() {
                    return;
                }
            }
            if let Some(saved) = self.window_restore_pending {
                let Some(geometry) = &self.window_geometry else {
                    return;
                };
                // eframe first shows its native window after the initial render.
                // Restoring before that lets the initial Windows show/move events
                // overwrite the saved position. Wait until the window is visible.
                if !geometry.visible() {
                    return;
                }
                geometry.restore(saved);
                self.window_restore_pending = None;
            }
            if let Some(saved) = self
                .window_geometry
                .as_ref()
                .and_then(desktop::Geometry::capture)
            {
                if self.config.window_placement != Some(saved) {
                    self.config.window_placement = Some(saved);
                    self.window_changed_at = Some(Instant::now());
                }
            }
        }
        if self
            .window_changed_at
            .is_some_and(|changed| changed.elapsed() >= Duration::from_millis(500))
        {
            self.save_session();
            self.window_changed_at = None;
        }
    }

    pub(crate) fn save_session(&mut self) {
        self.config.last_page = self.page;
        self.config.last_query = if self.config.private_search {
            String::new()
        } else {
            self.search_query.clone()
        };
        self.config.settings_tab = self.settings_tab;
        if self.local_status.active && self.use_local_playback() {
            self.config.last_local_volume =
                Some((self.volume * 100.0).round().clamp(0.0, 100.0) as u8);
        }
        self.config.last_artist = self.artist_name.clone();
        self.config.last_artist_id = self.browse.artist_id.clone();
        self.config.last_album = self.browse.album_seed.clone();
        if let Some(bookmark) = self.playback_bookmark() {
            self.config.last_playback = Some(bookmark);
        }
        let _ = self.config.save();
        self.last_session_save = Instant::now();
    }

    pub(crate) fn remember_search_track(&mut self, track: &Track) {
        if self.config.private_search || !track.uri.starts_with("spotify:") {
            return;
        }
        self.config
            .recent_search_tracks
            .retain(|item| item.uri != track.uri);
        self.config.recent_search_tracks.insert(0, track.clone());
        self.config.recent_search_tracks.truncate(12);
        self.save_session();
    }

    pub(crate) fn play_search_track(&mut self, track: Track, ctx: &egui::Context) {
        self.remember_search_track(&track);
        self.search_popup_open = false;
        self.play_track(track, ctx);
    }

    pub(crate) fn migrate_recent_search_tracks(&mut self) {
        if self.config.private_search || self.config.search_history_migrated {
            return;
        }
        self.config.search_history_migrated = true;
        if !self.config.recent_search_tracks.is_empty() {
            self.save_session();
            return;
        }
        let normalize = |value: &str| {
            value
                .chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let mut tracks = Vec::new();
        for query in &self.config.recent_searches {
            let query = normalize(query);
            if query.is_empty() {
                continue;
            }
            if let Some(track) = self
                .recent_tracks
                .iter()
                .chain(std::iter::once(&self.current_track))
                .find(|track| {
                    track.uri.starts_with("spotify:")
                        && (normalize(&track.name).contains(&query)
                            || normalize(&track.artist).contains(&query))
                })
            {
                if !tracks.iter().any(|item: &Track| item.uri == track.uri) {
                    tracks.push(track.clone());
                }
            }
        }
        self.config.recent_search_tracks = tracks;
        self.save_session();
    }

    pub(crate) fn capture_handoff(&mut self) {
        self.pending_handoff = self.playback_bookmark().map(|bookmark| PendingHandoff {
            bookmark,
            captured: Instant::now(),
            was_playing: self.is_playing,
        });
    }

    pub(crate) fn accept_token(&mut self, token: TokenState) {
        if self.token.as_ref().is_some_and(|current| {
            Arc::ptr_eq(&current.refresh_cache, &token.refresh_cache)
                && token.revision < current.revision
        }) {
            return;
        }
        // Spotify may rotate the refresh token. Keep the newest one in the OS keychain.
        let old_refresh = self
            .token
            .as_ref()
            .and_then(|token| token.refresh_token.as_deref());
        let storage_result = if token.refresh_token.as_deref() != old_refresh {
            token
                .refresh_token
                .as_deref()
                .map(spotify::save_refresh_token)
                .transpose()
        } else {
            Ok(None)
        };
        self.token = Some(token);
        if let Err(error) = storage_result {
            self.set_status(format!(
                "Spotify connected, but Windows could not store the session: {error}"
            ));
        }
    }

    pub(crate) fn current_nav_target(&self) -> NavTarget {
        match self.page {
            Page::Library => {
                if let Some(playlist) = &self.selected_playlist {
                    NavTarget::Playlist(playlist.id.clone())
                } else {
                    NavTarget::Page(Page::Library)
                }
            }
            Page::Artist => NavTarget::Artist {
                name: self.artist_name.clone(),
                id: self.browse.artist_id.clone(),
            },
            Page::Album => {
                if let Some(seed) = &self.browse.album_seed {
                    NavTarget::Album(seed.clone())
                } else {
                    NavTarget::Page(Page::Home)
                }
            }
            other => NavTarget::Page(other),
        }
    }

    pub(crate) fn record_nav(&mut self) {
        if self.nav_skip_record {
            return;
        }
        let current = self.current_nav_target();
        if self.nav_history.last() != Some(&current) {
            self.nav_history.push(current);
            if self.nav_history.len() > 32 {
                let extra = self.nav_history.len() - 32;
                self.nav_history.drain(..extra);
            }
        }
        self.nav_forward.clear();
    }

    pub(crate) fn can_go_back(&self) -> bool {
        !self.nav_history.is_empty()
    }

    pub(crate) fn can_go_forward(&self) -> bool {
        !self.nav_forward.is_empty()
    }

    pub(crate) fn go_back(&mut self, ctx: &egui::Context) {
        let Some(target) = self.nav_history.pop() else {
            return;
        };
        let current = self.current_nav_target();
        if self.nav_forward.last() != Some(&current) {
            self.nav_forward.push(current);
        }
        self.apply_nav_target(target, ctx);
    }

    pub(crate) fn go_forward(&mut self, ctx: &egui::Context) {
        let Some(target) = self.nav_forward.pop() else {
            return;
        };
        let current = self.current_nav_target();
        if self.nav_history.last() != Some(&current) {
            self.nav_history.push(current);
        }
        self.apply_nav_target(target, ctx);
    }

    fn apply_nav_target(&mut self, target: NavTarget, ctx: &egui::Context) {
        self.nav_skip_record = true;
        match target {
            NavTarget::Page(page) => self.navigate(page, ctx),
            NavTarget::Playlist(id) => {
                if let Some(playlist) = self.playlists.iter().find(|p| p.id == id).cloned() {
                    self.page = Page::Library;
                    self.open_playlist(playlist, ctx);
                } else {
                    self.navigate(Page::Library, ctx);
                }
            }
            NavTarget::Artist { name, id } => self.open_artist_id(name, id, ctx),
            NavTarget::Album(seed) => self.open_album(seed, ctx),
        }
        self.nav_skip_record = false;
    }
}
