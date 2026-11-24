use super::*;

#[derive(Default)]
pub(super) struct State {
    pub generation: u64,
    pub artist_id: String,
    pub artist: Option<catalog::Artist>,
    pub albums: Vec<catalog::Album>,
    pub next_albums: Option<usize>,
    pub next_songs: Option<catalog::SongCursor>,
    pub releases: bool,
    pub more_loading: bool,
    pub album_seed: Option<Track>,
    pub album: Option<catalog::AlbumDetails>,
    pub album_loading: bool,
    pub album_error: Option<String>,
}

impl State {
    pub fn restore(config: &AppConfig) -> Self {
        Self {
            artist_id: config.last_artist_id.clone(),
            album_seed: config.last_album.clone(),
            album_loading: config.last_page == Page::Album,
            ..Default::default()
        }
    }
}

impl AurafyApp {
    pub(super) fn open_artist(&mut self, name: String, ctx: &egui::Context) {
        let id = if name == self.artist_name {
            self.browse.artist_id.clone()
        } else {
            String::new()
        };
        self.open_artist_id(name, id, ctx);
    }

    pub(super) fn open_artist_id(&mut self, name: String, id: String, ctx: &egui::Context) {
        #[cfg(test)]
        let _ = ctx;
        if name.trim().is_empty() {
            return;
        }
        if self.page != Page::Artist
            || !self.artist_name.eq_ignore_ascii_case(&name)
            || (!id.is_empty() && self.browse.artist_id != id)
        {
            self.record_nav();
        }
        self.browse.generation = self.browse.generation.wrapping_add(1);
        self.browse.artist_id = id.clone();
        self.browse.artist = None;
        self.browse.albums.clear();
        self.browse.next_albums = None;
        self.browse.next_songs = None;
        self.browse.more_loading = false;
        self.browse.releases = false;
        self.page = Page::Artist;
        self.artist_name = name.clone();
        self.artist_tracks.clear();
        self.artist_error = None;
        self.search_popup_open = false;
        self.search_edited_at = None;
        self.artist_loading = false;
        if self.is_demo_mode() {
            self.artist_tracks = self
                .demo_tracks
                .iter()
                .filter(|track| track.artist_names.contains(&name))
                .cloned()
                .collect();
        } else if let Some(token) = self.token.clone() {
            self.artist_loading = true;
            #[cfg(test)]
            let _ = token;
            #[cfg(not(test))]
            {
                let generation = self.browse.generation;
                let tx = self.spotify_tx.clone();
                let client_id = self.config.client_id.clone();
                let ctx = ctx.clone();
                thread::spawn(move || {
                    let (token, result) = match spotify::refresh_if_needed(token, &client_id) {
                        Ok(token) => {
                            let result = catalog::artist_overview(&token.access_token, &id, &name);
                            (Some(token), result)
                        }
                        Err(error) => (None, Err(error)),
                    };
                    let _ = tx.send(AppEvent::ArtistLoaded {
                        generation,
                        result,
                        token,
                    });
                    ctx.request_repaint();
                });
            }
        } else {
            self.artist_loading = self.auth_in_progress;
            if !self.auth_in_progress {
                self.artist_error = Some("Reconnect Spotify to browse this artist.".into());
            }
        }
    }

    pub(super) fn load_more_artist(&mut self, ctx: &egui::Context) {
        if self.browse.more_loading {
            return;
        }
        let releases = self.browse.releases;
        let offset = self.browse.next_albums;
        let cursor = self.browse.next_songs.clone();
        if (releases && offset.is_none()) || (!releases && cursor.is_none()) {
            return;
        }
        let Some(token) = self.token.clone() else {
            return;
        };
        self.browse.more_loading = true;
        self.artist_error = None;
        #[cfg(test)]
        let _ = (token, offset, cursor, ctx);
        #[cfg(not(test))]
        {
            let generation = self.browse.generation;
            let id = self.browse.artist_id.clone();
            let name = self.artist_name.clone();
            let client_id = self.config.client_id.clone();
            let tx = self.spotify_tx.clone();
            let ctx = ctx.clone();
            thread::spawn(move || {
                match spotify::refresh_if_needed(token, &client_id) {
                    Ok(token) => {
                        let event = if releases {
                            AppEvent::ArtistAlbumsLoaded {
                                generation,
                                result: catalog::artist_albums(
                                    &token.access_token,
                                    &id,
                                    offset.unwrap(),
                                ),
                                token: Some(token),
                            }
                        } else {
                            AppEvent::ArtistSongsLoaded {
                                generation,
                                result: catalog::artist_songs(
                                    &token.access_token,
                                    &id,
                                    &name,
                                    cursor,
                                ),
                                token: Some(token),
                            }
                        };
                        let _ = tx.send(event);
                    }
                    Err(error) => {
                        let _ = tx.send(AppEvent::ArtistSongsLoaded {
                            generation,
                            result: Err(error),
                            token: None,
                        });
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    pub(super) fn open_album(&mut self, seed: Track, ctx: &egui::Context) {
        #[cfg(test)]
        let _ = ctx;
        if self.page != Page::Album
            || self
                .browse
                .album_seed
                .as_ref()
                .is_none_or(|s| s.album_id != seed.album_id || s.album != seed.album)
        {
            self.record_nav();
        }
        self.browse.generation = self.browse.generation.wrapping_add(1);
        self.page = Page::Album;
        self.search_popup_open = false;
        self.search_edited_at = None;
        self.browse.album_seed = Some(seed.clone());
        self.browse.album = None;
        self.browse.album_error = None;
        self.browse.album_loading = false;
        if self.is_demo_mode() {
            let album = catalog::Album {
                name: seed.album.clone(),
                artist_names: seed.artist_names.clone(),
                artwork_url: seed.artwork_url.clone(),
                ..Default::default()
            };
            self.browse.album = Some(catalog::AlbumDetails {
                album,
                tracks: self
                    .demo_tracks
                    .iter()
                    .filter(|track| track.album == seed.album)
                    .cloned()
                    .collect(),
            });
        } else if let Some(token) = self.token.clone() {
            self.browse.album_loading = true;
            #[cfg(test)]
            let _ = token;
            #[cfg(not(test))]
            {
                let generation = self.browse.generation;
                let tx = self.spotify_tx.clone();
                let client_id = self.config.client_id.clone();
                let ctx = ctx.clone();
                thread::spawn(move || {
                    let (token, result) = match spotify::refresh_if_needed(token, &client_id) {
                        Ok(token) => {
                            let result = catalog::album_for_track(&token.access_token, &seed);
                            (Some(token), result)
                        }
                        Err(error) => (None, Err(error)),
                    };
                    let _ = tx.send(AppEvent::AlbumLoaded {
                        generation,
                        result,
                        token,
                    });
                    ctx.request_repaint();
                });
            }
        } else if !self.auth_in_progress {
            self.browse.album_error = Some("Reconnect Spotify to browse this album.".into());
        } else {
            self.browse.album_loading = true;
        }
    }
}
