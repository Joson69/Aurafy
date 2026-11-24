use super::*;

impl AurafyApp {
    pub(crate) fn toggle_saved_track(&mut self, track: Track, ctx: &egui::Context) {
        if !self.is_connected() {
            self.set_status("Connect Spotify to save tracks to your account");
            return;
        }
        if let Err(error) = spotify::validate_track_uri(&track.uri) {
            self.set_status(error);
            return;
        }
        if self.library_changes.contains_key(&track.id) {
            return;
        }
        let save = !self.saved_track_ids.contains(&track.id);
        let uri = track.uri.clone();
        let action = format!("{}:{}", if save { "save" } else { "unsave" }, track.id);
        self.set_status(format!(
            "{} {}…",
            if save { "Saving" } else { "Removing" },
            track.name
        ));
        self.library_changes.insert(track.id.clone(), (track, save));
        self.library_revision += 1;
        self.start_spotify_action(
            action,
            move |access| {
                if save {
                    spotify::save_track(access, &uri)
                } else {
                    spotify::remove_saved_track(access, &uri)
                }
            },
            ctx,
        );
    }

    pub(crate) fn known_memberships(&self, uri: &str) -> HashSet<String> {
        self.playlist_membership
            .iter()
            .filter(|(_, items)| items.contains(uri))
            .map(|(id, _)| id.clone())
            .collect()
    }
    pub(crate) fn ensure_playlist_membership(&mut self, ctx: &egui::Context, all: bool) {
        if self.membership_loading {
            return;
        }
        let Some(token) = self.token.clone() else {
            return;
        };
        let Some(profile) = self.user_profile.as_ref() else {
            return;
        };
        let lists: Vec<_> = self
            .playlists
            .iter()
            .filter(|p| {
                p.is_owned_by(&profile.id)
                    && p.items_accessible
                    && !self.playlist_membership.contains_key(&p.id)
                    && !self.membership_attempted.contains(&p.id)
                    && (all
                        || self.playback_context_uri.as_deref()
                            == Some(format!("spotify:playlist:{}", p.id).as_str()))
            })
            .map(|p| p.id.clone())
            .collect();
        if lists.is_empty() {
            return;
        }
        self.membership_attempted.extend(lists.iter().cloned());
        self.membership_loading = true;
        #[cfg(not(test))]
        {
            let tx = self.spotify_tx.clone();
            let ctx = ctx.clone();
            let generation = self.membership_generation;
            let client = self.config.client_id.clone();
            thread::spawn(move || {
                let token = spotify::refresh_if_needed(token, &client);
                let count = lists.len();
                for (index, id) in lists.into_iter().enumerate() {
                    let uris = match &token {
                        Ok(t) => spotify::get_playlist_uris(&t.access_token, &id),
                        Err(e) => Err(e.clone()),
                    };
                    let _ = tx.send(AppEvent::MembershipLoaded {
                        generation,
                        playlist_id: id,
                        uris,
                        token: token.clone().ok(),
                        done: index + 1 == count,
                    });
                    ctx.request_repaint();
                }
            });
        }
        #[cfg(test)]
        {
            self.membership_loading = false;
            let _ = (token, ctx, lists);
        }
    }
    pub(crate) fn open_playlist_picker(&mut self, track: Track) {
        if self.playlist_add_pending.is_some() {
            return;
        }
        if self.playlist_picker_track_uri != track.uri {
            self.playlist_picker_added = self.known_memberships(&track.uri);
            self.playlist_picker_track_uri = track.uri.clone();
        }
        if let Some(selected) = &self.selected_playlist {
            if self
                .playlist_tracks
                .iter()
                .any(|item| item.uri == track.uri)
            {
                self.playlist_picker_added.insert(selected.id.clone());
            }
        }
        if !self.membership_loading {
            self.membership_attempted
                .retain(|id| self.playlist_membership.contains_key(id));
        }
        self.playlist_picker_anchor = self.current_track_menu_rect;
        self.playlist_picker_rect = None;
        self.track_to_add_to_playlist = Some(track);
        self.playlist_add_error = None;
        self.playlist_picker_filter.clear();
        self.show_add_to_playlist = true;
    }

    pub(crate) fn create_playlist(&mut self, ctx: &egui::Context) {
        let name = self.new_playlist_name.trim().to_string();
        if name.is_empty() {
            self.set_status("Give the playlist a name first");
            return;
        }
        let Some(token) = self.token.clone() else {
            self.set_status("Connect Spotify to create playlists");
            return;
        };
        let description = self.new_playlist_description.trim().to_string();
        let is_public = self.new_playlist_public;
        let client_id = self.config.client_id.clone();
        let tx = self.spotify_tx.clone();
        let ctx = ctx.clone();
        self.show_create_playlist = false;
        self.playlist_creation_in_progress = true;
        self.set_status(format!("Creating playlist ‘{name}’…"));
        thread::spawn(move || {
            let (token, result) = match spotify::refresh_if_needed(token, &client_id) {
                Ok(token) => {
                    let result = spotify::create_playlist(
                        &token.access_token,
                        &name,
                        &description,
                        is_public,
                    );
                    (Some(token), result)
                }
                Err(error) => (None, Err(error)),
            };
            let _ = tx.send(AppEvent::PlaylistCreated { result, token });
            ctx.request_repaint();
        });
        self.new_playlist_name.clear();
        self.new_playlist_description.clear();
        self.new_playlist_public = false;
    }

    pub(crate) fn add_track_to_playlist(
        &mut self,
        playlist: Playlist,
        track: Track,
        ctx: &egui::Context,
    ) {
        if self.playlist_add_pending.is_some() {
            return;
        }
        if !self.is_connected() {
            self.playlist_add_error = Some("Connect Spotify before adding to a playlist.".into());
            return;
        }
        if let Err(error) = spotify::validate_track_uri(&track.uri) {
            self.playlist_add_error = Some(error);
            return;
        }
        if !self
            .user_profile
            .as_ref()
            .is_some_and(|profile| playlist.is_owned_by(&profile.id))
        {
            self.playlist_add_error =
                Some("Choose a playlist owned by your Spotify account.".into());
            return;
        }
        if self.playlist_picker_track_uri == track.uri
            && self.playlist_picker_added.contains(&playlist.id)
        {
            return;
        }
        let uri = track.uri.clone();
        let playlist_id = playlist.id.clone();
        let playlist_name = playlist.name.clone();
        let track_name = track.name.clone();
        self.playlist_add_pending = Some(playlist_id.clone());
        self.playlist_add_error = None;
        self.start_spotify_action(
            format!("playlist_add:{}:{}", playlist_id, track.id),
            move |access| spotify::add_to_playlist(access, &playlist_id, &uri),
            ctx,
        );
        self.set_status(format!("Adding {track_name} to {playlist_name}…"));
    }

    pub(crate) fn remove_track_from_playlist(
        &mut self,
        playlist: Playlist,
        track: Track,
        ctx: &egui::Context,
    ) {
        if self.playlist_add_pending.is_some() {
            return;
        }
        if !self.is_connected()
            || !self
                .user_profile
                .as_ref()
                .is_some_and(|p| playlist.is_owned_by(&p.id))
        {
            self.playlist_add_error =
                Some("Choose a playlist owned by your Spotify account.".into());
            return;
        }
        if let Err(error) = spotify::validate_track_uri(&track.uri) {
            self.playlist_add_error = Some(error);
            return;
        }
        let id = playlist.id.clone();
        let uri = track.uri.clone();
        self.playlist_remove_target = Some((id.clone(), uri.clone()));
        self.playlist_add_pending = Some(id.clone());
        self.playlist_add_error = None;
        self.start_spotify_action(
            format!("playlist_remove:{id}:{}", track.id),
            move |access| spotify::remove_from_playlist(access, &id, &uri),
            ctx,
        );
    }

    pub(super) fn playlist_removal_committed(
        &mut self,
        id: &str,
        uris: &[String],
        ctx: &egui::Context,
    ) {
        let removed: HashSet<_> = uris.iter().collect();
        if let Some(items) = self.playlist_membership.get_mut(id) {
            items.retain(|uri| !removed.contains(uri));
        }
        if removed.contains(&self.playlist_picker_track_uri) {
            self.playlist_picker_added.remove(id);
        }
        self.membership_generation = self.membership_generation.wrapping_add(1);
        self.membership_loading = false;
        self.membership_attempted
            .retain(|id| self.playlist_membership.contains_key(id));
        if self
            .selected_playlist
            .as_ref()
            .is_some_and(|playlist| playlist.id == id)
        {
            self.playlist_tracks
                .retain(|track| !removed.contains(&track.uri));
            #[cfg(not(test))]
            if let Some(playlist) = self.selected_playlist.clone() {
                self.open_playlist(playlist, ctx);
            }
        }
        self.library_revision += 1;
        self.last_library_poll = Instant::now() - Duration::from_secs(60);
        #[cfg(not(test))]
        self.load_library(ctx, true);
        #[cfg(test)]
        let _ = ctx;
    }

    pub(crate) fn load_library(&mut self, ctx: &egui::Context, force: bool) {
        if !self.is_connected() {
            self.set_status("Connect Spotify to load your real library");
            return;
        }
        if self.library_loading
            || (!force && self.last_library_poll.elapsed() < Duration::from_secs(30))
        {
            return;
        }
        self.library_loading = true;
        self.library_refresh_needed = false;
        self.library_error = None;
        self.last_library_poll = Instant::now();
        let revision = self.library_revision;
        let token = self.token.clone().unwrap();
        let client_id = self.config.client_id.clone();
        let tx = self.spotify_tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let result = spotify::refresh_if_needed(token, &client_id);
            let (token, saved_tracks, playlists) = match result {
                Ok(token) => {
                    let saved_tracks = spotify::get_saved_tracks(&token.access_token);
                    let playlists = spotify::get_playlists(&token.access_token);
                    (Some(token), saved_tracks, playlists)
                }
                Err(error) => (None, Err(error.clone()), Err(error)),
            };
            let _ = tx.send(AppEvent::LibraryLoaded {
                revision,
                saved_tracks,
                playlists,
                token,
            });
            ctx.request_repaint();
        });
    }

    pub(crate) fn open_playlist(&mut self, playlist: Playlist, ctx: &egui::Context) {
        if !self.is_connected() {
            self.set_status("Connect Spotify to open playlists");
            return;
        }
        if self.page != Page::Library
            || self.selected_playlist.as_ref().map(|current| &current.id) != Some(&playlist.id)
        {
            self.record_nav();
        }
        self.page = Page::Library;
        if self.selected_playlist.as_ref().map(|current| &current.id) != Some(&playlist.id) {
            self.playlist_track_filter.clear();
        }
        self.selected_playlist = Some(playlist.clone());
        self.playlist_error = None;
        self.playlist_tracks.clear();
        if !playlist.items_accessible {
            self.playlist_loading = false;
            self.set_status("Spotify does not expose this playlist's items to this app");
            return;
        }
        self.playlist_loading = true;
        let playlist_id = playlist.id.clone();
        let tx = self.spotify_tx.clone();
        let ctx = ctx.clone();
        let client_id = self.config.client_id.clone();
        let token = self.token.clone().unwrap();
        thread::spawn(move || {
            let (token, tracks) = match spotify::refresh_if_needed(token, &client_id) {
                Ok(token) => {
                    let tracks = spotify::get_playlist_items(&token.access_token, &playlist_id);
                    (Some(token), tracks)
                }
                Err(error) => (None, Err(error)),
            };
            let _ = tx.send(AppEvent::PlaylistLoaded {
                playlist_id,
                tracks,
                token,
            });
            ctx.request_repaint();
        });
    }

    pub(crate) fn poll_player(&mut self, ctx: &egui::Context, include_queue: bool) {
        let Some(token) = self.token.clone() else {
            return;
        };
        if self.player_poll_in_progress {
            return;
        }
        self.player_poll_in_progress = true;
        self.last_player_poll = Instant::now();
        if include_queue {
            self.last_queue_poll = Instant::now();
        }
        let client_id = self.config.client_id.clone();
        let tx = self.spotify_tx.clone();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let refreshed = spotify::refresh_if_needed(token, &client_id);
            let (token, playback, queue) = match refreshed {
                Ok(token) => {
                    let playback = spotify::get_playback_state(&token.access_token);
                    let queue = if include_queue {
                        Some(spotify::get_queue(&token.access_token))
                    } else {
                        None
                    };
                    (Some(token), playback, queue)
                }
                Err(error) => (None, Err(error), None),
            };
            let _ = tx.send(AppEvent::PlayerSynced {
                playback,
                queue,
                token,
            });
            ctx.request_repaint();
        });
    }

    pub(crate) fn request_artwork(&mut self, url: &str, ctx: &egui::Context) {
        if self.artwork_cache.contains_key(url)
            || self.artwork_pending.contains(url)
            || self.artwork_failed.contains(url)
            || self.artwork_pending.len() >= 4
            || !url.starts_with("https://")
        {
            return;
        }
        let url_owned = url.to_string();
        self.artwork_pending.insert(url_owned.clone());
        // Offscreen/UI regression tests supply image responses explicitly.
        #[cfg(not(test))]
        {
            use std::io::Read;
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            thread::spawn(move || {
                let result = (|| {
                    let client = spotify::http_client()?;
                    let response = client.get(&url_owned).send().map_err(|e| e.to_string())?;
                    if !response.status().is_success() {
                        return Err(format!("Artwork HTTP {}", response.status()));
                    }
                    if response.content_length().unwrap_or(0) > 2_000_000 {
                        return Err("Artwork is larger than 2 MB".into());
                    }
                    let mut bytes = Vec::new();
                    response
                        .take(2_000_001)
                        .read_to_end(&mut bytes)
                        .map_err(|e| e.to_string())?;
                    if bytes.len() > 2_000_000 {
                        return Err("Artwork is larger than 2 MB".into());
                    }
                    Ok(bytes.to_vec())
                })();
                let _ = tx.send(AppEvent::ArtworkLoaded {
                    url: url_owned,
                    bytes: result,
                });
                ctx.request_repaint();
            });
        }
        #[cfg(test)]
        let _ = ctx;
    }

    pub(crate) fn seek_relative(&mut self, forward: bool, ctx: &egui::Context) {
        let step = self.config.seek_step() * 1000;
        let current = self.current_progress_ms();
        let position = if forward {
            current.saturating_add(step).min(self.duration_ms)
        } else {
            current.saturating_sub(step)
        };
        self.progress_ms = position;
        self.progress = position as f32 / self.duration_ms.max(1) as f32;
        self.progress_anchor = Instant::now();
        if !self.is_demo_mode() {
            self.start_spotify_action(
                "seek".into(),
                move |access| spotify::seek_to(access, position),
                ctx,
            );
        }
    }

    pub(crate) fn step_volume(&mut self, up: bool, ctx: &egui::Context) {
        let step = self.config.volume_step() * if up { 1.0 } else { -1.0 };
        self.volume = (self.volume + step).clamp(0.0, 1.0);
        self.queue_volume(true, ctx);
    }

    pub(crate) fn apply_display_preferences(&self, ctx: &egui::Context) {
        let album = self
            .current_track
            .artwork_url
            .as_deref()
            .and_then(|url| self.artwork_cache.colour(url));
        let palette = appearance::resolve(self.config.display, ctx.system_theme(), album);
        ctx.set_theme(match self.config.display.theme {
            appearance::Theme::System => egui::ThemePreference::System,
            _ if palette.dark => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::Light,
        });
        if appearance::palette(ctx) != palette {
            appearance::store(ctx, palette);
            ui::apply_style(ctx);
            ctx.style_mut(|style| {
                style.animation_time = if self.config.reduce_motion { 0.0 } else { 0.10 }
            });
        }
        let zoom = self.config.display.zoom();
        if (ctx.zoom_factor() - zoom).abs() > 0.001 {
            ctx.set_zoom_factor(zoom);
            ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(Vec2::new(
                1080.0 * zoom,
                680.0 * zoom,
            )));
        }
    }

    pub(crate) fn toggle_lyrics(&mut self) {
        if self.page == Page::Lyrics {
            self.page = self.lyrics_return_page;
        } else {
            self.lyrics_return_page = self.page;
            self.page = Page::Lyrics;
            self.lyrics_follow = !self.config.lyrics_manual_by_default;
            self.lyrics_last_line = None;
            self.search_popup_open = false;
        }
    }

    pub(crate) fn request_lyrics(&mut self, ctx: &egui::Context) {
        let Some(key) = lyrics::TrackKey::for_track(&self.current_track) else {
            return;
        };
        if !self.lyrics.begin(&key) {
            return;
        }
        #[cfg(not(test))]
        {
            let tx = self.tx.clone();
            let ctx = ctx.clone();
            thread::spawn(move || {
                let result = lyrics::fetch(&key);
                let _ = tx.send(AppEvent::LyricsLoaded { key, result });
                ctx.request_repaint();
            });
        }
        // UI previews inject keyed responses, and never contact a provider.
        #[cfg(test)]
        let _ = ctx;
    }

    pub(crate) fn seek_playback(&mut self, position_ms: u64, ctx: &egui::Context) {
        self.progress_ms = position_ms.min(self.duration_ms);
        self.progress = self.progress_ms as f32 / self.duration_ms.max(1) as f32;
        self.progress_anchor = Instant::now();
        if self.is_connected() || self.use_local_playback() {
            let position = self.progress_ms;
            self.start_spotify_action("seek".into(), move |t| spotify::seek_to(t, position), ctx);
        }
    }
}
