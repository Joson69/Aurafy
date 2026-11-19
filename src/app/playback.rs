use super::*;

impl AurafyApp {
    pub(crate) fn load_devices(&mut self, ctx: &egui::Context) {
        let Some(token) = self.token.clone() else {
            self.set_status("Connect Spotify before loading playback devices");
            return;
        };
        if self.devices_loading {
            return;
        }
        self.devices_loading = true;
        self.last_devices_poll = Instant::now();
        let client_id = self.config.client_id.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let (token, devices) = match spotify::refresh_if_needed(token, &client_id) {
                Ok(token) => {
                    let devices = spotify::get_devices(&token.access_token);
                    (Some(token), devices)
                }
                Err(error) => (None, Err(error)),
            };
            let _ = tx.send(AppEvent::DevicesLoaded { devices, token });
            ctx.request_repaint();
        });
    }

    pub(crate) fn local_device_name(&self) -> String {
        self.config.effective_device_name()
    }

    pub(crate) fn local_device_name_ref(&self) -> &str {
        let trimmed = self.config.connect_device_name.trim();
        if trimmed.is_empty() {
            local_player::DEVICE_NAME
        } else {
            trimmed
        }
    }

    pub(crate) fn playback_devices(&self) -> Vec<Device> {
        let mut devices = self.devices.clone();
        if self.local_status.paired && self.local_player.is_some() {
            let local_name = self.local_device_name();
            devices.retain(|device| device.name != local_name);
            if self.use_local_playback() {
                for device in &mut devices {
                    device.is_active = false;
                }
            }
            devices.insert(
                0,
                Device {
                    id: "auralis-local".into(),
                    name: local_name,
                    device_type: "This computer".into(),
                    is_active: self.use_local_playback(),
                    volume_percent: Some((self.volume * 100.0).round() as u8),
                },
            );
        }
        devices
    }

    pub(crate) fn should_refresh_devices(&self) -> bool {
        self.device_picker_open
            && self.is_connected()
            && !self.local_status.active
            && !self.devices_loading
            && self.last_devices_poll.elapsed() >= Duration::from_secs(30)
    }

    pub(crate) fn load_insights(&mut self, ctx: &egui::Context) {
        if self.insights.loading
            || self.insights.permission_needed
            || self.insights.cache.contains_key(&self.insights.range)
        {
            return;
        }
        let Some(token) = self.token.clone() else {
            return;
        };
        self.insights.loading = true;
        self.insights.attempted = true;
        self.insights.error = None;
        self.insights.generation = self.insights.generation.wrapping_add(1);
        let generation = self.insights.generation;
        let range = self.insights.range;
        #[cfg(not(test))]
        {
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            let client_id = self.config.client_id.clone();
            thread::spawn(move || {
                let (token, result) = match spotify::refresh_if_needed(token, &client_id) {
                    Ok(token) => {
                        let result = spotify::insights::fetch(&token.access_token, range);
                        (Some(token), result)
                    }
                    Err(error) => (None, Err(spotify::insights::Error::Other(error))),
                };
                let _ = tx.send(AppEvent::InsightsLoaded {
                    generation,
                    range,
                    result,
                    token,
                });
                ctx.request_repaint();
            });
        }
        #[cfg(test)]
        let _ = (token, generation, range, ctx);
    }

    pub(crate) fn load_home_data(&mut self, ctx: &egui::Context) {
        let Some(token) = self.token.clone() else {
            return;
        };
        if self.recent_loading
            || self
                .last_home_load
                .is_some_and(|last| last.elapsed() < Duration::from_secs(30))
        {
            return;
        }
        self.recent_loading = true;
        self.recent_error = None;
        self.last_home_load = Some(Instant::now());
        let client_id = self.config.client_id.clone();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let (token, recent_tracks, profile) =
                match spotify::refresh_if_needed(token, &client_id) {
                    Ok(token) => {
                        let recent_tracks = spotify::get_recently_played(&token.access_token);
                        let profile = spotify::get_profile(&token.access_token);
                        (Some(token), recent_tracks, profile)
                    }
                    Err(error) => (None, Err(error.clone()), Err(error)),
                };
            let _ = tx.send(AppEvent::HomeDataLoaded {
                recent_tracks,
                profile,
                token,
            });
            ctx.request_repaint();
        });
    }

    pub(crate) fn transfer_to_device(&mut self, device: Device, ctx: &egui::Context) {
        self.volume_control = volume::Control::default();
        if device.name == self.local_device_name_ref() && self.local_status.paired {
            self.listen_here(ctx);
            return;
        }
        let id = device.id.clone();
        let name = device.name.clone();
        self.start_spotify_action(
            format!("device:{name}"),
            move |access| spotify::transfer_playback(access, &id),
            ctx,
        );
        self.set_status(format!("Transferring playback to {name}…"));
    }

    pub(crate) fn enable_local_playback(&mut self, ctx: &egui::Context) {
        self.start_local_playback(ctx, false);
    }

    pub(crate) fn start_local_playback(&mut self, ctx: &egui::Context, reset_pairing: bool) {
        if self.local_starting {
            return;
        }
        let key = if self.local_key_input.trim().is_empty() {
            match local_player::load_key() {
                Ok(Some(key)) => key,
                Ok(None) => {
                    self.playback_error = Some("Paste your personal Spotify Soloist API key below to enable playback on this computer.".into());
                    return;
                }
                Err(error) => {
                    self.playback_error = Some(error);
                    return;
                }
            }
        } else {
            let key = self.local_key_input.trim().to_owned();
            if let Err(error) = local_player::save_key(&key) {
                self.playback_error = Some(error);
                return;
            }
            self.local_key_input.clear();
            key
        };
        self.local_player_generation = self.local_player_generation.wrapping_add(1);
        self.local_player.take();
        self.select_local_on_ready = true;
        self.local_starting = true;
        self.local_status = local_player::Status {
            message: "Starting local playback…".into(),
            ..Default::default()
        };
        self.playback_error = None;
        self.config.local_playback = true;
        if let Err(error) = self.config.save() {
            self.set_status(error);
        }
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let generation = self.local_player_generation;
        let last_playback = std::cell::RefCell::new(None::<PlaybackState>);
        #[cfg(target_os = "windows")]
        let wake_window = self.window_geometry;
        self.local_player = Some(local_player::Player::start(
            key,
            reset_pairing,
            local_player::Options {
                initial_volume: self
                    .config
                    .startup_volume
                    .or(self.config.last_local_volume)
                    .unwrap_or(40)
                    .min(100),
                cache_mb: self.config.cache_mb(),
                audio: self.config.audio,
                device_name: self.local_device_name(),
            },
            move |event| {
                let position_update = if let local_player::Event::Playback(next) = &event {
                    let mut previous = last_playback.borrow_mut();
                    let passive = previous
                        .as_ref()
                        .is_some_and(|old| local_player::position_only(old, next));
                    *previous = Some(next.clone());
                    passive
                } else {
                    false
                };
                let _ = tx.send(AppEvent::LocalPlayer { generation, event });
                if position_update {
                    ctx.request_repaint_after(Duration::from_secs(1));
                } else {
                    #[cfg(target_os = "windows")]
                    if let Some(window) = wake_window {
                        window.wake_repaint(&ctx);
                        return;
                    }
                    ctx.request_repaint();
                }
            },
        ));
    }

    pub(crate) fn stop_local_playback(&mut self) {
        self.volume_control = volume::Control::default();
        self.select_local_on_ready = false;
        self.local_player_generation = self.local_player_generation.wrapping_add(1);
        self.local_player.take();
        if self.local_status.active {
            self.is_playing = false;
            self.device_name = None;
            self.device_id = None;
        }
        self.local_status = local_player::Status::default();
        self.local_starting = false;
        self.config.local_playback = false;
        let _ = self.config.save();
    }

    pub(crate) fn restart_local_pairing(&mut self, ctx: &egui::Context) {
        self.local_player_generation = self.local_player_generation.wrapping_add(1);
        self.local_player.take();
        self.local_starting = false;
        self.local_status = local_player::Status::default();
        self.start_local_playback(ctx, true);
    }

    pub(crate) fn reconnect_local_player(&mut self, ctx: &egui::Context) {
        self.local_player_generation = self.local_player_generation.wrapping_add(1);
        self.local_player.take();
        self.local_starting = false;
        self.local_status = local_player::Status::default();
        self.enable_local_playback(ctx);
    }

    pub(crate) fn use_local_playback(&self) -> bool {
        self.local_player.is_some()
            && self.local_status.paired
            && (self.local_status.active || !self.follow_remote_device)
    }

    pub(crate) fn use_local_for_play(&self) -> bool {
        self.use_local_playback()
    }

    pub(crate) fn selected_output_name(&self) -> Option<&str> {
        if self.use_local_playback() {
            Some(self.local_device_name_ref())
        } else {
            self.device_name.as_deref()
        }
    }

    pub(crate) fn local_command(&mut self, action: &str, requests: Vec<serde_json::Value>) {
        let result = self
            .local_player
            .as_ref()
            .ok_or_else(|| "Enable local playback in Settings → Playback first.".to_string())
            .and_then(|player| player.command(action, requests));
        if let Err(error) = result {
            if action == "volume" {
                self.volume_control.complete(false);
            }
            self.playback_error = Some(error.clone());
            self.set_status(error);
        }
    }

    pub(crate) fn native_resume_request(&mut self) -> serde_json::Value {
        if self.playback_idle && !self.current_track.uri.is_empty() {
            // An exhausted native context cannot resume. Explicitly replay the
            // displayed track, without restoring its previous end position.
            self.restoring_playback = false;
            self.pending_handoff = None;
            self.requested_track_uri = Some(self.current_track.uri.clone());
            serde_json::json!({"type":"command","command":"play","uri":self.current_track.uri})
        } else {
            self.capture_handoff();
            local_player::request("play")
        }
    }

    pub(crate) fn listen_here(&mut self, ctx: &egui::Context) {
        self.volume_control = volume::Control::default();
        if !self.local_status.paired {
            self.page = Page::Settings;
            self.settings_tab = SettingsTab::Account;
            self.set_status("Enable local playback, then choose Aurafy once in Spotify's device picker to pair this computer.");
            if self.local_player.is_none() && self.config.local_playback {
                self.enable_local_playback(ctx);
            }
            return;
        }
        self.select_local_on_ready = false;
        if self.local_status.active {
            let play = self.native_resume_request();
            self.local_command("resume", vec![play]);
            return;
        }
        let play = self.native_resume_request();
        self.follow_remote_device = false;
        // A restored receiver can activate itself before it appears in Spotify's
        // Web API device list. Keep the current context; never start a new URI.
        self.local_command("listen_here", vec![local_player::request("activate"), play]);
        self.set_status("Moving playback to Aurafy…");
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn handle_local_event(&mut self, event: local_player::Event) {
        self.handle_local_event_with_ctx(event, None);
    }

    pub(crate) fn handle_local_event_with_ctx(
        &mut self,
        event: local_player::Event,
        ctx: Option<&egui::Context>,
    ) {
        match event {
            local_player::Event::Status(status) => {
                self.local_starting = status.connecting;
                if status.message.starts_with("Finish pairing:") {
                    self.set_status(status.message.clone());
                }
                if !self.local_status.active
                    && status.active
                    && self.is_playing
                    && self.device_name.is_some()
                    && self.requested_track_uri.is_none()
                    && self.pending_handoff.is_none()
                {
                    self.capture_handoff();
                }
                if self.local_status.active && !status.active {
                    self.queue.clear();
                    self.volume_control = volume::Control::default();
                    // A paused receiver may release its active connection. Keep
                    // the selected output; only confirmed remote playback or
                    // an explicit device choice selects another output.
                    self.is_playing = false;
                    self.device_name = None;
                    self.device_id = None;
                    self.pending_handoff = None;
                    self.last_player_poll = Instant::now() - Duration::from_secs(10);
                    self.last_devices_poll = Instant::now() - Duration::from_secs(10);
                }
                if !status.running && !status.message.is_empty() {
                    self.playback_error = Some(status.message.clone());
                }
                if status.active {
                    self.device_name = Some(self.local_device_name());
                    self.follow_remote_device = false;
                } else if status.paired && self.current_track.uri.is_empty() {
                    self.current_track = Track {
                        name: "Nothing playing yet".into(),
                        artist: "Choose a song, or press Play to resume".into(),
                        ..Default::default()
                    };
                }
                self.local_status = status;
                if self.select_local_on_ready && self.local_status.paired {
                    self.select_local_on_ready = false;
                    let mut requests = Vec::new();
                    if !self.local_status.active {
                        requests.push(local_player::request("activate"));
                    }
                    if !self.config.pause_on_start {
                        requests.push(local_player::request("play"));
                    }
                    if !requests.is_empty() {
                        self.capture_handoff();
                        self.local_command("select_local", requests);
                    }
                }
                self.publish_runtime_status();
            }
            local_player::Event::Playback(state) => {
                if self.local_status.active && self.use_local_for_play() {
                    let incoming_uri = state
                        .track
                        .as_ref()
                        .map(|t| t.uri.as_str())
                        .filter(|s| !s.is_empty());
                    if let Some(req) = self.requested_track_uri.as_deref()
                        && incoming_uri != Some(req)
                        && self.priority_dispatched.as_ref().is_some_and(|(d, at)| {
                            d == req && at.elapsed() < Duration::from_secs(5)
                        })
                    {
                        return;
                    }
                    if !state.is_playing
                        && self.suppress_upstream_queue
                        && self.priority_queue.is_empty()
                        && let Some(uri) = incoming_uri
                        && self.priority_dispatched.as_ref().is_some_and(|(d, at)| {
                            d == uri && at.elapsed() < Duration::from_secs(6)
                        })
                    {
                        self.is_playing = false;
                        return;
                    }
                    let active_playback =
                        state.is_playing || (self.priority_auto_armed && !state.is_idle);
                    let same_or_blank_context = state.context_uri.is_none()
                        || state.context_uri == self.playback_context_uri
                        || state.context_uri == self.active_collection_uri;
                    let near_track_end = self.duration_ms > 5000
                        && self.current_progress_ms() >= self.duration_ms.saturating_sub(15_000);
                    if active_playback
                        && self.repeat_state != "track"
                        && !self.restoring_playback
                        && self.requested_track_uri.is_none()
                        && !self.current_track.uri.is_empty()
                        && let Some(uri) = incoming_uri
                        && uri != self.current_track.uri
                        && state.progress_ms < 10_000
                        && !self.priority_dispatched.as_ref().is_some_and(|(d, at)| {
                            d == uri && at.elapsed() < Duration::from_secs(6)
                        })
                        && !self
                            .last_manual_skip
                            .is_some_and(|at| at.elapsed() < Duration::from_secs(6))
                        && (same_or_blank_context || near_track_end)
                        && (!self.priority_queue.is_empty() || self.suppress_upstream_queue)
                    {
                        if let Some(ctx) = ctx {
                            if !self.priority_queue.is_empty() {
                                if self.play_next_priority_track(ctx) {
                                    return;
                                }
                            } else if self.suppress_upstream_queue {
                                self.handle_suppressed_queue_end(Some(uri.to_owned()), ctx);
                                return;
                            }
                        }
                    }
                }
                if state.is_playing {
                    self.priority_auto_armed = true;
                }
                if self.use_local_for_play() {
                    self.playback_idle = state.is_idle;
                    self.playback_context_uri = state.context_uri.clone();
                    if let Some(uri) = state.context_uri.as_deref()
                        && self.active_collection_uri.as_deref() != Some(uri)
                    {
                        self.active_collection_uri = None;
                    }
                }
                if self.local_status.active {
                    if state.is_playing
                        && state
                            .track
                            .as_ref()
                            .is_some_and(|track| track.duration_ms > 0)
                        && let Some(pending) = self.pending_handoff.take()
                        && let Some(position) = pending.correction(&state)
                    {
                        self.local_command("restore_position",vec![serde_json::json!({"type":"command","command":"seek","position_ms":position})]);
                    }
                    if state
                        .track
                        .as_ref()
                        .is_some_and(|track| Some(&track.uri) == self.requested_track_uri.as_ref())
                    {
                        self.requested_track_uri = None;
                    }
                    self.apply_playback_state(state);
                } else if self.device_name.is_none()
                    && let Some(track) = state.track
                    && !track.uri.is_empty()
                {
                    // Restore the actual paused/native track without claiming it
                    // is active or replacing a session playing on another device.
                    let position = self
                        .config
                        .last_playback
                        .as_ref()
                        .filter(|saved| {
                            saved.track.uri == track.uri
                                && state.progress_ms < 5000
                                && saved.position_ms > state.progress_ms.saturating_add(2500)
                        })
                        .map(|saved| saved.position_ms)
                        .unwrap_or(state.progress_ms);
                    self.duration_ms = track.duration_ms.max(1);
                    self.current_track = track;
                    self.progress_ms = position.min(self.duration_ms);
                    self.progress = self.progress_ms as f32 / self.duration_ms as f32;
                    self.progress_anchor = Instant::now();
                }
            }
            local_player::Event::Queue(queue) => {
                if self.local_status.active && !self.suppress_upstream_queue {
                    self.queue = queue;
                }
            }
            local_player::Event::CommandFinished { action, result } => {
                if action == "volume" {
                    self.volume_control.complete(result.is_ok());
                }
                match result {
                    Ok(()) => {
                        self.playback_error = None;
                        match action.as_str() {
                            "listen_here" => {
                                self.set_status("Playback transfer requested · Waiting for Aurafy");
                            }
                            "queue" => self.set_status("Added to queue"),
                            _ => {}
                        }
                    }
                    Err(error) => {
                        self.pending_handoff = None;
                        self.requested_track_uri = None;
                        self.playback_error = Some(error.clone());
                        self.set_status(error);
                    }
                }
            }
        }
    }

    pub(crate) fn apply_playback_state(&mut self, mut state: PlaybackState) {
        self.playback_idle = state.is_idle;
        self.playback_context_uri = state.context_uri.clone();
        // The Web API can report a zero-position paused snapshot while the
        // native receiver reconnects. Keep the bookmark until actual playback
        // resumes; an explicit seek or song selection cancels restoration.
        if self.restoring_playback
            && !state.is_playing
            && state.progress_ms < 5000
            && state
                .track
                .as_ref()
                .is_some_and(|track| track.uri == self.current_track.uri)
        {
            let saved_position = self.current_progress_ms();
            if saved_position > state.progress_ms.saturating_add(2500) {
                state.progress_ms = saved_position;
            }
        }
        if state.is_playing {
            self.restoring_playback = false;
        }
        self.is_playing = state.is_playing;
        if let Some(track) = state.track
            && (!track.id.is_empty() || !track.uri.is_empty())
        {
            self.duration_ms = track.duration_ms.max(1);
            self.current_track = track;
        }
        if !self.seek_interacting {
            self.progress_ms = state.progress_ms.min(self.duration_ms);
            self.progress =
                (self.progress_ms as f32 / self.duration_ms.max(1) as f32).clamp(0.0, 1.0);
            self.progress_anchor = Instant::now();
        }
        if self.device_name.is_some()
            && state.device_name.is_some()
            && self.device_name != state.device_name
        {
            self.volume_control = volume::Control::default();
        }
        self.device_id = state.device_id;
        self.device_name = state.device_name;
        for device in &mut self.devices {
            device.is_active = if let Some(id) = &self.device_id {
                device.id == *id
            } else {
                self.device_name.as_deref() == Some(device.name.as_str())
            };
        }
        self.shuffle_enabled = state.shuffle_state;
        self.repeat_state = state.repeat_state;
        if let Some(volume) = state.volume_percent
            && self.volume_control.accepts_snapshot(volume, Instant::now())
        {
            self.volume = volume as f32 / 100.0;
        }
        if self.is_playing {
            self.playback_error = None;
        }
        if let Some(bookmark) = self.playback_bookmark() {
            self.config.last_playback = Some(bookmark);
        }
    }

    pub(crate) fn handle_ad_block(&mut self, is_ad: bool, ctx: &egui::Context) {
        if self.config.ad_blocker && is_ad {
            if self.pre_ad_volume.is_none() {
                let current_vol = (self.volume * 100.0).round().clamp(0.0, 100.0) as u8;
                self.pre_ad_volume = Some(current_vol.max(1));
                self.volume = 0.0;
                self.queue_volume(true, ctx);
            }
            if self
                .last_ad_skip
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(2))
            {
                self.last_ad_skip = Some(Instant::now());
                self.set_status("Ad detected · Muting and skipping…");
                self.send_playback_action("next", ctx);
            }
        } else if !is_ad && let Some(prev_vol) = self.pre_ad_volume.take() {
            self.last_ad_skip = None;
            self.volume = f32::from(prev_vol) / 100.0;
            self.queue_volume(true, ctx);
        }
    }

    pub(crate) fn use_free_player(&self) -> bool {
        !self.is_demo_mode()
            && self.local_player.is_none()
            && !self.local_starting
            && !self.use_local_for_play()
            && self.device_id.is_none()
            && self.free_player.as_ref().is_some_and(|fp| fp.active)
    }

    pub(crate) fn can_start_free_player(&self) -> bool {
        !self.is_demo_mode()
            && self.local_player.is_none()
            && !self.local_starting
            && !self.use_local_for_play()
            && self.device_id.is_none()
    }

    pub(crate) fn ensure_free_player(
        &mut self,
        ctx: &egui::Context,
    ) -> &mut free_player::FreePlayer {
        let player = self.free_player.get_or_insert_with(|| {
            let fp = free_player::FreePlayer::start(ctx.clone());
            fp.send(free_player::FreeCommand::SetVolume(f64::from(self.volume)));
            fp
        });
        player.active = true;
        player
    }

    pub(crate) fn poll_free_player(&mut self, ctx: &egui::Context) {
        let Some(fp) = self.free_player.as_mut() else {
            return;
        };
        let events = fp.poll();
        let mut ended = None;
        for event in events {
            match event {
                free_player::FreeEvent::Status(msg) => {
                    self.set_status(msg);
                }
                free_player::FreeEvent::State {
                    uri,
                    is_playing,
                    position_ms,
                    duration_ms,
                } => {
                    if self.current_track.uri == uri {
                        self.is_playing = is_playing;
                        if !self.seek_interacting {
                            self.duration_ms = duration_ms.max(1);
                            self.progress_ms = position_ms.min(self.duration_ms);
                            self.progress = (self.progress_ms as f32
                                / self.duration_ms.max(1) as f32)
                                .clamp(0.0, 1.0);
                            self.progress_anchor = Instant::now();
                        }
                        self.playback_error = None;
                    }
                }
                free_player::FreeEvent::Ended(uri) => {
                    ended = Some(uri);
                }
                free_player::FreeEvent::Error(err) => {
                    self.is_playing = false;
                    self.playback_error = Some(err.clone());
                    self.set_status(err);
                }
            }
        }
        if let Some(ended_uri) = ended {
            if self.repeat_state == "track" && !self.current_track.uri.is_empty() {
                self.dispatch_track_play(self.current_track.clone(), ctx);
            } else if !self.priority_queue.is_empty() {
                let _ = self.play_next_priority_track(ctx);
            } else {
                self.handle_suppressed_queue_end(Some(ended_uri), ctx);
            }
        }
    }

    pub(crate) fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
        self.status_until = Instant::now() + Duration::from_secs(2);
    }
}
