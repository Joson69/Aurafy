use super::*;

impl AurafyApp {
    pub(crate) fn handle_desktop_action(&mut self, action: desktop::Action, ctx: &egui::Context) {
        match action {
            desktop::Action::Open => {
                self.window_state.open(ctx);
                self.publish_runtime_status();
            }
            desktop::Action::Play => {
                if !self.is_playing {
                    self.send_playback_action("resume", ctx);
                }
            }
            desktop::Action::Pause => {
                if self.is_playing {
                    self.send_playback_action("pause", ctx);
                }
            }
            desktop::Action::TogglePlayback => {
                self.send_playback_action(if self.is_playing { "pause" } else { "resume" }, ctx)
            }
            desktop::Action::Previous => self.send_playback_action("previous", ctx),
            desktop::Action::Next => self.send_playback_action("next", ctx),
            desktop::Action::Quit => {
                self.save_session();
                self.window_state.quit(ctx);
            }
        }
    }

    pub(crate) fn handle_events(&mut self, ctx: &egui::Context) {
        self.poll_free_player(ctx);
        self.poll_playlist_editor(ctx);
        while let Ok(event) = self.rx.try_recv().or_else(|_| self.spotify_rx.try_recv()) {
            // Responses already in flight must not reconnect a signed-out account.
            if self.token.is_none()
                && matches!(
                    &event,
                    AppEvent::SearchFinished { token: Some(_), .. }
                        | AppEvent::SearchCatalogFinished { token: Some(_), .. }
                        | AppEvent::ArtistLoaded { token: Some(_), .. }
                        | AppEvent::ArtistSongsLoaded { token: Some(_), .. }
                        | AppEvent::ArtistAlbumsLoaded { token: Some(_), .. }
                        | AppEvent::AlbumLoaded { token: Some(_), .. }
                        | AppEvent::ActionFinished { token: Some(_), .. }
                        | AppEvent::PlayerSynced { token: Some(_), .. }
                        | AppEvent::LibraryLoaded { token: Some(_), .. }
                        | AppEvent::PlaylistLoaded { token: Some(_), .. }
                        | AppEvent::CollectionPlayLoaded { token: Some(_), .. }
                        | AppEvent::PlaylistCreated { token: Some(_), .. }
                        | AppEvent::DevicesLoaded { token: Some(_), .. }
                        | AppEvent::HomeDataLoaded { token: Some(_), .. }
                        | AppEvent::InsightsLoaded { token: Some(_), .. }
                        | AppEvent::AutoplayLoaded { token: Some(_), .. }
                )
            {
                continue;
            }
            match event {
                AppEvent::MembershipLoaded {
                    generation,
                    playlist_id,
                    uris,
                    token,
                    done,
                } => {
                    if generation != self.membership_generation || self.token.is_none() {
                        continue;
                    }
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if done {
                        self.membership_loading = false;
                    }
                    if let Ok(uris) = uris {
                        if uris.contains(&self.playlist_picker_track_uri) {
                            self.playlist_picker_added.insert(playlist_id.clone());
                        } else {
                            self.playlist_picker_added.remove(&playlist_id);
                        }
                        self.playlist_membership.insert(playlist_id, uris);
                    }
                }
                AppEvent::InsightsLoaded {
                    generation,
                    range,
                    result,
                    token,
                } => {
                    if generation != self.insights.generation {
                        continue;
                    }
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if self.insights.finish(generation, range, result)
                        && self.insights.range != range
                    {
                        self.load_insights(ctx);
                    }
                }
                AppEvent::LyricsLoaded { key, result } => self.lyrics.finish(key, result),
                AppEvent::AutoplayLoaded {
                    generation,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if generation != self.autoplay_generation {
                        continue;
                    }
                    self.autoplay_pending = false;
                    if self.queue_cleared_by_user {
                        continue;
                    }
                    match result {
                        Ok(mut tracks) if !tracks.is_empty() => {
                            let first = tracks.remove(0);
                            self.priority_queue.extend(tracks);
                            self.priority_dispatched = Some((first.uri.clone(), Instant::now()));
                            self.dispatch_track_play(first, ctx);
                            self.set_status("Continuing playback…");
                        }
                        Ok(_) => {
                            self.is_playing = false;
                            self.set_status("Reached end of queue");
                        }
                        Err(_) => {
                            self.is_playing = false;
                        }
                    }
                }
                AppEvent::ArtistLoaded {
                    generation,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if generation == self.browse.generation {
                        self.artist_loading = false;
                        match result {
                            Ok(data) => {
                                self.artist_name = data.artist.name.clone();
                                self.browse.artist_id = data.artist.id.clone();
                                self.browse.artist = Some(data.artist);
                                self.artist_tracks = data.songs.items;
                                self.browse.next_songs = data.songs.next;
                                self.browse.albums = data.albums.items;
                                self.browse.next_albums = data.albums.next_offset;
                                self.artist_error = data.notice;
                            }
                            Err(error) => self.artist_error = Some(error),
                        }
                    }
                }
                AppEvent::ArtistSongsLoaded {
                    generation,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if generation == self.browse.generation {
                        self.browse.more_loading = false;
                        match result {
                            Ok(page) => {
                                for track in page.items {
                                    if !self.artist_tracks.iter().any(|old| old.uri == track.uri) {
                                        self.artist_tracks.push(track);
                                    }
                                }
                                self.browse.next_songs = page.next;
                            }
                            Err(error) => self.artist_error = Some(error),
                        }
                    }
                }
                AppEvent::ArtistAlbumsLoaded {
                    generation,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if generation == self.browse.generation {
                        self.browse.more_loading = false;
                        match result {
                            Ok(page) => {
                                for album in page.items {
                                    if !self.browse.albums.iter().any(|old| old.id == album.id) {
                                        self.browse.albums.push(album);
                                    }
                                }
                                self.browse.next_albums = page.next_offset;
                            }
                            Err(error) => self.artist_error = Some(error),
                        }
                    }
                }
                AppEvent::AlbumLoaded {
                    generation,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if generation == self.browse.generation {
                        self.browse.album_loading = false;
                        match result {
                            Ok(details) => self.browse.album = Some(details),
                            Err(error) => self.browse.album_error = Some(error),
                        }
                    }
                }
                AppEvent::Desktop(action) => self.handle_desktop_action(action, ctx),
                AppEvent::LocalPlayer { generation, event } => {
                    if generation == self.local_player_generation {
                        self.handle_local_event_with_ctx(event, Some(ctx));
                    }
                }
                AppEvent::AuthBrowserReady {
                    url,
                    error,
                    uses_fallback_port,
                } => {
                    if !self
                        .auth_cancel
                        .as_ref()
                        .is_some_and(|cancelled| cancelled.load(Ordering::Relaxed))
                    {
                        self.auth_url = Some(url);
                        self.auth_uses_fallback_port = uses_fallback_port;
                        self.auth_error = error;
                        self.set_status("Waiting for Spotify sign-in · You can copy the sign-in link from Settings");
                    }
                }
                AppEvent::Connected(result) => {
                    self.auth_in_progress = false;
                    self.auth_url = None;
                    let cancelled = self
                        .auth_cancel
                        .take()
                        .is_some_and(|cancelled| cancelled.load(Ordering::Relaxed));
                    if cancelled {
                        self.auth_error = None;
                        self.set_status("Spotify sign-in cancelled · You can connect again");
                        continue;
                    }
                    match result {
                        Ok(token) => {
                            self.reset_spotify_requests();
                            self.auth_error = None;
                            self.accept_token(token);
                            if self.current_track.id.starts_with("demo-") {
                                self.current_track = Track {
                                    name: "Restoring playback…".into(),
                                    artist: "Connecting to your Spotify session".into(),
                                    ..Default::default()
                                };
                                self.priority_queue.clear();
                                self.priority_dispatched = None;
                                self.priority_auto_armed = false;
                                self.suppress_upstream_queue = false;
                                self.active_collection_uri = None;
                                self.queue.clear();
                                self.is_playing = false;
                                self.progress_ms = 0;
                                self.progress = 0.0;
                                self.duration_ms = 1;
                            }
                            self.last_player_poll = None;
                            self.last_queue_poll = None;
                            self.set_status(
                                "Spotify connected · Live search and Connect controls enabled",
                            );
                            self.load_library(ctx, true);
                            self.load_devices(ctx);
                            self.membership_generation = self.membership_generation.wrapping_add(1);
                            self.membership_loading = false;
                            self.membership_attempted.clear();
                            self.playlist_membership.clear();
                            let top_music_authorization = self.insights.permission_needed;
                            self.insights.reset();
                            if top_music_authorization {
                                self.page = Page::Home;
                            }
                            self.load_home_data(ctx);
                            if self.page == Page::Search && !self.search_query.trim().is_empty() {
                                self.run_search(self.search_query.clone(), ctx);
                            }
                            if self.page == Page::Artist {
                                self.open_artist(self.artist_name.clone(), ctx);
                            }
                            if self.page == Page::Album
                                && let Some(seed) = self.browse.album_seed.clone()
                            {
                                self.open_album(seed, ctx);
                            }
                        }
                        Err(error) => {
                            self.auth_error = Some(error.clone());
                            self.search_loading = false;
                            if self.page == Page::Search {
                                self.search_error = Some(error.clone());
                            }
                            self.show_settings = true;
                            self.set_status(error);
                        }
                    }
                }
                AppEvent::SearchFinished {
                    query,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if query == self.last_sent_query {
                        self.search_loading = false;
                        match result {
                            Ok(tracks) => {
                                self.search_results = tracks;
                                self.search_error = None;
                            }
                            Err(error) => {
                                self.search_error = Some(error);
                            }
                        }
                    }
                }
                AppEvent::SearchCatalogFinished {
                    query,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if query == self.last_sent_query {
                        self.search_loading = false;
                        match result {
                            Ok(catalog) => {
                                self.search_results = catalog.tracks.clone();
                                self.search_catalog = Some(catalog);
                                self.search_error = None;
                            }
                            Err(error) => {
                                self.search_catalog = None;
                                self.search_error = Some(error);
                            }
                        }
                    }
                }
                AppEvent::ActionFinished {
                    action,
                    result,
                    token,
                } => {
                    if action == "volume" {
                        self.volume_control.complete(result.is_ok());
                    }
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    match result {
                        Ok(()) => {
                            match action.as_str() {
                                "pause" => {
                                    let position = self.current_progress_ms();
                                    self.progress_ms = position;
                                    self.progress =
                                        position as f32 / self.duration_ms.max(1) as f32;
                                    self.is_playing = false;
                                }
                                "resume" | "play_track" => self.is_playing = true,
                                "play_context" => {
                                    if self.local_player.is_some() && !self.follow_remote_device {
                                        self.local_command("refresh_context", vec![]);
                                    }
                                    self.last_queue_poll = None;
                                    self.last_player_poll = None;
                                    self.set_status("Collection playback started");
                                }
                                "next" => self.set_status("Skipped to next track"),
                                "previous" => self.set_status("Went to previous track"),
                                value if value.starts_with("queue:") => {
                                    self.set_status("Added to queue");
                                    self.last_queue_poll = None;
                                    self.last_player_poll = None;
                                }
                                value if value.starts_with("save:") => {
                                    if let Some((track, _)) =
                                        self.library_changes.remove(&value[5..])
                                    {
                                        self.saved_track_ids.insert(track.id.clone());
                                        self.saved_tracks.retain(|saved| saved.id != track.id);
                                        self.saved_tracks.insert(0, track);
                                    }
                                    self.library_revision += 1;
                                    self.set_status("Saved to your Spotify library")
                                }
                                value if value.starts_with("unsave:") => {
                                    if let Some((track, _)) =
                                        self.library_changes.remove(&value[7..])
                                    {
                                        self.saved_track_ids.remove(&track.id);
                                        self.saved_tracks.retain(|saved| saved.id != track.id);
                                    }
                                    self.library_revision += 1;
                                    self.set_status("Removed from your Spotify library")
                                }
                                value if value.starts_with("playlist_remove:") => {
                                    self.playlist_add_pending = None;
                                    self.playlist_add_error = None;
                                    if let Some((id, uri)) = self.playlist_remove_target.take() {
                                        self.playlist_removal_committed(&id, &[uri], ctx);
                                    }
                                    self.set_status("Removed from playlist");
                                }
                                value if value.starts_with("playlist_add:") => {
                                    self.playlist_add_pending = None;
                                    self.playlist_add_error = None;
                                    if self.track_to_add_to_playlist.as_ref().is_some_and(|track| {
                                        track.uri == self.playlist_picker_track_uri
                                    }) {
                                        if let Some(id) = value.split(':').nth(1) {
                                            self.playlist_picker_added.insert(id.to_owned());
                                            if let Some(items) =
                                                self.playlist_membership.get_mut(id)
                                            {
                                                items
                                                    .insert(self.playlist_picker_track_uri.clone());
                                            }
                                        }
                                    }
                                    self.set_status("Added track to playlist");
                                    self.last_library_poll = None;
                                    let playlist_id = value.split(':').nth(1).unwrap_or_default();
                                    if let Some(playlist) = self
                                        .playlists
                                        .iter_mut()
                                        .find(|item| item.id == playlist_id)
                                    {
                                        playlist.total_items += 1;
                                    }
                                    if let Some(playlist) = self
                                        .playlists
                                        .iter()
                                        .find(|item| item.id == playlist_id)
                                        .cloned()
                                    {
                                        if self
                                            .selected_playlist
                                            .as_ref()
                                            .map(|item| item.id.as_str())
                                            == Some(playlist_id)
                                        {
                                            self.open_playlist(playlist, ctx);
                                        }
                                    }
                                }
                                value if value.starts_with("device:") => {
                                    if value != "device:Aurafy" {
                                        self.select_local_on_ready = false;
                                        self.follow_remote_device = true;
                                    }
                                    self.set_status(
                                        "Playback transfer accepted · Waiting for the device",
                                    );
                                    self.load_devices(ctx);
                                }
                                "seek" => self.set_status("Playback position updated"),
                                "shuffle" => self.set_status("Shuffle updated"),
                                "repeat" => self.set_status("Repeat mode updated"),
                                _ => {}
                            }
                            if action != "volume" {
                                self.last_player_poll = None;
                            }
                            if action == "pause"
                                || action == "resume"
                                || action == "play_track"
                                || action == "next"
                                || action == "previous"
                            {
                                self.progress_anchor = Instant::now();
                            }
                        }
                        Err(error) => {
                            if action == "resume"
                                && error.ends_with(spotify::PLAYBACK_NOT_FOUND)
                                && !self.is_playing
                                && !self.local_status.active
                                && self.config.local_playback
                                && self.local_player.is_some()
                                && (self.local_status.paired || self.local_starting)
                            {
                                self.follow_remote_device = false;
                                self.device_name = None;
                                self.device_id = None;
                                self.playback_error = None;
                                self.send_playback_action("resume", ctx);
                                continue;
                            }
                            if matches!(
                                action.as_str(),
                                "play_track"
                                    | "resume"
                                    | "pause"
                                    | "next"
                                    | "previous"
                                    | "seek"
                                    | "volume"
                                    | "shuffle"
                                    | "repeat"
                            ) || action.starts_with("device:")
                            {
                                self.playback_error = Some(error.clone());
                            }
                            if action.starts_with("save:") || action.starts_with("unsave:") {
                                if let Some((_, id)) = action.split_once(':') {
                                    self.library_changes.remove(id);
                                }
                                self.library_revision += 1;
                                self.last_library_poll = None;
                            }
                            if action.starts_with("playlist_add:")
                                || action.starts_with("playlist_remove:")
                            {
                                self.playlist_remove_target = None;
                                self.playlist_add_pending = None;
                                self.playlist_add_error = Some(error.clone());
                            }
                            self.last_player_poll = None;
                            self.set_status(error);
                        }
                    }
                }
                AppEvent::PlayerSynced {
                    playback,
                    queue,
                    token,
                } => {
                    self.player_poll_in_progress = false;
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    match playback {
                        Ok(Some(state)) => {
                            if !self.local_status.active {
                                let local_name = self.local_device_name();
                                if state.is_playing
                                    && state
                                        .device_name
                                        .as_deref()
                                        .is_some_and(|name| name != local_name)
                                {
                                    self.follow_remote_device = true;
                                }
                                let is_ad = state.is_ad;
                                self.apply_playback_state(state);
                                self.handle_ad_block(is_ad, ctx);
                            }
                        }
                        Ok(None) if !self.local_status.active && !self.use_free_player() => {
                            self.follow_remote_device = false;
                            self.is_playing = false;
                            self.device_name = None;
                            self.device_id = None;
                            if self.current_track.uri.is_empty() {
                                self.current_track = Track {
                                    name: "Nothing playing yet".into(),
                                    artist: "Choose a song, or press Play to resume".into(),
                                    ..Default::default()
                                };
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            if error.contains("authorization expired")
                                || error.contains("Reconnect Spotify")
                            {
                                self.set_status(error);
                            }
                        }
                    }
                    if !self.local_status.active
                        && (!self.config.local_playback || self.follow_remote_device)
                        && !self.suppress_upstream_queue
                        && let Some(Ok(queue)) = queue
                    {
                        self.queue = queue;
                    }
                }
                AppEvent::LibraryLoaded {
                    revision,
                    saved_tracks,
                    playlists,
                    token,
                } => {
                    self.library_loading = false;
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    match saved_tracks {
                        _ if revision != self.library_revision => {
                            self.library_refresh_needed = true;
                        }
                        Ok(tracks) => {
                            self.saved_track_ids =
                                tracks.iter().map(|track| track.id.clone()).collect();
                            self.saved_tracks = tracks;
                        }
                        Err(error) => self.library_error = Some(error),
                    }
                    match playlists {
                        Ok(playlists) => {
                            self.playlist_membership.retain(|id, _| {
                                self.playlists
                                    .iter()
                                    .find(|p| &p.id == id)
                                    .zip(playlists.iter().find(|p| &p.id == id))
                                    .is_some_and(|(old, new)| old.total_items == new.total_items)
                            });
                            self.membership_attempted
                                .retain(|id| self.playlist_membership.contains_key(id));
                            self.playlists = playlists;
                        }
                        Err(error) => self.library_error = Some(error),
                    }
                    if self.library_error.is_none() {
                        self.set_status(format!(
                            "Library loaded · {} saved tracks · {} playlists",
                            self.saved_tracks.len(),
                            self.playlists.len()
                        ));
                    }
                }
                AppEvent::PlaylistLoaded {
                    playlist_id,
                    tracks,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if self
                        .selected_playlist
                        .as_ref()
                        .map(|playlist| playlist.id.as_str())
                        == Some(playlist_id.as_str())
                    {
                        self.playlist_loading = false;
                        match tracks {
                            Ok(tracks) => {
                                self.playlist_tracks = tracks;
                                self.playlist_error = None;
                            }
                            Err(error) => self.playlist_error = Some(error),
                        }
                    }
                }
                AppEvent::CollectionPlayLoaded {
                    generation,
                    start,
                    track,
                    result,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    if generation == self.collection_play_generation {
                        match result {
                            Ok(tracks) => {
                                if !self.start_local_collection_queue(
                                    start.uri.clone(),
                                    tracks,
                                    start.position,
                                    track.clone(),
                                    ctx,
                                ) {
                                    self.start_remote_collection_context(start, track, ctx);
                                }
                            }
                            Err(error) => self.set_status(error),
                        }
                    }
                }
                AppEvent::PlaylistCreated { result, token } => {
                    self.playlist_creation_in_progress = false;
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    match result {
                        Ok(playlist) => {
                            let name = playlist.name.clone();
                            self.playlists.insert(0, playlist.clone());
                            self.page = Page::Library;
                            self.set_status(format!("Created playlist ‘{name}’"));
                            if let Some(track) = self.track_to_add_to_playlist.clone() {
                                self.show_add_to_playlist = true;
                                self.add_track_to_playlist(playlist, track, ctx);
                            }
                        }
                        Err(error) => {
                            self.track_to_add_to_playlist = None;
                            self.set_status(error);
                        }
                    }
                }
                AppEvent::ArtworkLoaded { url, bytes } => {
                    if self.config.hide_artwork
                        && self
                            .user_profile
                            .as_ref()
                            .and_then(|p| p.image_url.as_deref())
                            != Some(url.as_str())
                    {
                        self.artwork_pending.remove(&url);
                        continue;
                    }
                    self.artwork_pending.remove(&url);
                    match bytes {
                        Ok(bytes) => {
                            if let Ok(decoded) = image::load_from_memory(&bytes) {
                                let resized = decoded
                                    .resize_exact(256, 256, image::imageops::FilterType::Triangle)
                                    .to_rgba8();
                                let size = [resized.width() as usize, resized.height() as usize];
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    resized.as_raw(),
                                );
                                let texture = ctx.load_texture(
                                    url.clone(),
                                    color_image,
                                    egui::TextureOptions::LINEAR,
                                );
                                let colour = appearance::album_colour(resized.as_raw());
                                self.artwork_cache.insert_with_colour(url, texture, colour);
                            } else {
                                self.artwork_failed.insert(url);
                            }
                        }
                        Err(_) => {
                            self.artwork_failed.insert(url);
                        }
                    }
                }
                AppEvent::DevicesLoaded { devices, token } => {
                    self.devices_loading = false;
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    match devices {
                        Ok(devices) => {
                            self.devices = devices;
                            self.devices_error = None;
                        }
                        Err(error) => self.devices_error = Some(error),
                    }
                }
                AppEvent::HomeDataLoaded {
                    recent_tracks,
                    profile,
                    token,
                } => {
                    if let Some(token) = token {
                        self.accept_token(token);
                    }
                    self.recent_loading = false;
                    match recent_tracks {
                        Ok(tracks) => {
                            self.recent_tracks = tracks;
                            self.migrate_recent_search_tracks();
                        }
                        Err(error) => self.recent_error = Some(error),
                    }
                    if let Ok(profile) = profile {
                        self.user_name = if profile.display_name.trim().is_empty() {
                            None
                        } else {
                            Some(profile.display_name.clone())
                        };
                        self.user_profile = Some(profile);
                        self.profile_error = None;
                    } else if let Err(error) = profile {
                        self.profile_error = Some(error);
                    }
                }
                AppEvent::UpdateChecked(result) => {
                    self.updates.checking = false;
                    match result {
                        Ok(outcome) => {
                            self.updates.outcome = Some(outcome);
                            self.updates.error = None;
                        }
                        Err(error) => {
                            self.updates.error = Some(error);
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn current_progress_ms(&self) -> u64 {
        if self.is_playing && !self.seek_interacting {
            self.progress_ms
                .saturating_add(self.progress_anchor.elapsed().as_millis() as u64)
                .min(self.duration_ms)
        } else {
            (self.progress * self.duration_ms as f32).round() as u64
        }
    }

    pub(crate) fn draw_playlist_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_create_playlist {
            let mut open = self.show_create_playlist;
            egui::Window::new("Create playlist")
                .open(&mut open)
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .default_width(440.0)
                .frame(
                    Frame::new()
                        .fill(appearance::palette(ctx).panel2)
                        .stroke(Stroke::new(1.0_f32, appearance::palette(ctx).border))
                        .corner_radius(14)
                        .inner_margin(22),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Create playlist").size(22.0).strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("×").clicked() {
                                self.show_create_playlist = false;
                            }
                        });
                    });
                    ui.add_space(14.0);
                    ui.label(
                        RichText::new("Name")
                            .size(14.0)
                            .strong()
                            .color(appearance::palette(ctx).text),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        TextEdit::singleline(&mut self.new_playlist_name)
                            .hint_text("New playlist")
                            .margin(Margin::symmetric(12, 10))
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(8.0);
                    ui.add(
                        TextEdit::multiline(&mut self.new_playlist_description)
                            .hint_text("Description (optional)")
                            .margin(Margin::symmetric(12, 10))
                            .desired_width(f32::INFINITY)
                            .desired_rows(3),
                    );
                    ui.checkbox(&mut self.new_playlist_public, "Make this playlist public");
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_create_playlist = false;
                        }
                        if ui
                            .add_enabled(
                                !self.new_playlist_name.trim().is_empty()
                                    && !self.playlist_creation_in_progress
                                    && self.token.is_some(),
                                egui::Button::new(
                                    RichText::new("Create playlist")
                                        .color(appearance::palette(ctx).button_foreground),
                                )
                                .fill(appearance::palette(ctx).green),
                            )
                            .clicked()
                        {
                            self.create_playlist(ctx);
                        }
                    });
                });
            self.show_create_playlist = open && self.show_create_playlist;
        }

        if self.show_add_to_playlist {
            self.draw_playlist_picker(ctx);
        }
        if !self.show_add_to_playlist
            && !self.show_create_playlist
            && !self.playlist_creation_in_progress
            && self.playlist_add_pending.is_none()
        {
            self.track_to_add_to_playlist = None;
        }
    }
}
impl eframe::App for AurafyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let ui_started = Instant::now();
        self.ui_frames += 1;
        self.ui_repaint_causes = ctx
            .repaint_causes()
            .iter()
            .map(ToString::to_string)
            .collect();
        self.remember_window(Some(ctx));
        if self.window_restore_pending.is_some() {
            ctx.request_repaint_after(Duration::from_millis(20));
        }
        if !self.startup_launch_handled
            && self.window_restore_pending.is_none()
            && self
                .window_geometry
                .as_ref()
                .is_none_or(desktop::Geometry::visible)
        {
            self.startup_launch_handled = true;
            if self.tray.as_ref().is_some_and(desktop::Tray::registered) {
                self.window_state.hidden = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            }
        }
        self.handle_events(ctx);
        self.flush_volume(ctx);
        if self.library_refresh_needed && self.is_connected() && !self.library_loading {
            self.load_library(ctx, true);
        }
        self.tick_sleep_timer(ctx);
        if ctx.input(|input| input.viewport().close_requested()) {
            self.save_session();
            self.window_state.close(
                self.tray.as_ref().is_some_and(desktop::Tray::registered),
                self.config.quit_on_close,
                ctx,
            );
            self.publish_runtime_status();
        }
        if let Some(tray) = &mut self.tray {
            tray.update(
                self.is_playing,
                &self.current_track.name,
                &self.current_track.artist,
            );
        }
        #[cfg(target_os = "windows")]
        {
            let pos = self.current_progress_ms();
            if let Some(smtc) = &mut self.smtc {
                smtc.sync(&self.current_track, self.is_playing, pos, self.duration_ms);
            }
        }

        if self.should_refresh_devices() {
            self.load_devices(ctx);
        }

        if self.show_settings {
            self.page = Page::Settings;
            self.settings_tab = SettingsTab::Account;
            self.show_settings = false;
        }

        if !ctx.wants_keyboard_input() {
            let (toggle, previous, next, quit, seek_back, seek_forward, volume_down, volume_up) =
                ctx.input(|input| {
                    (
                        input.key_pressed(Key::Space),
                        input.modifiers.ctrl && input.key_pressed(Key::ArrowLeft),
                        input.modifiers.ctrl && input.key_pressed(Key::ArrowRight),
                        input.modifiers.ctrl && input.key_pressed(Key::Q),
                        input.modifiers.alt
                            && !input.modifiers.ctrl
                            && input.key_pressed(Key::ArrowLeft),
                        input.modifiers.alt
                            && !input.modifiers.ctrl
                            && input.key_pressed(Key::ArrowRight),
                        input.modifiers.ctrl && input.key_pressed(Key::ArrowDown),
                        input.modifiers.ctrl && input.key_pressed(Key::ArrowUp),
                    )
                });
            if quit {
                self.handle_desktop_action(desktop::Action::Quit, ctx);
            } else if toggle {
                self.send_playback_action(if self.is_playing { "pause" } else { "resume" }, ctx);
            } else if previous {
                self.send_playback_action("previous", ctx);
            } else if next {
                self.send_playback_action("next", ctx);
            } else if seek_back || seek_forward {
                self.seek_relative(seek_forward, ctx);
            } else if volume_down || volume_up {
                self.step_volume(volume_up, ctx);
            }
        }
        if ctx.input(|input| input.key_pressed(Key::F11)) {
            let fullscreen = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
        } else if ctx.input(|input| {
            input.key_pressed(Key::Escape) && input.viewport().fullscreen.unwrap_or(false)
        }) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }

        if self.is_connected()
            && !self.local_status.active
            && !self.player_poll_in_progress
            && self
                .last_player_poll
                .is_none_or(|last| last.elapsed() >= Duration::from_secs(3))
        {
            let include_queue = (!self.config.local_playback || self.follow_remote_device)
                && self.show_queue
                && self
                    .last_queue_poll
                    .is_none_or(|last| last.elapsed() >= Duration::from_secs(18));
            self.poll_player(ctx, include_queue);
        }
        if self.is_playing {
            self.priority_auto_armed = true;
        }
        let recently_dispatched = self
            .priority_dispatched
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() < Duration::from_secs(3));
        if self.is_playing && !self.seek_interacting {
            let position = self.current_progress_ms();
            if !self.priority_queue.is_empty()
                && self.repeat_state != "track"
                && !recently_dispatched
                && self.duration_ms > 2000
                && position >= self.duration_ms.saturating_sub(550)
            {
                let _ = self.play_next_priority_track(ctx);
            } else if self.priority_queue.is_empty()
                && self.suppress_upstream_queue
                && self.repeat_state != "track"
                && !recently_dispatched
                && self.duration_ms > 2000
                && position >= self.duration_ms.saturating_sub(550)
            {
                self.handle_suppressed_queue_end(None, ctx);
            } else if self.is_demo_mode() && position >= self.duration_ms {
                self.advance_demo_track(1);
            } else {
                self.progress = (position as f32 / self.duration_ms.max(1) as f32).clamp(0.0, 1.0);
            }
        } else if self.playback_idle
            && self.priority_auto_armed
            && self.use_local_for_play()
            && !self.priority_queue.is_empty()
            && !recently_dispatched
        {
            self.priority_auto_armed = false;
            let _ = self.play_next_priority_track(ctx);
        } else if self.playback_idle
            && self.priority_auto_armed
            && !self.is_demo_mode()
            && self.priority_queue.is_empty()
            && self.queue.is_empty()
            && !self.queue_cleared_by_user
            && self.repeat_state == "off"
            && !recently_dispatched
            && !self.autoplay_pending
            && self.token.is_some()
            && !self.current_track.uri.is_empty()
        {
            self.priority_auto_armed = false;
            self.start_autoplay_continuation(ctx);
        }

        if !self.window_state.hidden {
            self.artwork_cache.begin_frame();
            self.draw_player(ctx);
            self.draw_header(ctx);
            self.draw_sidebar(ctx);
            self.draw_queue(ctx);
            self.draw_main(ctx);
            self.draw_playlist_dialogs(ctx);
            self.draw_playlist_editor(ctx);
            self.draw_resize_handles(ctx);
            self.artwork_cache.finish_frame();
        }
        #[cfg(target_os = "windows")]
        if let Some(mut capture) = self.player_capture.take() {
            if let Some(geometry) = &self.window_geometry {
                let ready = !self.auth_in_progress
                    && !self.local_starting
                    && self.artwork_pending.is_empty();
                if !capture.update(ctx, geometry, self.player_layout, ready) {
                    self.player_capture = Some(capture);
                }
            } else {
                self.player_capture = Some(capture);
            }
        }
        if self.last_session_save.elapsed() >= Duration::from_secs(10) && !self.is_demo_mode() {
            self.save_session();
            self.publish_runtime_status();
        }
        self.ui_build_ms = ui_started.elapsed().as_secs_f64() * 1000.0;
        let mut repaint_after = self.config.display.interval(
            self.is_playing,
            self.window_state.hidden,
            self.config.smooth_progress,
            self.config.reduce_motion,
        );
        if self.is_playing && (!self.priority_queue.is_empty() || self.suppress_upstream_queue) {
            let remaining = self
                .duration_ms
                .saturating_sub(500)
                .saturating_sub(self.current_progress_ms());
            repaint_after = repaint_after.min(Duration::from_millis(remaining.max(25)));
        }
        ctx.request_repaint_after(repaint_after);
    }
}

pub(crate) fn paint_cover_rounded(ui: &Ui, rect: Rect, label: &str, radius: f32) {
    let palettes = [
        [
            Color32::from_rgb(91, 42, 131),
            Color32::from_rgb(28, 156, 170),
        ],
        [
            Color32::from_rgb(184, 68, 91),
            Color32::from_rgb(67, 47, 118),
        ],
        [
            Color32::from_rgb(39, 112, 123),
            Color32::from_rgb(22, 44, 63),
        ],
        [
            Color32::from_rgb(190, 117, 52),
            Color32::from_rgb(104, 48, 89),
        ],
        [
            Color32::from_rgb(67, 80, 147),
            Color32::from_rgb(31, 31, 45),
        ],
        [
            Color32::from_rgb(34, 136, 91),
            Color32::from_rgb(33, 55, 94),
        ],
    ];
    let index = label
        .bytes()
        .fold(0_usize, |acc, b| acc.wrapping_add(b as usize))
        % palettes.len();
    let [a, b] = palettes[index];
    let circular = radius >= rect.width().min(rect.height()) * 0.45;
    if circular {
        let r = rect.width().min(rect.height()) * 0.5;
        ui.painter().circle_filled(rect.center(), r, a);
        ui.painter().circle_filled(
            rect.center() + Vec2::new(r * 0.16, r * 0.18),
            r * 0.62,
            Color32::from_rgba_unmultiplied(b.r(), b.g(), b.b(), 190),
        );
        ui.painter().circle_filled(
            rect.center() + Vec2::new(r * 0.22, -r * 0.18),
            r * 0.34,
            Color32::from_rgba_unmultiplied(255, 255, 255, 42),
        );
    } else {
        ui.painter().rect_filled(rect, radius, a);
        let inner = Rect::from_min_size(
            rect.min + Vec2::new(rect.width() * 0.22, rect.height() * 0.22),
            Vec2::new(rect.width() * 0.78, rect.height() * 0.78),
        );
        ui.painter().rect_filled(inner, radius, b);
        ui.painter().circle_filled(
            rect.center() + Vec2::new(rect.width() * 0.13, -rect.height() * 0.08),
            rect.width() * 0.19,
            Color32::from_rgba_unmultiplied(255, 255, 255, 45),
        );
    }
    let initial = label
        .chars()
        .next()
        .unwrap_or('A')
        .to_uppercase()
        .to_string();
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional((rect.width() * 0.38).max(12.0)),
        Color32::from_rgba_unmultiplied(255, 255, 255, 220),
    );
}

pub(crate) fn format_time(seconds: u64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub(crate) fn demo_tracks() -> Vec<Track> {
    let raw = [
        ("Good Morning", "Kanye West", "Graduation", 195),
        ("Don't Stop The Music", "Rihanna", "Good Girl Gone Bad", 267),
        (
            "Everyday (feat. Rod Stewart)",
            "A$AP Rocky",
            "AT.LONG.LAST.A$AP",
            260,
        ),
        (
            "goosebumps",
            "Travis Scott",
            "Birds in the Trap Sing McKnight",
            243,
        ),
        ("One Dance", "Drake, Wizkid, Kyla", "Views", 173),
        (
            "Goodbyes (feat. Young Thug)",
            "Post Malone",
            "Hollywood's Bleeding",
            174,
        ),
        ("THE SCOTTS", "THE SCOTTS, Travis Scott", "THE SCOTTS", 165),
        ("Bound 2", "Kanye West", "Yeezus", 229),
        (
            "Moves Like Jagger",
            "Maroon 5, Christina Aguilera",
            "Hands All Over",
            201,
        ),
        ("Live Your Life", "T.I., Rihanna", "Paper Trail", 338),
    ];
    raw.iter()
        .enumerate()
        .map(|(index, (name, artist, album, duration))| Track {
            id: format!("demo-{index}"),
            name: (*name).into(),
            artist: (*artist).into(),
            artist_names: vec![(*artist).into()],
            album: (*album).into(),
            uri: String::new(),
            duration_ms: *duration * 1000,
            artwork_url: None,
            explicit: false,
            ..Default::default()
        })
        .collect()
}

impl Drop for AurafyApp {
    fn drop(&mut self) {
        self.remember_window(None);
        self.save_session();
        if let Some(cancelled) = &self.auth_cancel {
            cancelled.store(true, Ordering::Relaxed);
        }
    }
}
