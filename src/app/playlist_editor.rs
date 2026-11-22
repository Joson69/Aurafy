use super::*;

const MAX_REMOVAL: usize = spotify::PLAYLIST_REMOVAL_LIMIT;

pub(crate) struct Editor {
    playlist: Playlist,
    owner: String,
    snapshot: String,
    songs: Vec<Song>,
    artist: String,
    selected: HashSet<String>,
    review: bool,
    applying: bool,
    error: Option<String>,
    worker: Option<Receiver<Reply>>,
}

struct Song {
    track: Track,
    copies: usize,
}

type Reply = (Option<TokenState>, Result<Outcome, String>);

enum Outcome {
    Loaded(spotify::PlaylistEditSnapshot),
    Removed(Vec<String>, usize),
}

#[derive(PartialEq)]
enum Action {
    KeepOpen,
    Close,
    Reload,
    Remove,
}

fn group_songs(tracks: Vec<Track>) -> Vec<Song> {
    let mut songs: Vec<Song> = Vec::new();
    let mut indices: HashMap<String, usize> = HashMap::new();
    for track in tracks {
        if spotify::validate_track_uri(&track.uri).is_err() {
            continue;
        }
        let index = *indices.entry(track.uri.clone()).or_insert(songs.len());
        if let Some(song) = songs.get_mut(index) {
            song.copies += 1;
        } else {
            songs.push(Song { track, copies: 1 });
        }
    }
    songs
}

impl AurafyApp {
    pub(crate) fn can_edit_playlist(&self, playlist: &Playlist) -> bool {
        self.is_connected()
            && playlist.items_accessible
            && self
                .user_profile
                .as_ref()
                .is_some_and(|profile| playlist.is_owned_by(&profile.id))
    }

    pub(crate) fn open_playlist_editor(&mut self, playlist: Playlist, ctx: &egui::Context) {
        if !self.can_edit_playlist(&playlist) || self.playlist_add_pending.is_some() {
            return;
        }
        let mut editor = Editor {
            owner: playlist.owner_id.clone(),
            playlist,
            snapshot: String::new(),
            songs: Vec::new(),
            artist: String::new(),
            selected: HashSet::new(),
            review: false,
            applying: false,
            error: None,
            worker: None,
        };
        let (id, owner) = (editor.playlist.id.clone(), editor.owner.clone());
        editor.worker = Some(self.playlist_editor_job(ctx, move |access| {
            spotify::load_playlist_edit_snapshot(access, &id, &owner).map(Outcome::Loaded)
        }));
        self.playlist_editor = Some(editor);
    }

    fn playlist_editor_job(
        &self,
        ctx: &egui::Context,
        work: impl FnOnce(&str) -> Result<Outcome, String> + Send + 'static,
    ) -> Receiver<Reply> {
        let token = self.token.clone();
        let client_id = self.config.client_id.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let reply = match token
                .ok_or_else(|| "Connect Spotify first.".to_owned())
                .and_then(|token| spotify::refresh_if_needed(token, &client_id))
            {
                Ok(token) => {
                    let outcome = work(&token.access_token);
                    (Some(token), outcome)
                }
                Err(error) => (None, Err(error)),
            };
            let _ = tx.send(reply);
            ctx.request_repaint();
        });
        rx
    }

    pub(super) fn poll_playlist_editor(&mut self, ctx: &egui::Context) {
        let Some(editor) = &mut self.playlist_editor else {
            return;
        };
        let Some(worker) = &editor.worker else {
            return;
        };
        let (token, outcome) = match worker.try_recv() {
            Ok(reply) => reply,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                (None, Err("The playlist request ended. Try again.".into()))
            }
        };
        let mut editor = self.playlist_editor.take().unwrap();
        if editor.applying {
            self.playlist_add_pending = None;
        }
        if !self.can_edit_playlist(&editor.playlist) {
            return;
        }
        if let Some(token) = token
            && self
                .token
                .as_ref()
                .is_some_and(|current| token.expires_at > current.expires_at)
        {
            self.accept_token(token);
        }
        editor.worker = None;
        editor.applying = false;
        match outcome {
            Ok(Outcome::Loaded(data)) => {
                editor.snapshot = data.version;
                editor.songs = group_songs(data.tracks);
            }
            Ok(Outcome::Removed(uris, copies)) => {
                self.playlist_removal_committed(&editor.playlist.id, &uris, ctx);
                self.set_status(format!("Removed {copies} from {}", editor.playlist.name));
                return;
            }
            Err(error) => editor.error = Some(error),
        }
        self.playlist_editor = Some(editor);
    }

    pub(super) fn draw_playlist_editor(&mut self, ctx: &egui::Context) {
        let Some(editor) = &mut self.playlist_editor else {
            return;
        };
        let shown = egui::Modal::new(Id::new("playlist-editor")).show(ctx, |ui| editor.draw(ui));
        let action = if shown.should_close() && !editor.applying {
            Action::Close
        } else {
            shown.inner
        };
        match action {
            Action::KeepOpen => {}
            Action::Close => self.playlist_editor = None,
            Action::Reload => {
                let playlist = editor.playlist.clone();
                self.open_playlist_editor(playlist, ctx);
            }
            Action::Remove => self.apply_playlist_editor(ctx),
        }
    }

    fn apply_playlist_editor(&mut self, ctx: &egui::Context) {
        let Some(editor) = &self.playlist_editor else {
            return;
        };
        if !editor.review
            || editor.worker.is_some()
            || self.playlist_add_pending.is_some()
            || !self.can_edit_playlist(&editor.playlist)
        {
            return;
        }
        let (uris, copies) = editor.removal();
        if uris.is_empty() || uris.len() > MAX_REMOVAL || editor.snapshot.is_empty() {
            return;
        }
        let (id, owner, version) = (
            editor.playlist.id.clone(),
            editor.owner.clone(),
            editor.snapshot.clone(),
        );
        self.playlist_add_pending = Some(id.clone());
        let worker = self.playlist_editor_job(ctx, move |access| {
            spotify::remove_playlist_selection(access, &id, &owner, &version, &uris)?;
            Ok(Outcome::Removed(uris, copies))
        });
        let editor = self.playlist_editor.as_mut().unwrap();
        editor.error = None;
        editor.applying = true;
        editor.worker = Some(worker);
    }
}

impl Editor {
    fn removal(&self) -> (Vec<String>, usize) {
        let mut uris = Vec::new();
        let mut copies = 0;
        for song in &self.songs {
            if self.selected.contains(&song.track.uri) {
                uris.push(song.track.uri.clone());
                copies += song.copies;
            }
        }
        (uris, copies)
    }

    fn draw(&mut self, ui: &mut Ui) -> Action {
        let colours = appearance::palette(ui.ctx());
        ui.set_width((ui.ctx().content_rect().width() - 48.0).clamp(280.0, 570.0));
        ui.heading(if self.review {
            "Review removal"
        } else {
            "Edit playlist"
        });
        ui.add(
            egui::Label::new(RichText::new(&self.playlist.name).color(colours.muted)).truncate(),
        );
        ui.add_space(8.0);
        if let Some(error) = &self.error {
            ui.label(error);
            if ui.button("Reload playlist").clicked() {
                return Action::Reload;
            }
        }
        if self.snapshot.is_empty() {
            if self.worker.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading all playlist tracks…");
                });
            }
            return if ui.button("Close").clicked() {
                Action::Close
            } else {
                Action::KeepOpen
            };
        }

        if !self.review {
            ui.add(
                TextEdit::singleline(&mut self.artist)
                    .hint_text("Filter by artist")
                    .desired_width(f32::INFINITY),
            );
        }
        let query = self.artist.trim().to_lowercase();
        let visible: Vec<_> = self
            .songs
            .iter()
            .filter(|song| {
                if self.review {
                    self.selected.contains(&song.track.uri)
                } else {
                    query.is_empty() || song.track.artist.to_lowercase().contains(&query)
                }
            })
            .collect();
        if !self.review {
            let slots = MAX_REMOVAL.saturating_sub(self.selected.len());
            let matches = visible
                .iter()
                .filter(|song| !self.selected.contains(&song.track.uri))
                .count();
            let count = slots.min(matches);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        count > 0,
                        egui::Button::new(format!("Select {count} matches")),
                    )
                    .clicked()
                {
                    let eligible: Vec<_> = visible
                        .iter()
                        .filter(|song| !self.selected.contains(&song.track.uri))
                        .take(count)
                        .map(|song| song.track.uri.clone())
                        .collect();
                    self.selected.extend(eligible);
                }
                if ui
                    .add_enabled(
                        !self.selected.is_empty(),
                        egui::Button::new("Clear selection"),
                    )
                    .clicked()
                {
                    self.selected.clear();
                }
            });
        }
        if visible.is_empty() {
            ui.label(if self.songs.is_empty() {
                "No editable songs in this playlist."
            } else {
                "No songs match that artist."
            });
        } else {
            ScrollArea::vertical()
                .id_salt(("playlist-editor-songs", self.review, &self.artist))
                .max_height((ui.ctx().content_rect().height() * 0.48).min(408.0))
                .show_rows(ui, 42.0, visible.len(), |ui, rows| {
                    for index in rows {
                        let song = visible[index];
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), 42.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                if !self.review {
                                    let mut selected = self.selected.contains(&song.track.uri);
                                    let enabled = selected || self.selected.len() < MAX_REMOVAL;
                                    if ui
                                        .add_enabled(
                                            enabled,
                                            egui::Checkbox::new(&mut selected, ""),
                                        )
                                        .on_hover_text(&song.track.name)
                                        .changed()
                                    {
                                        if selected {
                                            self.selected.insert(song.track.uri.clone());
                                        } else {
                                            self.selected.remove(&song.track.uri);
                                        }
                                    }
                                }
                                ui.vertical(|ui| {
                                    ui.add(egui::Label::new(&song.track.name).truncate());
                                    let detail = if song.copies > 1 {
                                        format!("{} · {} entries", song.track.artist, song.copies)
                                    } else {
                                        song.track.artist.clone()
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(detail).color(colours.muted),
                                        )
                                        .truncate(),
                                    );
                                });
                            },
                        );
                    }
                });
        }
        let copies: usize = self
            .songs
            .iter()
            .filter(|song| self.selected.contains(&song.track.uri))
            .map(|song| song.copies)
            .sum();
        ui.separator();
        if self.applying {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Removing selected tracks…");
            });
            return Action::KeepOpen;
        }
        ui.label(format!(
            "{} songs selected · {copies} playlist entries",
            self.selected.len()
        ));
        if self.review {
            ui.label("Confirm to remove every playlist entry shown above.");
        } else if self.selected.len() == MAX_REMOVAL {
            ui.label("Spotify allows 100 distinct songs per removal.");
        }
        let mut action = Action::KeepOpen;
        ui.horizontal_wrapped(|ui| {
            if self.review {
                if ui.button("Back to selection").clicked() {
                    self.review = false;
                    self.error = None;
                }
                if ui.button(format!("Remove {copies} entries")).clicked() {
                    action = Action::Remove;
                }
            } else {
                if ui.button("Cancel").clicked() {
                    action = Action::Close;
                }
                if ui
                    .add_enabled(copies > 0, crate::ui::primary("Review removal", ui.ctx()))
                    .clicked()
                {
                    self.review = true;
                }
            }
        });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_keeps_original_order_and_counts_repeated_tracks() {
        let first = Track {
            uri: format!("spotify:track:{:022}", 1),
            ..Default::default()
        };
        let second = Track {
            uri: format!("spotify:track:{:022}", 2),
            ..Default::default()
        };
        let songs = group_songs(vec![
            first.clone(),
            Track::default(),
            second.clone(),
            first.clone(),
        ]);
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].track.uri, first.uri);
        assert_eq!(songs[0].copies, 2);
        assert_eq!(songs[1].track.uri, second.uri);
        assert_eq!(songs[1].copies, 1);
    }
}
