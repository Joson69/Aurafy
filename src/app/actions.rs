use super::*;

impl AurafyApp {
    pub(crate) fn start_auth(&mut self, ctx: &egui::Context) {
        if self.auth_in_progress {
            return;
        }
        self.show_settings = true;
        if let Err(error) = spotify::validate_client_id(&self.config.client_id) {
            self.auth_error = Some(error.clone());
            self.set_status(error);
            return;
        }
        self.config.client_id = self.config.client_id.trim().to_owned();
        if let Err(error) = self.config.save() {
            self.auth_error = Some(error.clone());
            self.set_status(error);
            return;
        }
        self.auth_error = None;
        self.auth_url = None;
        self.auth_in_progress = true;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.auth_cancel = Some(cancelled.clone());
        self.set_status("Opening Spotify sign-in in your browser…");
        let tx = self.tx.clone();
        let client_id = self.config.client_id.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result =
                spotify::authorize(&client_id, &cancelled, |url, error, uses_fallback_port| {
                    let _ = tx.send(AppEvent::AuthBrowserReady {
                        url,
                        error,
                        uses_fallback_port,
                    });
                    ctx.request_repaint();
                });
            let _ = tx.send(AppEvent::Connected(result));
            ctx.request_repaint();
        });
    }

    pub(crate) fn disconnect_spotify(&mut self) {
        self.ask = Default::default();
        self.stop_local_playback();
        self.insights.reset();
        self.membership_generation = self.membership_generation.wrapping_add(1);
        self.membership_loading = false;
        self.playlist_membership.clear();
        self.membership_attempted.clear();
        spotify::clear_refresh_token();
        self.token = None;
        self.artist_name.clear();
        self.artist_tracks.clear();
        self.artist_loading = false;
        self.artist_error = None;
        self.browse = browse::State::default();
        self.config.last_artist.clear();
        self.config.last_playback = None;
        self.config.recent_search_tracks.clear();
        self.pending_handoff = None;
        self.requested_track_uri = None;
        self.is_playing = false;
        self.device_name = None;
        self.device_id = None;
        self.devices.clear();
        self.devices_loading = false;
        self.current_track = Track {
            name: "Nothing playing yet".into(),
            artist: "Connect Spotify to play your music".into(),
            ..Default::default()
        };
        self.progress = 0.0;
        self.progress_ms = 0;
        self.duration_ms = self.current_track.duration_ms.max(1);
        self.progress_anchor = Instant::now();
        self.recent_tracks.clear();
        self.recent_loading = false;
        self.recent_error = None;
        self.last_home_load = None;
        self.user_name = None;
        self.user_profile = None;
        self.profile_error = None;
        self.saved_tracks.clear();
        self.saved_track_ids.clear();
        self.library_changes.clear();
        self.library_refresh_needed = false;
        self.library_revision += 1;
        self.playlists.clear();
        self.selected_playlist = None;
        self.playlist_track_filter.clear();
        self.playlist_error = None;
        self.playlist_tracks.clear();
        self.show_add_to_playlist = false;
        self.playlist_add_pending = None;
        self.playlist_add_error = None;
        self.playlist_picker_added.clear();
        self.playlist_picker_track_uri.clear();
        self.track_to_add_to_playlist = None;
        self.priority_queue.clear();
        self.manual_queue_len = 0;
        self.priority_dispatched = None;
        self.priority_auto_armed = false;
        self.suppress_upstream_queue = false;
        self.active_collection_uri = None;
        self.collection_play_generation = self.collection_play_generation.wrapping_add(1);
        self.queue.clear();
        self.save_session();
        self.set_status("Disconnected from Spotify");
    }

    pub(crate) fn run_search(&mut self, query: String, ctx: &egui::Context) {
        let query = query.trim().to_string();
        if query.is_empty() {
            self.search_popup_open = true;
            return;
        }
        if !self.is_demo_mode() && !self.is_connected() {
            if self.page != Page::Search {
                self.record_nav();
            }
            self.page = Page::Search;
            self.last_sent_query = query;
            self.search_loading = self.auth_in_progress;
            self.search_error = if self.auth_in_progress {
                None
            } else {
                Some("Reconnect Spotify in Settings → Account to search your music.".into())
            };
            return;
        }
        if self.page != Page::Search {
            self.record_nav();
        }
        self.page = Page::Search;
        self.search_popup_open = false;
        self.search_loading = true;
        self.search_error = None;
        self.search_results.clear();
        self.search_catalog = None;
        self.last_sent_query = query.clone();
        if !self.config.private_search {
            self.config
                .recent_searches
                .retain(|entry| !entry.eq_ignore_ascii_case(&query));
            self.config.recent_searches.insert(0, query.clone());
            self.config.recent_searches.truncate(8);
            let _ = self.config.save();
        }

        if let Some(token) = self.token.clone() {
            let tx = self.tx.clone();
            let client_id = self.config.client_id.clone();
            let ctx = ctx.clone();
            thread::spawn(move || {
                let (token, result) = match spotify::refresh_if_needed(token, &client_id) {
                    Ok(token) => {
                        let result = spotify::catalog::search_catalog(&token.access_token, &query);
                        (Some(token), result)
                    }
                    Err(error) => (None, Err(error)),
                };
                let _ = tx.send(AppEvent::SearchCatalogFinished {
                    query,
                    result,
                    token,
                });
                ctx.request_repaint();
            });
        } else {
            let q = query.to_lowercase();
            let tracks: Vec<Track> = self
                .demo_tracks
                .iter()
                .filter(|track| {
                    track.name.to_lowercase().contains(&q)
                        || track.artist.to_lowercase().contains(&q)
                        || track.album.to_lowercase().contains(&q)
                })
                .cloned()
                .collect();
            let mut seen_artists = HashSet::new();
            let artists: Vec<catalog::Artist> = self
                .demo_tracks
                .iter()
                .filter(|track| {
                    track.artist.to_lowercase().contains(&q)
                        || track.name.to_lowercase().contains(&q)
                })
                .filter_map(|track| {
                    let name = track
                        .artist_names
                        .first()
                        .cloned()
                        .unwrap_or_else(|| track.artist.clone());
                    if name.is_empty() || !seen_artists.insert(name.to_lowercase()) {
                        return None;
                    }
                    Some(catalog::Artist {
                        id: track.artist_ids.first().cloned().unwrap_or_default(),
                        name,
                        image_url: track.artwork_url.clone(),
                    })
                })
                .collect();
            let mut seen_albums = HashSet::new();
            let albums: Vec<catalog::Album> = self
                .demo_tracks
                .iter()
                .filter(|track| {
                    track.album.to_lowercase().contains(&q)
                        || track.artist.to_lowercase().contains(&q)
                        || track.name.to_lowercase().contains(&q)
                })
                .filter_map(|track| {
                    if track.album.is_empty() || !seen_albums.insert(track.album.to_lowercase()) {
                        return None;
                    }
                    Some(catalog::Album {
                        id: track.album_id.clone(),
                        name: track.album.clone(),
                        artwork_url: track.artwork_url.clone(),
                        artist_names: track.artist_names.clone(),
                        artist_ids: track.artist_ids.clone(),
                        release_date: String::new(),
                        kind: "album".into(),
                        total_tracks: self
                            .demo_tracks
                            .iter()
                            .filter(|t| t.album == track.album)
                            .count(),
                    })
                })
                .collect();
            let playlists: Vec<Playlist> = self
                .playlists
                .iter()
                .filter(|p| p.name.to_lowercase().contains(&q))
                .cloned()
                .collect();
            self.search_results = tracks.clone();
            self.search_catalog = Some(catalog::SearchCatalog {
                tracks,
                artists,
                albums,
                playlists,
            });
            self.search_loading = false;
            self.set_status(
                "Showing local demo results · Connect Spotify for live catalogue search",
            );
        }
    }

    pub(crate) fn queue_volume(&mut self, finished: bool, ctx: &egui::Context) {
        if self.use_free_player() {
            let vol = f64::from(self.volume);
            if let Some(fp) = &self.free_player {
                fp.send(free_player::FreeCommand::SetVolume(vol));
            }
            self.config.last_local_volume =
                Some((self.volume * 100.0).round().clamp(0.0, 100.0) as u8);
            if finished {
                self.save_session();
            }
            return;
        }
        if self.is_connected() || self.use_local_playback() {
            self.volume_control.queue(
                (self.volume * 100.0).round() as u8,
                finished,
                Instant::now(),
            );
            self.flush_volume(ctx);
        }
        if finished {
            self.save_session();
        }
    }

    pub(crate) fn flush_volume(&mut self, ctx: &egui::Context) {
        let local = self.use_local_playback();
        let interval = Duration::from_millis(if local { 50 } else { 150 });
        if let Some(value) = self.volume_control.next(Instant::now(), interval) {
            let device = self.device_id.clone();
            self.start_spotify_action(
                "volume".into(),
                move |token| spotify::set_volume(token, value, device.as_deref()),
                ctx,
            );
        }
        if self.volume_control.waiting() {
            ctx.request_repaint_after(interval);
        }
    }

    pub(crate) fn start_spotify_action<F>(&mut self, action: String, work: F, ctx: &egui::Context)
    where
        F: FnOnce(&str) -> Result<(), String> + Send + 'static,
    {
        if action == "seek" {
            if let Some(timer) = &mut self.sleep_timer {
                timer.seeked(self.progress_ms);
            }
            self.pending_handoff = None;
            self.restoring_playback = false;
        }
        if self.use_free_player() {
            if action == "seek"
                && let Some(fp) = &self.free_player
            {
                fp.send(free_player::FreeCommand::Seek(self.progress_ms));
                return;
            }
            if action == "volume"
                && let Some(fp) = &self.free_player
            {
                fp.send(free_player::FreeCommand::SetVolume(f64::from(self.volume)));
                return;
            }
        }
        if self.use_local_playback() {
            if action == "volume" {
                self.config.last_local_volume =
                    Some((self.volume * 100.0).round().clamp(0.0, 100.0) as u8);
            }
            let request = match action.as_str() {
                "seek" => Some(
                    serde_json::json!({"type":"command","command":"seek","position_ms":self.progress_ms}),
                ),
                "volume" => Some(
                    serde_json::json!({"type":"command","command":"set_volume","volume":(self.volume*100.0).round() as u8}),
                ),
                "shuffle" => Some(
                    serde_json::json!({"type":"command","command":"set_shuffle","enabled":self.shuffle_enabled}),
                ),
                _ => None,
            };
            if let Some(request) = request {
                self.local_command(&action, vec![request]);
                return;
            }
            if action == "repeat" {
                self.local_command("repeat", local_player::repeat_requests(&self.repeat_state));
                return;
            }
        }
        let Some(token) = self.token.clone() else {
            self.set_status("Connect Spotify to use this control with a real device");
            return;
        };
        // UI regression tests inject responses and never call Spotify.
        #[cfg(test)]
        {
            let _ = (token, work, action, ctx);
        }
        #[cfg(not(test))]
        {
            let tx = self.tx.clone();
            let client_id = self.config.client_id.clone();
            let ctx = ctx.clone();
            thread::spawn(move || {
                let (token, result) = match spotify::refresh_if_needed(token, &client_id) {
                    Ok(token) => {
                        let result = work(&token.access_token);
                        (Some(token), result)
                    }
                    Err(error) => (None, Err(error)),
                };
                let _ = tx.send(AppEvent::ActionFinished {
                    action,
                    result,
                    token,
                });
                ctx.request_repaint();
            });
        }
    }

    pub(crate) fn table_play_context(
        &self,
        track: &Track,
        index: usize,
    ) -> Option<spotify::playback::ContextStart> {
        let uri = match self.page {
            Page::Library => format!("spotify:playlist:{}", self.selected_playlist.as_ref()?.id),
            Page::Album => format!("spotify:album:{}", self.browse.album.as_ref()?.album.id),
            _ => return None,
        };
        Some(spotify::playback::ContextStart {
            uri,
            position: track.context_position.unwrap_or(index),
        })
    }

    pub(crate) fn play_table_track(&mut self, track: Track, index: usize, ctx: &egui::Context) {
        if let Some(start) = self.table_play_context(&track, index) {
            self.play_collection(start, Some(track), ctx);
        } else {
            self.play_track(track, ctx);
        }
    }

    pub(crate) fn is_playlist_actively_playing(&self, playlist_id: &str) -> bool {
        let uri = format!("spotify:playlist:{playlist_id}");
        if self.active_collection_uri.as_deref() == Some(uri.as_str()) && !self.playback_idle {
            return true;
        }
        if self.playback_context_uri.as_deref() != Some(uri.as_str())
            || self.playback_idle
            || self.suppress_upstream_queue
        {
            return false;
        }
        if !self.playlist_tracks.is_empty()
            && self
                .selected_playlist
                .as_ref()
                .is_some_and(|selected| selected.id == playlist_id)
        {
            if !self.current_track.uri.is_empty()
                && !self
                    .playlist_tracks
                    .iter()
                    .any(|track| track.uri == self.current_track.uri)
            {
                return false;
            }
            if let Some(next_up) = self.queue.first()
                && !next_up.uri.is_empty()
                && !self
                    .playlist_tracks
                    .iter()
                    .any(|track| track.uri == next_up.uri)
            {
                return false;
            }
        }
        true
    }

    pub(crate) fn start_local_collection_queue(
        &mut self,
        uri: String,
        tracks: Vec<Track>,
        start_position: usize,
        track: Option<Track>,
        ctx: &egui::Context,
    ) -> bool {
        let valid: Vec<Track> = tracks
            .into_iter()
            .filter(|t| spotify::validate_track_uri(&t.uri).is_ok())
            .collect();
        if valid.is_empty() {
            return false;
        }
        let start_idx = track
            .as_ref()
            .and_then(|target| {
                valid
                    .iter()
                    .position(|t| t.context_position == Some(start_position) && t.uri == target.uri)
                    .or_else(|| valid.iter().position(|t| t.uri == target.uri))
            })
            .unwrap_or_else(|| start_position.min(valid.len().saturating_sub(1)));
        let first = valid[start_idx].clone();
        let remaining: Vec<Track> = valid.into_iter().skip(start_idx + 1).take(1000).collect();
        self.suppress_upstream_queue = true;
        self.queue.clear();
        self.priority_queue = remaining;
        self.manual_queue_len = 0;
        self.priority_dispatched = Some((first.uri.clone(), Instant::now()));
        self.dispatch_track_play(first, ctx);
        self.active_collection_uri = Some(uri);
        self.set_status(if self.priority_queue.is_empty() {
            "Collection playback started".to_owned()
        } else {
            format!(
                "Collection playback started · {} up next",
                self.priority_queue.len()
            )
        });
        true
    }

    fn collection_tracks_for(&self, uri: &str) -> Option<Vec<Track>> {
        if self
            .selected_playlist
            .as_ref()
            .is_some_and(|p| format!("spotify:playlist:{}", p.id) == uri)
            && !self.playlist_tracks.is_empty()
        {
            let tracks: Vec<Track> = self
                .playlist_tracks
                .iter()
                .filter(|t| spotify::validate_track_uri(&t.uri).is_ok())
                .cloned()
                .collect();
            return (!tracks.is_empty()).then_some(tracks);
        }
        if let Some(album) = self
            .browse
            .album
            .as_ref()
            .filter(|a| format!("spotify:album:{}", a.album.id) == uri)
            && !album.tracks.is_empty()
        {
            let tracks: Vec<Track> = album
                .tracks
                .iter()
                .filter(|t| spotify::validate_track_uri(&t.uri).is_ok())
                .cloned()
                .collect();
            return (!tracks.is_empty()).then_some(tracks);
        }
        None
    }

    pub(crate) fn play_playlist(&mut self, playlist: &Playlist, ctx: &egui::Context) {
        if self.is_playlist_actively_playing(&playlist.id) {
            self.send_playback_action(if self.is_playing { "pause" } else { "resume" }, ctx);
        } else {
            self.play_collection(
                spotify::playback::ContextStart {
                    uri: format!("spotify:playlist:{}", playlist.id),
                    position: 0,
                },
                None,
                ctx,
            );
        }
    }

    pub(crate) fn play_collection(
        &mut self,
        start: spotify::playback::ContextStart,
        track: Option<Track>,
        ctx: &egui::Context,
    ) {
        if !self.is_connected() {
            self.set_status("Connect Spotify to play this collection.");
            return;
        }
        if self.local_starting && !self.follow_remote_device {
            self.set_status("Restoring the local player… Please wait a moment.");
            return;
        }
        self.collection_play_generation = self.collection_play_generation.wrapping_add(1);
        let generation = self.collection_play_generation;
        if self.use_local_for_play() || self.can_start_free_player() {
            if let Some(tracks) = self.collection_tracks_for(&start.uri)
                && self.start_local_collection_queue(
                    start.uri.clone(),
                    tracks,
                    start.position,
                    track.clone(),
                    ctx,
                )
            {
                return;
            }
            if let Some(token) = self.token.clone()
                && (start.uri.starts_with("spotify:playlist:")
                    || start.uri.starts_with("spotify:album:"))
            {
                let tx = self.tx.clone();
                let ctx_clone = ctx.clone();
                let client_id = self.config.client_id.clone();
                self.set_status("Loading collection tracks…");
                thread::spawn(move || {
                    let (token, result) = match spotify::refresh_if_needed(token, &client_id) {
                        Ok(token) => {
                            let res = if let Some(id) = start.uri.strip_prefix("spotify:playlist:")
                            {
                                spotify::get_playlist_items(&token.access_token, id)
                            } else if let Some(id) = start.uri.strip_prefix("spotify:album:") {
                                spotify::catalog::album_details(&token.access_token, id)
                                    .map(|details| details.tracks)
                            } else {
                                Err("Unsupported collection URI.".into())
                            };
                            (Some(token), res)
                        }
                        Err(error) => (None, Err(error)),
                    };
                    let _ = tx.send(AppEvent::CollectionPlayLoaded {
                        generation,
                        start,
                        track,
                        result,
                        token,
                    });
                    ctx_clone.request_repaint();
                });
                return;
            }
        }
        self.start_remote_collection_context(start, track, ctx);
    }

    pub(crate) fn start_remote_collection_context(
        &mut self,
        start: spotify::playback::ContextStart,
        track: Option<Track>,
        ctx: &egui::Context,
    ) {
        self.restoring_playback = false;
        self.pending_handoff = None;
        self.suppress_upstream_queue = false;
        self.active_collection_uri = None;
        self.priority_queue.clear();
        self.priority_dispatched = None;
        self.manual_queue_len = 0;
        self.requested_track_uri = track.map(|track| track.uri);
        let local = self.use_local_for_play();
        let device = self.device_id.clone();
        let local_name = self.local_device_name();
        if local {
            self.select_local_on_ready = false;
            if !self.local_status.active {
                self.local_command("activate_context", vec![local_player::request("activate")]);
            }
        }
        self.set_status("Starting collection playback…");
        self.start_spotify_action(
            "play_context".into(),
            move |access| {
                let target = if local {
                    Some(spotify::playback::local_device_id(access, &local_name)?)
                } else {
                    device
                };
                spotify::playback::start_context(access, &start, target.as_deref())
            },
            ctx,
        );
    }

    pub(crate) fn play_track(&mut self, track: Track, ctx: &egui::Context) {
        let manual_keep = self.manual_queue_len.min(self.priority_queue.len());
        self.priority_queue.truncate(manual_keep);
        // A standalone play starts a Spotify-owned queue. Keep explicit
        // Aurafy queue entries but drop the previous collection's local tail.
        self.suppress_upstream_queue = false;
        self.priority_dispatched = None;
        self.queue.clear();
        self.dispatch_track_play(track, ctx);
    }

    pub(crate) fn dispatch_track_play(&mut self, track: Track, ctx: &egui::Context) {
        self.restoring_playback = false;
        self.playback_context_uri = None;
        self.active_collection_uri = None;
        self.collection_play_generation = self.collection_play_generation.wrapping_add(1);
        self.playback_idle = false;
        if self.page == Page::Search {
            self.remember_search_track(&track);
        }
        self.pending_handoff = None;
        if self.local_starting && !self.follow_remote_device {
            self.set_status("Restoring the local player… Please wait a moment.");
            return;
        }
        if self.use_local_for_play() && !track.uri.is_empty() {
            self.select_local_on_ready = false;
            self.requested_track_uri = Some(track.uri.clone());
            let mut requests = Vec::new();
            if !self.local_status.active {
                requests.push(local_player::request("activate"));
            }
            requests.push(serde_json::json!({"type":"command","command":"play","uri":track.uri}));
            self.local_command("play_track", requests);
            return;
        }
        if self.can_start_free_player() && !track.uri.is_empty() {
            self.current_track = track.clone();
            if let Some(url) = &track.artwork_url {
                self.request_artwork(url, ctx);
            }
            self.progress = 0.0;
            self.progress_ms = 0;
            self.duration_ms = track.duration_ms.max(1);
            self.progress_anchor = Instant::now();
            self.is_playing = true;
            self.playback_error = None;
            self.set_status(format!("Playing {} (Free Audio Engine)…", track.name));
            let fp = self.ensure_free_player(ctx);
            fp.send(free_player::FreeCommand::Play(track));
            return;
        }
        self.current_track = track.clone();
        self.progress = 0.0;
        self.progress_ms = 0;
        self.duration_ms = track.duration_ms.max(1);
        self.progress_anchor = Instant::now();
        if self.is_connected() && !track.uri.is_empty() {
            let uri = track.uri.clone();
            self.set_status(format!("Sending {} to Spotify…", track.name));
            self.start_spotify_action(
                "play_track".into(),
                move |access| spotify::play_track(access, &uri),
                ctx,
            );
        } else if self.is_demo_mode() {
            self.is_playing = true;
            self.set_status(
                "Demo preview only · Connect Spotify and choose a Connect device for audio",
            );
        } else {
            self.is_playing = false;
            self.set_status("Your Spotify player is reconnecting. Wait a moment, then try again.");
        }
    }

    pub(crate) fn send_playback_action(&mut self, action: &str, ctx: &egui::Context) {
        if action == "pause" {
            self.priority_auto_armed = false;
        } else if action == "next" {
            if !self.priority_queue.is_empty() {
                let _ = self.play_next_priority_track(ctx);
                return;
            }
            if self.suppress_upstream_queue {
                self.set_status("Queue is empty");
                return;
            }
        }
        if self.local_starting && !self.follow_remote_device {
            self.set_status("Restoring the local player… Please wait a moment.");
            return;
        }
        if self.local_player.is_some() && !self.local_status.paired && self.device_name.is_none() {
            self.listen_here(ctx);
            return;
        }
        if self.use_free_player() {
            match action {
                "pause" => {
                    self.is_playing = false;
                    if let Some(fp) = &self.free_player {
                        fp.send(free_player::FreeCommand::Pause);
                    }
                }
                "resume" => {
                    self.is_playing = true;
                    if let Some(fp) = &self.free_player {
                        fp.send(free_player::FreeCommand::Resume);
                    }
                }
                "previous" => {
                    if self.progress_ms > 3000 {
                        self.progress_ms = 0;
                        self.progress = 0.0;
                        if let Some(fp) = &self.free_player {
                            fp.send(free_player::FreeCommand::Seek(0));
                        }
                    } else if let Some(prev) = self.recent_tracks.first().cloned() {
                        self.dispatch_track_play(prev, ctx);
                    }
                }
                _ => {}
            }
            return;
        }
        if self.use_local_playback() || (action == "resume" && self.use_local_for_play()) {
            self.select_local_on_ready = false;
            let command = match action {
                "resume" => "play",
                "pause" => "pause",
                "next" => {
                    self.last_manual_skip = Some(Instant::now());
                    "skip_next"
                }
                "previous" => {
                    self.last_manual_skip = Some(Instant::now());
                    "skip_prev"
                }
                _ => return,
            };
            let mut requests = Vec::new();
            if !self.local_status.active {
                requests.push(local_player::request("activate"));
            }
            requests.push(if action == "resume" {
                self.native_resume_request()
            } else {
                local_player::request(command)
            });
            self.local_command(action, requests);
            return;
        }
        if self.is_demo_mode() {
            let position = self.current_progress_ms();
            match action {
                "pause" => {
                    self.progress_ms = position;
                    self.progress = position as f32 / self.duration_ms.max(1) as f32;
                    self.is_playing = false;
                }
                "resume" => {
                    self.is_playing = true;
                }
                "next" => self.advance_demo_track(1),
                "previous" => self.advance_demo_track(-1),
                _ => {}
            }
            self.progress_anchor = Instant::now();
            self.set_status(
                "Demo preview only · Connect Spotify and choose a Connect device for audio",
            );
            return;
        }
        if !self.is_connected() {
            self.set_status("Your Spotify session is reconnecting. Please wait a moment.");
            return;
        }
        let action_owned = action.to_string();
        let work_action = action_owned.clone();
        self.set_status("Sending playback command to Spotify…");
        self.start_spotify_action(
            action_owned,
            move |access| spotify::playback_command(access, &work_action),
            ctx,
        );
    }

    pub(crate) fn advance_demo_track(&mut self, delta: isize) {
        if delta > 0 && !self.priority_queue.is_empty() {
            self.current_track = self.priority_queue.remove(0);
        } else if delta > 0 && !self.queue.is_empty() {
            self.current_track = self.queue.remove(0);
        } else {
            let source = if self.saved_tracks.is_empty() {
                &self.demo_tracks
            } else {
                &self.saved_tracks
            };
            if source.is_empty() {
                return;
            }
            let current = source
                .iter()
                .position(|item| item.id == self.current_track.id)
                .unwrap_or(0) as isize;
            let next = (current + delta).rem_euclid(source.len() as isize) as usize;
            self.current_track = source[next].clone();
        }
        self.duration_ms = self.current_track.duration_ms.max(1);
        self.progress_ms = 0;
        self.progress = 0.0;
        self.progress_anchor = Instant::now();
        self.is_playing = true;
    }

    pub(crate) fn queue_priority_tracks(&mut self, incoming: &[Track]) {
        let space = 1000usize.saturating_sub(self.priority_queue.len());
        let added: Vec<Track> = incoming.iter().take(space).cloned().collect();
        if added.is_empty() {
            return;
        }
        let insert_at = self.manual_queue_len.min(self.priority_queue.len());
        self.manual_queue_len += added.len();
        self.priority_queue.splice(insert_at..insert_at, added);
    }

    pub(crate) fn clear_up_next_queue(&mut self) -> String {
        let cleared_priority = self.priority_queue.len();
        self.priority_queue.clear();
        self.manual_queue_len = 0;
        self.priority_dispatched = None;
        self.active_collection_uri = None;
        let cleared_upstream = self.queue.len();
        self.queue.clear();
        if !self.is_demo_mode() {
            self.suppress_upstream_queue = true;
        }
        let total = cleared_priority + cleared_upstream;
        let message = if total == 0 {
            "Your queue is already empty.".to_owned()
        } else {
            format!("Cleared {total} tracks from your queue.")
        };
        self.set_status(message.clone());
        message
    }

    pub(crate) fn play_next_priority_track(&mut self, ctx: &egui::Context) -> bool {
        if self.priority_queue.is_empty() {
            return false;
        }
        let next = self.priority_queue.remove(0);
        self.manual_queue_len = self.manual_queue_len.saturating_sub(1);
        self.priority_dispatched = Some((next.uri.clone(), Instant::now()));
        let collection = self.active_collection_uri.take();
        self.dispatch_track_play(next, ctx);
        self.active_collection_uri = collection;
        true
    }

    pub(crate) fn handle_suppressed_queue_end(
        &mut self,
        blocked_uri: Option<String>,
        ctx: &egui::Context,
    ) {
        if self.repeat_state == "context"
            && let Some(uri) = self.active_collection_uri.clone()
            && let Some(tracks) = self.collection_tracks_for(&uri)
            && self.start_local_collection_queue(uri, tracks, 0, None, ctx)
        {
            return;
        }
        self.active_collection_uri = None;
        self.priority_auto_armed = false;
        self.is_playing = false;
        self.priority_dispatched = Some((
            blocked_uri.unwrap_or_else(|| self.current_track.uri.clone()),
            Instant::now(),
        ));
        self.send_playback_action("pause", ctx);
        self.set_status("Reached end of queue");
    }

    pub(crate) fn append_queue_preview(&mut self, incoming: &[Track]) {
        let space = 50usize.saturating_sub(self.queue.len());
        self.queue.extend(incoming.iter().take(space).cloned());
    }

    pub(crate) fn queue_local_tracks(&mut self, tracks: &[Track]) {
        self.append_queue_preview(tracks);
        let mut requests = Vec::with_capacity(tracks.len() + 1);
        if !self.local_status.active && !self.local_starting && !self.follow_remote_device {
            requests.push(serde_json::json!({"type":"command","command":"activate"}));
        }
        requests.extend(
            tracks.iter().filter(|t| !t.uri.is_empty()).map(
                |t| serde_json::json!({"type":"command","command":"add_to_queue","uri":t.uri}),
            ),
        );
        self.local_command("queue", requests);
    }

    pub(crate) fn queue_spotify_tracks(
        &mut self,
        action: String,
        tracks: Vec<Track>,
        max_preview: usize,
        ctx: &egui::Context,
    ) {
        let preview_len = tracks.len().min(max_preview);
        self.append_queue_preview(&tracks[..preview_len]);
        let count = tracks.len();
        self.set_status(if count == 1 {
            format!("Adding {} to Spotify queue…", tracks[0].name)
        } else {
            format!("Queueing {count} tracks on Spotify…")
        });
        let uris: Vec<_> = tracks.into_iter().map(|t| t.uri).collect();
        self.start_spotify_action(
            action,
            move |access| {
                let total = uris.len();
                for (idx, uri) in uris.iter().enumerate() {
                    if idx > 0 {
                        thread::sleep(Duration::from_millis(80));
                    }
                    if let Err(error) = spotify::add_to_queue(access, uri) {
                        if idx == 0 {
                            return Err(error);
                        }
                        return Err(format!("Queued {idx} of {total} tracks. {error}"));
                    }
                }
                Ok(())
            },
            ctx,
        );
    }

    pub(crate) fn add_to_queue(&mut self, track: Track, ctx: &egui::Context) {
        let has_existing = !self.queue.is_empty()
            || !self.priority_queue.is_empty()
            || self.suppress_upstream_queue;
        if has_existing
            && !track.uri.is_empty()
            && (self.use_local_playback() || self.is_connected() || self.is_demo_mode())
        {
            self.queue_priority_tracks(std::slice::from_ref(&track));
            self.set_status(format!("Added {} to play next", track.name));
            return;
        }
        if self.use_local_playback() && !track.uri.is_empty() {
            self.set_status(format!("Adding {} to queue…", track.name));
            self.queue_local_tracks(std::slice::from_ref(&track));
            return;
        }
        if self.is_connected() && !track.uri.is_empty() {
            let track_id = track.id.clone();
            self.queue_spotify_tracks(format!("queue:{track_id}"), vec![track], 1, ctx);
        } else {
            self.queue.push(track.clone());
            self.set_status(format!("Added {} to the demo queue", track.name));
        }
    }

    pub(crate) fn queue_tracks_batch(&mut self, tracks: Vec<Track>, ctx: &egui::Context) {
        let valid: Vec<_> = tracks
            .into_iter()
            .filter(|t| spotify::validate_track_uri(&t.uri).is_ok())
            .collect();
        if valid.is_empty() {
            if !self.is_connected() {
                self.set_status("Connect Spotify to queue tracks");
            }
            return;
        }
        let has_existing = !self.queue.is_empty()
            || !self.priority_queue.is_empty()
            || self.suppress_upstream_queue;
        if has_existing && (self.use_local_playback() || self.is_connected() || self.is_demo_mode())
        {
            let before = self.priority_queue.len();
            self.queue_priority_tracks(&valid);
            let added = self.priority_queue.len().saturating_sub(before);
            self.set_status(format!("Added {added} tracks to Up next"));
            return;
        }
        let valid: Vec<_> = valid.into_iter().take(25).collect();
        let count = valid.len();
        if self.use_local_playback() {
            self.set_status(format!("Adding {count} tracks to queue…"));
            self.queue_local_tracks(&valid);
            return;
        }
        if self.is_connected() {
            self.queue_spotify_tracks("queue:batch".into(), valid, 25, ctx);
        } else {
            self.queue.extend(valid);
            self.set_status(format!("Added {count} tracks to the demo queue"));
        }
    }

    pub(crate) fn play_tracks_shuffled(&mut self, tracks: Vec<Track>, ctx: &egui::Context) {
        use rand::seq::SliceRandom;
        let mut shuffled: Vec<Track> = if self.is_demo_mode() {
            tracks
                .into_iter()
                .filter(|t| !t.uri.is_empty() || !t.name.is_empty())
                .collect()
        } else {
            tracks
                .into_iter()
                .filter(|t| spotify::validate_track_uri(&t.uri).is_ok())
                .collect()
        };
        if shuffled.is_empty() {
            self.set_status("No playable tracks found to shuffle.");
            return;
        }
        shuffled.shuffle(&mut rand::thread_rng());
        let use_priority_shuffle =
            self.use_local_for_play() && (!self.queue.is_empty() || self.suppress_upstream_queue);
        let first = shuffled.remove(0);
        if use_priority_shuffle && !first.uri.is_empty() {
            if self.local_starting && !self.follow_remote_device {
                self.set_status("Restoring the local player… Please wait a moment.");
                return;
            }
            self.playback_context_uri = None;
            self.suppress_upstream_queue = true;
            self.queue.clear();
            self.priority_queue = shuffled.into_iter().take(1000).collect();
            self.manual_queue_len = 0;
            self.priority_dispatched = Some((first.uri.clone(), Instant::now()));
            self.dispatch_track_play(first, ctx);
            self.set_status(if self.priority_queue.is_empty() {
                "Shuffling track".to_owned()
            } else {
                format!("Shuffling · {} up next", self.priority_queue.len())
            });
            return;
        }
        self.suppress_upstream_queue = false;
        self.priority_queue.clear();
        self.priority_dispatched = None;
        let remaining: Vec<Track> = shuffled.into_iter().take(24).collect();
        if self.use_local_for_play() && !first.uri.is_empty() {
            if self.local_starting && !self.follow_remote_device {
                self.set_status("Restoring the local player… Please wait a moment.");
                return;
            }
            self.playback_context_uri = None;
            self.dispatch_track_play(first, ctx);
            if !remaining.is_empty() {
                self.queue_tracks_batch(remaining, ctx);
            }
            return;
        }
        if self.is_connected() && !first.uri.is_empty() {
            self.restoring_playback = false;
            self.pending_handoff = None;
            self.playback_context_uri = None;
            self.current_track = first.clone();
            self.progress = 0.0;
            self.progress_ms = 0;
            self.duration_ms = first.duration_ms.max(1);
            self.progress_anchor = Instant::now();
            self.is_playing = true;
            self.queue = remaining.clone();
            if let Some(url) = &first.artwork_url {
                self.request_artwork(url, ctx);
            }
            let first_uri = first.uri.clone();
            let queue_uris: Vec<String> = remaining
                .into_iter()
                .filter(|t| spotify::validate_track_uri(&t.uri).is_ok())
                .map(|t| t.uri)
                .collect();
            let total = queue_uris.len() + 1;
            self.set_status(format!("Shuffling {total} tracks on Spotify…"));
            self.start_spotify_action(
                "play_shuffled".into(),
                move |access| {
                    spotify::play_track(access, &first_uri)?;
                    if !queue_uris.is_empty() {
                        thread::sleep(Duration::from_millis(250));
                        for (idx, uri) in queue_uris.iter().enumerate() {
                            if idx > 0 {
                                thread::sleep(Duration::from_millis(80));
                            }
                            spotify::add_to_queue(access, uri)?;
                        }
                    }
                    Ok(())
                },
                ctx,
            );
            return;
        }
        self.play_track(first, ctx);
        if self.is_demo_mode() {
            let count = remaining.len() + 1;
            self.queue = remaining;
            self.set_status(format!("Shuffling {count} tracks"));
        }
    }
}
