use super::*;

const MODEL: &str = "qwen3:4b";

#[derive(Default)]
pub(crate) struct Ask {
    pub(crate) open: bool,
    input: String,
    messages: Vec<(bool, String)>,
    matches: Option<(String, Query, Option<String>)>,
    proposal: Option<Proposal>,
    pending: Option<Receiver<Result<Answer, String>>>,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Query {
    Artist { artist: String, shared_only: bool },
    Count,
    RemoveLast,
    Clarify { question: String },
}

struct Proposal {
    playlist: Playlist,
    owner: String,
    version: String,
    tracks: Vec<Track>,
}

enum Answer {
    Text(String),
    Matches(String, Query, Option<String>, String),
    Preview(Proposal),
    Applied(Playlist),
}

fn interpret(prompt: &str) -> Result<Query, String> {
    let normalized = prompt
        .trim()
        .trim_end_matches(['.', '?', '!'])
        .to_lowercase();
    if [
        "remove that feature",
        "remove those",
        "remove these matches",
    ]
    .contains(&normalized.as_str())
    {
        return Ok(Query::RemoveLast);
    }
    if [
        "how many songs are here",
        "how many tracks are in this playlist",
    ]
    .contains(&normalized.as_str())
    {
        return Ok(Query::Count);
    }
    if let Some(artist) = normalized.strip_prefix("which songs here feature ") {
        return Ok(Query::Artist {
            artist: artist.into(),
            shared_only: true,
        });
    }
    let response: serde_json::Value = reqwest::blocking::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(35))
        .build()
        .map_err(|_| "Could not start the local model request.")?
        .post("http://127.0.0.1:11434/api/chat")
        .json(&serde_json::json!({
          "model": MODEL, "stream": false, "think": false, "format": "json",
          "messages": [
            {"role":"system","content":"Route a music assistant request. Return one JSON object only. Supported: {action:artist,artist:string,shared_only:boolean} for finding songs credited to an artist (shared_only for collaborations/features); {action:count} for counting tracks; {action:remove_last} only for removing the immediately previous search results, never for a new artist or playlist; {action:clarify,question:string} for ambiguity or unsupported requests. The scope is the visible collection. If the user specifies another collection, ask them to open it. Never claim an edit happened."},
            {"role":"user","content":prompt}
          ]
        }))
        .send()
        .map_err(|_| "Start Ollama to use free-form chat. Try “Which songs here feature The Weeknd?” without it.")?
        .error_for_status()
        .map_err(|_| "The local model is unavailable. In Ollama, install qwen3:4b and try again.")?
        .json()
        .map_err(|_| "Could not read the local model reply.")?;
    serde_json::from_str(response["message"]["content"].as_str().unwrap_or_default()).map_err(
        |_| "I couldn't interpret that request. Try asking for an artist or a track count.".into(),
    )
}

fn artist_matches(track: &Track, artist: &str, shared_only: bool) -> bool {
    let credits: Vec<_> = if track.artist_names.is_empty() {
        track.artist.split(',').collect()
    } else {
        track.artist_names.iter().map(String::as_str).collect()
    };
    (!shared_only || credits.len() > 1)
        && credits
            .iter()
            .any(|credit| credit.trim().to_lowercase() == artist)
}

impl AurafyApp {
    fn ask_work(
        &mut self,
        ctx: &egui::Context,
        work: impl FnOnce() -> Result<Answer, String> + Send + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        self.ask.pending = Some(rx);
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(work());
            ctx.request_repaint();
        });
    }

    fn submit_ask(&mut self, ctx: &egui::Context) {
        let prompt = self.ask.input.trim().to_owned();
        if prompt.is_empty() || self.ask.pending.is_some() {
            return;
        }
        self.ask.input.clear();
        self.ask.messages.push((true, prompt.clone()));
        self.ask.proposal = None;
        let playlist = (self.page == Page::Library)
            .then(|| self.selected_playlist.clone())
            .flatten();
        let owner = self
            .user_profile
            .as_ref()
            .map(|profile| profile.id.clone())
            .unwrap_or_default();
        let token = self.token.clone();
        let client_id = self.config.client_id.clone();
        let queue = self.queue.clone();
        let previous = self.ask.matches.clone();
        self.ask_work(ctx, move || {
            let query = interpret(&prompt)?;
            if matches!(query, Query::RemoveLast) && !prompt.to_lowercase().contains("remove") {
                return Err("Say which tracks you want to remove, or ask a question about this playlist.".into());
            }
            if let Query::Clarify { question } = query {
                return Ok(Answer::Text(question.chars().take(300).collect()));
            }
            let Some(playlist) = playlist else {
                return match query {
                    Query::Count => Ok(Answer::Text(format!("The current queue shows {} tracks.", queue.len()))),
                    Query::Artist { artist, shared_only } => Ok(Answer::Text(format_tracks(
                        &queue.into_iter().filter(|t| artist_matches(t, &artist.trim().to_lowercase(), shared_only)).collect::<Vec<_>>(),
                        "the current queue"
                    ))),
                    _ => Err("Open the playlist you want to edit and find the tracks there first.".into()),
                };
            };
            let token = spotify::refresh_if_needed(token.ok_or("Connect Spotify first.")?, &client_id)?;
            let access = &token.access_token;
            if let Query::Count = query {
                let tracks = spotify::playlist_tracks(access, &playlist.id, 20)?;
                return Ok(Answer::Text(format!("Found {} playable tracks in the first 1,000 playlist entries of {}.", tracks.len(), playlist.name)));
            }
            let (criteria, preview, reviewed_version) = match query {
                Query::RemoveLast => {
                    let Some((id, criteria, version)) = previous else {
                        return Err("Find the songs in this playlist first so I know which tracks you mean.".into());
                    };
                    if id != playlist.id { return Err("Open the playlist from the previous result first.".into()); }
                    (criteria, true, version)
                }
                other => (other, false, None),
            };
            let version = if preview || playlist.is_owned_by(&owner) {
                Some(spotify::playlist_version(access, &playlist.id, &owner)?)
            } else { None };
            if preview && reviewed_version != version {
                return Err("The playlist changed since the results were shown. Run the search again.".into());
            }
            let Query::Artist { ref artist, shared_only } = criteria else { return Err("Find an artist’s tracks before previewing removal.".into()); };
            let artist = artist.trim().to_lowercase();
            if artist.is_empty() { return Err("Which artist should I look for?".into()); }
            let mut seen = HashSet::new();
            let tracks: Vec<_> = spotify::playlist_tracks(access, &playlist.id, 19)?
                .into_iter().filter(|track| artist_matches(track, &artist, shared_only)).collect();
            if let Some(version) = &version
                && spotify::playlist_version(access, &playlist.id, &owner)? != *version
            {
                return Err("The playlist changed during the scan. Try again.".into());
            }
            if preview {
                let tracks: Vec<_> = tracks.into_iter().filter(|t| seen.insert(t.uri.clone())).collect();
                if tracks.is_empty() || tracks.len() > 100 { return Err("A removal preview needs between 1 and 100 distinct matches.".into()); }
                // URI removals affect every occurrence, including entries outside a capped scan.
                if playlist.total_items >= 1000 { return Err("This playlist is too large for a complete removal review.".into()); }
                return Ok(Answer::Preview(Proposal { playlist, owner, version: version.unwrap(), tracks }));
            }
            let result = format_tracks(&tracks, &playlist.name);
            Ok(Answer::Matches(playlist.id, criteria, version, result))
        });
    }

    pub(super) fn draw_ask(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.ask.pending {
            let result = match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("The request ended early. Try again.".into()))
                }
            };
            if let Some(result) = result {
                self.ask.pending = None;
                match result {
                    Ok(Answer::Matches(id, criteria, version, text)) => {
                        self.ask.matches = Some((id, criteria, version));
                        self.ask.messages.push((false, text));
                    }
                    Ok(Answer::Preview(proposal)) => self.ask.proposal = Some(proposal),
                    Ok(Answer::Applied(playlist)) => {
                        self.playlist_membership.remove(&playlist.id);
                        self.membership_attempted.remove(&playlist.id);
                        self.load_library(ctx, true);
                        if self.page == Page::Library
                            && self
                                .selected_playlist
                                .as_ref()
                                .is_some_and(|active| active.id == playlist.id)
                        {
                            self.open_playlist(playlist, ctx);
                        }
                        self.ask
                            .messages
                            .push((false, "Removed the reviewed songs.".into()));
                    }
                    Ok(Answer::Text(text)) | Err(text) => self.ask.messages.push((false, text)),
                }
            }
        }
        if !self.ask.open {
            return;
        }
        let screen = ctx.content_rect();
        let right = if self.show_queue { 300.0 } else { 18.0 };
        let player_top = self
            .player_layout
            .map_or(screen.bottom() - 80.0, |layout| layout.bounds.top());
        let bottom = player_top - 14.0;
        let width = (screen.width() - right - 20.0).clamp(250.0, 440.0);
        let height = (bottom - screen.top() - 96.0).clamp(230.0, 515.0);
        let position = Pos2::new(screen.right() - right - width, bottom - height);
        let colours = appearance::palette(ctx);
        let busy = self.ask.pending.is_some();
        let playlist = self
            .selected_playlist
            .as_ref()
            .filter(|_| self.page == Page::Library);
        let context = playlist
            .map_or("Current queue", |playlist| playlist.name.as_str())
            .to_owned();
        let editable = playlist.is_some_and(|playlist| {
            self.user_profile
                .as_ref()
                .is_some_and(|owner| playlist.is_owned_by(&owner.id))
                && self
                    .ask
                    .matches
                    .as_ref()
                    .is_some_and(|(id, _, version)| id == &playlist.id && version.is_some())
        });
        let mut submit = false;
        let mut confirm = false;
        let mut cancel = false;
        egui::Area::new(Id::new("ask-aurafy-panel"))
            .order(Order::Foreground)
            .fixed_pos(position)
            .movable(false)
            .show(ctx, |ui| {
                ui.set_width(width);
                Frame::new()
                    .fill(colours.panel)
                    .stroke(Stroke::new(1.0, colours.border))
                    .corner_radius(15)
                    .shadow(egui::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(72) })
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        let inner_width = width - 2.0;
                        Frame::new()
                            .fill(colours.panel2.lerp_to_gamma(colours.green, 0.13))
                            .corner_radius(CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 })
                            .inner_margin(Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                ui.set_width(inner_width - 28.0);
                                ui.horizontal(|ui| {
                                    ui.allocate_ui_with_layout(Vec2::new((ui.available_width() - 124.0).max(80.0), 42.0), Layout::top_down(Align::Min), |ui| {
                                        ui.label(RichText::new("Ask Aurafy").size(17.0).strong().color(colours.text));
                                        ui.add(egui::Label::new(RichText::new(&context).size(12.0).color(colours.muted)).truncate());
                                    });
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.spacing_mut().item_spacing.x = 6.0;
                                        if ui.add_sized([28.0, 28.0], egui::Button::new("×").frame(false)).on_hover_text("Close chat").clicked() {
                                            self.ask.open = false;
                                        }
                                        if ui.add_enabled(!busy, egui::Button::new("New chat")).clicked() {
                                            self.ask = Ask { open: true, ..Ask::default() };
                                        }
                                    });
                                });
                            });

                        let body_height = (height - 125.0).max(105.0);
                        ScrollArea::vertical()
                            .id_salt("ask-transcript")
                            .stick_to_bottom(!self.ask.messages.is_empty())
                            .auto_shrink([false, false])
                            .max_height(body_height)
                            .show(ui, |ui| {
                                Frame::new().inner_margin(Margin::same(14)).show(ui, |ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 12.0);
                                    if self.ask.messages.is_empty() {
                                        ask_bubble(ui, &colours, false, "Ask me about the playlist you have open or what’s coming up in your queue.");
                                        ui.vertical(|ui| {
                                            for (label, prompt) in [
                                                ("Count tracks", "How many songs are here?"),
                                                ("Find The Weeknd features", "Which songs here feature The Weeknd?"),
                                            ] {
                                                if ui.button(label).clicked() {
                                                    self.ask.input = prompt.into();
                                                    submit = true;
                                                }
                                            }
                                        });
                                    }
                                    for (user, message) in &self.ask.messages {
                                        ask_bubble(ui, &colours, *user, message);
                                    }
                                    if busy {
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label(RichText::new("Working on it…").color(colours.muted));
                                        });
                                    } else if let Some(proposal) = &self.ask.proposal {
                                        Frame::new().fill(colours.panel2).corner_radius(10)
                                            .inner_margin(Margin::same(12)).show(ui, |ui| {
                                                ui.label(RichText::new(format!("Remove from {}?", proposal.playlist.name)).strong());
                                                ui.label(RichText::new("All occurrences of these songs will be removed.").size(12.0).color(colours.muted));
                                                for track in &proposal.tracks {
                                                    ui.label(format!("{} · {}", track.name, track.artist));
                                                }
                                                ui.horizontal_wrapped(|ui| {
                                                    confirm = ui.add(crate::ui::primary("Confirm removal", ctx)).clicked();
                                                    cancel = ui.button("Cancel").clicked();
                                                });
                                            });
                                    } else if editable && !self.ask.messages.is_empty()
                                        && ui.button("Review removal…").clicked()
                                    {
                                        self.ask.input = "Remove these matches".into();
                                        submit = true;
                                    }
                                });
                            });
                        Frame::new().fill(colours.panel2)
                            .corner_radius(CornerRadius { nw: 0, ne: 0, sw: 14, se: 14 })
                            .inner_margin(Margin::symmetric(12, 10)).show(ui, |ui| {
                                ui.set_width(inner_width - 24.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    let input_width = (ui.available_width() - 80.0).max(70.0);
                                    let input = ui.add_sized([input_width, 35.0],
                                        TextEdit::singleline(&mut self.ask.input)
                                            .hint_text("Ask about this music…")
                                            .char_limit(500)
                                            .margin(Margin::symmetric(10, 8)));
                                    let enter = input.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                                    let send = ui.add_enabled(
                                        !busy && !self.ask.input.trim().is_empty(),
                                        crate::ui::primary("Send", ctx).min_size(Vec2::new(68.0, 35.0)),
                                    );
                                    submit |= !busy && (enter || send.clicked());
                                });
                            });
                    });
            });
        if submit {
            self.submit_ask(ctx);
        }
        if cancel {
            self.ask.proposal = None;
        }
        if confirm && let Some(proposal) = self.ask.proposal.take() {
            let token = self.token.clone();
            let id = self.config.client_id.clone();
            self.ask_work(ctx, move || {
                let token =
                    spotify::refresh_if_needed(token.ok_or("Connect Spotify first.")?, &id)?;
                let uris: Vec<_> = proposal.tracks.into_iter().map(|track| track.uri).collect();
                spotify::remove_reviewed_tracks(
                    &token.access_token,
                    &proposal.playlist.id,
                    &proposal.owner,
                    &proposal.version,
                    &uris,
                )?;
                Ok(Answer::Applied(proposal.playlist))
            });
        }
        if self.ask.messages.len() > 24 {
            let excess = self.ask.messages.len() - 24;
            self.ask.messages.drain(..excess);
        }
    }
}

fn ask_bubble(ui: &mut Ui, colours: &appearance::Palette, user: bool, message: &str) {
    let bubble_width = ui.available_width() * 0.85;
    ui.with_layout(
        if user {
            Layout::right_to_left(Align::TOP)
        } else {
            Layout::left_to_right(Align::TOP)
        },
        |ui| {
            Frame::new()
                .fill(if user {
                    colours.selected
                } else {
                    colours.panel2
                })
                .corner_radius(11)
                .inner_margin(Margin::symmetric(12, 9))
                .show(ui, |ui| {
                    ui.set_max_width(bubble_width - 24.0);
                    ui.add(egui::Label::new(RichText::new(message).color(colours.text)).wrap());
                });
        },
    );
}

fn format_tracks(tracks: &[Track], scope: &str) -> String {
    if tracks.is_empty() {
        return format!("No matching credits in {scope}.");
    }
    let rows = tracks
        .iter()
        .take(15)
        .map(|t| format!("{} · {}", t.name, t.artist))
        .collect::<Vec<_>>()
        .join("\n");
    let suffix = if tracks.len() > 15 {
        "\nShowing the first 15."
    } else {
        ""
    };
    format!("{} matches in {scope}\n{rows}{suffix}", tracks.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_search_identifies_only_shared_exact_credits() {
        let solo = Track {
            artist_names: vec!["The Weeknd".into()],
            ..Track::default()
        };
        let shared = Track {
            artist_names: vec!["Metro Boomin".into(), "The Weeknd".into()],
            ..Track::default()
        };
        assert!(!artist_matches(&solo, "the weeknd", true));
        assert!(artist_matches(&shared, "the weeknd", true));
        assert!(!artist_matches(&shared, "weeknd", false));
        assert!(matches!(
            interpret("Remove that feature."),
            Ok(Query::RemoveLast)
        ));
    }
}

#[cfg(test)]
#[test]
fn floating_panel_leaves_queue_and_player_clear() {
    for size in [
        Vec2::new(1080.0, 680.0),
        Vec2::new(1440.0, 740.0),
        Vec2::new(1920.0, 1020.0),
    ] {
        for queue in [false, true] {
            let ctx = egui::Context::default();
            let mut app = AurafyApp::build(&ctx, AppConfig::default());
            app.ask.open = true;
            app.show_queue = queue;
            let screen = Rect::from_min_size(Pos2::ZERO, size);
            for _ in 0..3 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        app.draw_ask(ctx);
                    },
                );
            }
            let panel = ctx
                .memory(|memory| memory.area_rect(Id::new("ask-aurafy-panel")))
                .expect("floating chat rendered");
            assert!(screen.contains_rect(panel), "{panel:?} exceeds {screen:?}");
            assert!(
                panel.bottom() <= screen.bottom() - 80.0,
                "{panel:?} overlaps player"
            );
            if queue {
                assert!(
                    panel.right() <= screen.right() - 284.0,
                    "{panel:?} overlaps Queue"
                );
            }
        }
    }
}
