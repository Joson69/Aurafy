use super::*;

impl AurafyApp {
    fn draw_artist(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let image = self
            .browse
            .artist
            .as_ref()
            .and_then(|artist| artist.image_url.clone());
        let name = self.artist_name.clone();
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(112.0), Sense::hover());
            self.art_rounded(ui, rect, image.as_deref(), &name, ctx, 255);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("ARTIST")
                        .size(11.0)
                        .strong()
                        .color(colours.muted),
                );
                ui.label(RichText::new(&name).font(player_font(34.0, true)));
                if !self.artist_tracks.is_empty() {
                    ui.label(
                        RichText::new(format!("{} top songs", self.artist_tracks.len()))
                            .size(12.0)
                            .color(colours.muted),
                    );
                }
            });
        });
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            let has_songs = !self.artist_tracks.is_empty();
            if has_songs {
                let (play_rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                let on_current = self
                    .artist_tracks
                    .iter()
                    .any(|t| !t.uri.is_empty() && t.uri == self.current_track.uri);
                let playing = self.is_playing && on_current;
                if playlist_play_at(ui, play_rect, (&name, "artist-header-play"), playing).clicked()
                {
                    if playing {
                        self.send_playback_action("pause", ctx);
                    } else if on_current {
                        self.send_playback_action("resume", ctx);
                    } else {
                        let track = self.artist_tracks[0].clone();
                        self.play_track(track, ctx);
                    }
                }
                ui.add_space(4.0);
            }
            if has_songs && action_button(ui, "Shuffle", Some(Icon::Shuffle), false).clicked() {
                let tracks = self.artist_tracks.clone();
                self.play_tracks_shuffled(tracks, ctx);
            }
            if has_songs && action_button(ui, "Queue all", Some(Icon::Queue), false).clicked() {
                let tracks = self.artist_tracks.clone();
                self.queue_tracks_batch(tracks, ctx);
            }
            if !self.browse.artist_id.is_empty()
                && action_button(ui, "Open in Spotify", None, false).clicked()
            {
                let url = format!("https://open.spotify.com/artist/{}", self.browse.artist_id);
                if let Err(err) = browser::open_url(&url) {
                    self.set_status(err);
                }
            }
        });
        ui.add_space(14.0);
        let lang = self.config.language;
        ui.horizontal(|ui| {
            if ui
                .add(pill(
                    i18n::tr(lang, i18n::Text::Songs),
                    !self.browse.releases,
                    ctx,
                ))
                .clicked()
            {
                self.browse.releases = false;
            }
            if ui
                .add(pill("Albums & singles", self.browse.releases, ctx))
                .clicked()
            {
                self.browse.releases = true;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let more = if self.browse.releases {
                    self.browse.next_albums.is_some()
                } else {
                    self.browse.next_songs.is_some()
                };
                if more
                    && ui
                        .add_enabled_ui(!self.browse.more_loading, |ui| {
                            compact_pill_button(ui, "Load more", None, false)
                        })
                        .inner
                        .clicked()
                {
                    self.load_more_artist(ctx);
                }
                if self.browse.more_loading || self.artist_loading {
                    ui.spinner();
                }
            });
        });
        ui.add_space(12.0);
        if let Some(error) = self.artist_error.clone() {
            self.error_notice(ui, &error);
            if action_button(ui, "Try again", Some(Icon::Refresh), false).clicked() {
                self.open_artist(self.artist_name.clone(), ctx);
            }
        }
        if self.browse.releases {
            let albums = self.browse.albums.clone();
            if albums.is_empty() && !self.artist_loading {
                ui.label(RichText::new("No releases available.").color(colours.muted));
            }
            let columns = ((ui.available_width() + 12.0) / 170.0).floor().max(1.0) as usize;
            let width = (ui.available_width() - (columns - 1) as f32 * 12.0) / columns as f32;
            ScrollArea::vertical()
                .id_salt(("artist-releases", &self.browse.artist_id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for row in albums.chunks(columns) {
                        ui.horizontal(|ui| {
                            for album in row {
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(width, width + 58.0),
                                    Sense::click(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    10.0,
                                    if response.hovered() {
                                        colours.hover
                                    } else {
                                        colours.panel2
                                    },
                                );
                                let art = Rect::from_min_size(
                                    rect.min + Vec2::splat(8.0),
                                    Vec2::splat(width - 16.0),
                                );
                                self.art(ui, art, album.artwork_url.as_deref(), &album.name, ctx);
                                text_with_font(
                                    ui,
                                    Pos2::new(rect.left() + 10.0, art.bottom() + 10.0),
                                    &album.name,
                                    player_font(13.0, true),
                                    colours.text,
                                    width - 20.0,
                                );
                                let year =
                                    album.release_date.get(..4).unwrap_or(&album.release_date);
                                text(
                                    ui,
                                    Pos2::new(rect.left() + 10.0, art.bottom() + 31.0),
                                    &format!("{year} · {}", album.kind),
                                    11.0,
                                    colours.muted,
                                    width - 20.0,
                                );
                                if response
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    self.open_album(album.as_track(), ctx);
                                }
                            }
                        });
                    }
                });
        } else if !self.artist_tracks.is_empty() {
            self.draw_track_table(ui, ctx, self.artist_tracks.clone());
        } else if !self.artist_loading {
            ui.label(
                RichText::new(
                    "No songs available. Browse Albums & singles for this artist's releases.",
                )
                .color(colours.muted),
            );
        }
    }

    fn draw_album(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let seed = self.browse.album_seed.clone().unwrap_or_default();
        let album = self
            .browse
            .album
            .as_ref()
            .map(|details| details.album.clone())
            .unwrap_or_else(|| catalog::Album {
                id: seed.album_id.clone(),
                name: seed.album.clone(),
                artwork_url: seed.artwork_url.clone(),
                artist_names: seed.artist_names.clone(),
                artist_ids: seed.artist_ids.clone(),
                ..Default::default()
            });
        let total_ms: u64 = self
            .browse
            .album
            .as_ref()
            .map(|d| d.tracks.iter().map(|t| t.duration_ms).sum())
            .unwrap_or(0);
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(132.0), Sense::hover());
            self.art(ui, rect, album.artwork_url.as_deref(), &album.name, ctx);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(if album.kind.is_empty() {
                        "ALBUM"
                    } else {
                        match album.kind.as_str() {
                            "single" => "SINGLE",
                            "compilation" => "COMPILATION",
                            _ => "ALBUM",
                        }
                    })
                    .size(11.0)
                    .strong()
                    .color(colours.muted),
                );
                ui.label(
                    RichText::new(if album.name.is_empty() {
                        "Album"
                    } else {
                        &album.name
                    })
                    .font(player_font(30.0, true)),
                );
                let (artists, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::hover());
                let mut credits = album.as_track();
                credits.uri = format!("spotify:album:{}", album.id);
                self.artist_links(ui, ctx, &credits, artists.min, 12.5, artists.width());
                let mut meta = Vec::new();
                if !album.release_date.is_empty() {
                    meta.push(
                        album
                            .release_date
                            .get(..4)
                            .unwrap_or(&album.release_date)
                            .to_owned(),
                    );
                }
                if album.total_tracks > 0 {
                    meta.push(format!("{} songs", album.total_tracks));
                }
                if total_ms > 0 {
                    meta.push(format!("{} min", (total_ms / 60_000).max(1)));
                }
                if !meta.is_empty() {
                    ui.label(
                        RichText::new(meta.join(" · "))
                            .size(12.0)
                            .color(colours.muted),
                    );
                }
            });
        });
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            let tracks = self
                .browse
                .album
                .as_ref()
                .map(|d| d.tracks.clone())
                .unwrap_or_default();
            if !tracks.is_empty() {
                let (play_rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
                let album_uri = format!("spotify:album:{}", album.id);
                let queue_clean = !self.suppress_upstream_queue
                    && !self.queue.first().is_some_and(|next| {
                        !next.uri.is_empty() && !tracks.iter().any(|t| t.uri == next.uri)
                    });
                let on_current = self.active_collection_uri.as_deref() == Some(album_uri.as_str())
                    || (queue_clean
                        && tracks
                            .iter()
                            .any(|t| !t.uri.is_empty() && t.uri == self.current_track.uri));
                let playing = self.is_playing && on_current;
                if playlist_play_at(ui, play_rect, (&album.id, "album-header-play"), playing)
                    .clicked()
                {
                    if playing {
                        self.send_playback_action("pause", ctx);
                    } else if on_current {
                        self.send_playback_action("resume", ctx);
                    } else {
                        let first = tracks[0].clone();
                        self.play_table_track(first, 0, ctx);
                    }
                }
                ui.add_space(4.0);
            }
            if !tracks.is_empty()
                && action_button(ui, "Shuffle", Some(Icon::Shuffle), false).clicked()
            {
                self.play_tracks_shuffled(tracks.clone(), ctx);
            }
            if !tracks.is_empty()
                && action_button(ui, "Queue all", Some(Icon::Queue), false).clicked()
            {
                self.queue_tracks_batch(tracks.clone(), ctx);
            }
            if !album.id.is_empty() && action_button(ui, "Open in Spotify", None, false).clicked() {
                let url = format!("https://open.spotify.com/album/{}", album.id);
                if let Err(err) = browser::open_url(&url) {
                    self.set_status(err);
                }
            }
        });
        ui.add_space(14.0);
        if self.browse.album_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading album…");
            });
        }
        if let Some(error) = self.browse.album_error.clone() {
            self.error_notice(ui, &error);
            if action_button(ui, "Try again", Some(Icon::Refresh), false).clicked() {
                self.open_album(seed, ctx);
            }
        }
        if let Some(details) = self.browse.album.clone() {
            self.draw_track_table(ui, ctx, details.tracks);
        }
    }

    pub(crate) fn draw_main(&mut self, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        CentralPanel::default()
            .frame(Frame::new().fill(colours.bg).inner_margin(Margin::ZERO))
            .show(ctx, |ui| match self.page {
                Page::Lyrics => {
                    Frame::new()
                        .inner_margin(Margin::symmetric(28, 18))
                        .show(ui, |ui| self.draw_lyrics(ui, ctx));
                }
                Page::Artist => {
                    Frame::new()
                        .inner_margin(Margin {
                            left: 28,
                            right: 28,
                            top: 18,
                            bottom: 0,
                        })
                        .show(ui, |ui| self.draw_artist(ui, ctx));
                }
                Page::Album => {
                    Frame::new()
                        .inner_margin(Margin {
                            left: 28,
                            right: 28,
                            top: 18,
                            bottom: 0,
                        })
                        .show(ui, |ui| self.draw_album(ui, ctx));
                }
                Page::Search => {
                    Frame::new()
                        .inner_margin(Margin {
                            left: 28,
                            right: 28,
                            top: 18,
                            bottom: 0,
                        })
                        .show(ui, |ui| self.draw_search_results(ui, ctx));
                }
                Page::Library if self.selected_playlist.is_some() => {
                    Frame::new()
                        .inner_margin(Margin {
                            left: 28,
                            right: 28,
                            top: 18,
                            bottom: 0,
                        })
                        .show(ui, |ui| self.draw_library(ui, ctx));
                }
                Page::Liked | Page::Recent => {
                    Frame::new()
                        .inner_margin(Margin {
                            left: 28,
                            right: 28,
                            top: 18,
                            bottom: 0,
                        })
                        .show(ui, |ui| {
                            let liked = self.page == Page::Liked;
                            let lang = self.config.language;
                            heading(
                                ui,
                                if liked {
                                    i18n::tr(lang, i18n::Text::LikedSongs)
                                } else {
                                    i18n::tr(lang, i18n::Text::RecentlyPlayed)
                                },
                                if liked {
                                    i18n::tr(lang, i18n::Text::LikedSubtitle)
                                } else {
                                    i18n::tr(lang, i18n::Text::RecentSubtitle)
                                },
                            );
                            if let Some(error) = self.library_error.clone().filter(|_| liked) {
                                self.error_notice(ui, &error);
                            }
                            let tracks = if self.is_demo_mode() {
                                self.demo_tracks.clone()
                            } else if liked {
                                self.saved_tracks.clone()
                            } else {
                                self.recent_tracks.clone()
                            };
                            self.draw_track_table(ui, ctx, tracks);
                        });
                }
                page => {
                    ScrollArea::vertical()
                        .id_salt((
                            match page {
                                Page::Home => "home",
                                Page::Library => "library",
                                Page::Profile => "profile",

                                _ => "settings",
                            },
                            if page == Page::Settings {
                                self.settings_tab as u8
                            } else {
                                0
                            },
                        ))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            Frame::new()
                                .inner_margin(Margin {
                                    left: 28,
                                    right: 28,
                                    top: 18,
                                    bottom: 0,
                                })
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    match page {
                                        Page::Home => self.draw_home(ui, ctx),
                                        Page::Library => self.draw_library(ui, ctx),
                                        Page::Profile => self.draw_profile(ui, ctx),
                                        Page::Settings => self.draw_settings_page(ui, ctx),

                                        _ => {}
                                    }
                                    ui.add_space(24.0);
                                });
                        });
                }
            });

        self.draw_toast(ctx);
    }

    fn draw_home(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let lang = self.config.language;
        heading(ui, i18n::tr(lang, i18n::Text::Home), "");
        if self.is_demo_mode() {
            card(ui.ctx()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Welcome to Aurafy").size(20.0).strong());
                ui.label(
                    RichText::new("Explore the sample library in silent demo mode. Set up Spotify to load your own music.")
                        .color(colours.muted),
                );
                ui.add_space(8.0);
                if ui.add(primary("Set up Spotify", ui.ctx())).clicked() {
                    self.open_spotify_setup(ctx);
                }
            });
            ui.add_space(16.0);
        }
        let count = if self.is_demo_mode() {
            self.demo_tracks.len()
        } else {
            self.saved_tracks.len()
        };
        let width = (ui.available_width() - 24.0) / 3.0;
        ui.horizontal(|ui| {
            for (kind, title, sub, page, color) in [
                (
                    Icon::Heart,
                    i18n::tr(lang, i18n::Text::LikedSongs),
                    if self.library_loading {
                        "Loading saved songs".into()
                    } else if count == 0 {
                        "Saved songs".into()
                    } else {
                        format!("{count} saved songs")
                    },
                    Page::Liked,
                    Color32::from_rgb(137, 124, 209),
                ),
                (
                    Icon::Clock,
                    i18n::tr(lang, i18n::Text::RecentlyPlayed),
                    "Listening history".into(),
                    Page::Recent,
                    Color32::from_rgb(115, 181, 205),
                ),
                (
                    Icon::Library,
                    i18n::tr(lang, i18n::Text::HomePlaylists),
                    format!("{} playlists", self.playlists.len()),
                    Page::Library,
                    colours.green,
                ),
            ] {
                let (r, response) = ui.allocate_exact_size(Vec2::new(width, 76.0), Sense::click());
                ui.painter().rect_filled(
                    r,
                    10.0,
                    if response.hovered() {
                        colours.hover
                    } else {
                        colours.panel
                    },
                );
                let badge = Rect::from_min_size(r.min + Vec2::new(14.0, 17.0), Vec2::splat(42.0));
                ui.painter()
                    .rect_filled(badge, 9.0, color.gamma_multiply(0.22));
                icon_at(ui, badge, kind, color);
                text(
                    ui,
                    r.min + Vec2::new(69.0, 18.0),
                    title,
                    13.5,
                    colours.text,
                    width - 80.0,
                );
                text(
                    ui,
                    r.min + Vec2::new(69.0, 42.0),
                    &sub,
                    11.0,
                    colours.muted,
                    width - 80.0,
                );
                if response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.navigate(page, ctx);
                }
            }
        });
        ui.add_space(20.0);
        if !self.playlists.is_empty() {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(i18n::tr(lang, i18n::Text::HomePlaylists))
                        .size(21.0)
                        .strong(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(egui::Button::new("View all").frame(false)).clicked() {
                        self.navigate(Page::Library, ctx);
                    }
                });
            });
            self.home_playlists(ui, ctx);
            ui.add_space(22.0);
        }
        if self.is_connected() {
            self.draw_top_music(ui, ctx);
            ui.add_space(24.0);
        } else {
            ui.add_space(12.0);
        }
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(i18n::tr(lang, i18n::Text::RecentlyPlayed))
                    .size(21.0)
                    .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("See all").color(colours.muted))
                            .frame(false),
                    )
                    .clicked()
                {
                    self.navigate(Page::Recent, ctx);
                }
            });
        });
        ui.add_space(7.0);
        let tracks = if self.is_demo_mode() {
            self.demo_tracks.clone()
        } else {
            self.recent_tracks.clone()
        };
        let mut seen = HashSet::new();
        let tracks: Vec<_> = tracks
            .into_iter()
            .filter(|t| seen.insert(t.id.clone()))
            .collect();
        if tracks.is_empty() {
            ui.label(RichText::new("Your recent listening will appear here.").color(colours.muted));
        }
        self.track_cards(ui, ctx, &tracks, 6);
        ui.add_space(22.0);
        if let Some(error) = self.library_error.clone() {
            ui.add_space(16.0);
            self.error_notice(ui, &error);
        }
    }

    fn home_playlists(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let cols = if ui.available_width() >= 760.0 { 4 } else { 2 };
        let width = (ui.available_width() - 8.0 * (cols - 1) as f32) / cols as f32;
        let playlists: Vec<_> = self.playlists.iter().take(cols * 2).cloned().collect();
        for row in playlists.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                for playlist in row {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, 58.0), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        7.0,
                        if response.hovered() {
                            colours.hover
                        } else {
                            colours.panel2
                        },
                    );
                    let artwork =
                        Rect::from_min_size(rect.min + Vec2::splat(5.0), Vec2::splat(48.0));
                    self.art(
                        ui,
                        artwork,
                        playlist.image_url.as_deref(),
                        &playlist.name,
                        ctx,
                    );
                    let playing =
                        self.is_playlist_actively_playing(&playlist.id) && self.is_playing;
                    let hovered = ui.input(|input| {
                        input
                            .pointer
                            .hover_pos()
                            .is_some_and(|pos| rect.contains(pos))
                    });
                    let show_play = hovered || playing;
                    text(
                        ui,
                        rect.min + Vec2::new(64.0, 20.0),
                        &playlist.name,
                        12.5,
                        colours.text,
                        width - if show_play { 108.0 } else { 72.0 },
                    );
                    let play_clicked = show_play
                        && playlist_play_at(
                            ui,
                            Rect::from_center_size(
                                Pos2::new(rect.right() - 25.0, rect.center().y),
                                Vec2::splat(34.0),
                            ),
                            (&playlist.id, "home-play"),
                            playing,
                        )
                        .clicked();
                    if play_clicked {
                        self.play_playlist(playlist, ctx);
                    } else if response
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.open_playlist(playlist.clone(), ctx);
                    }
                }
            });
            ui.add_space(4.0);
        }
    }

    fn draw_top_music(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        use spotify::insights::Range;
        let colours = appearance::palette(ctx);
        if !self.insights.attempted {
            self.load_insights(ctx);
        }
        let lang = self.config.language;
        let old_range = self.insights.range;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(i18n::tr(lang, i18n::Text::YourTopMusic))
                    .size(21.0)
                    .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                egui::ComboBox::from_id_salt("top-music-range")
                    .width(132.0)
                    .selected_text(self.insights.range.label())
                    .show_ui(ui, |ui| {
                        for range in [Range::FourWeeks, Range::SixMonths, Range::Year] {
                            ui.selectable_value(&mut self.insights.range, range, range.label());
                        }
                    });
            });
        });
        if old_range != self.insights.range {
            self.config.top_music_range = self.insights.range;
            self.persist_preferences();
            self.insights.expanded = false;
            self.load_insights(ctx);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui
                .add(pill(
                    i18n::tr(lang, i18n::Text::Songs),
                    !self.insights.artists,
                    ctx,
                ))
                .clicked()
            {
                self.insights.artists = false;
            }
            if ui
                .add(pill(
                    i18n::tr(lang, i18n::Text::Artists),
                    self.insights.artists,
                    ctx,
                ))
                .clicked()
            {
                self.insights.artists = true;
            }
        });
        ui.add_space(8.0);
        if let Some(data) = self.insights.cache.get(&self.insights.range).cloned() {
            let count = if self.insights.artists {
                data.artists.len()
            } else {
                data.tracks.len()
            };
            let max = if self.insights.expanded { 20 } else { 6 };
            if self.insights.artists {
                self.top_artist_cards(ui, ctx, &data.artists, max);
            } else {
                self.track_cards(ui, ctx, &data.tracks, max);
            }
            if count == 0 {
                ui.label(
                    RichText::new("No top music available for this period.").color(colours.muted),
                );
            }
            if count > 6
                && ui
                    .add(
                        egui::Button::new(if self.insights.expanded {
                            "Show less"
                        } else {
                            "Show more"
                        })
                        .frame(false),
                    )
                    .clicked()
            {
                self.insights.expanded = !self.insights.expanded;
            }
        } else if self.insights.permission_needed {
            card(ui.ctx()).show(ui, |ui| {
                ui.label("Allow access to your top tracks and artists on Spotify.");
                ui.add_space(6.0);
                if ui.add(primary("Allow access", ui.ctx())).clicked() {
                    self.start_auth(ctx);
                }
            });
        } else if let Some(error) = self.insights.error.clone() {
            ui.label(RichText::new(error).color(colours.muted));
            if compact_pill_button(ui, "Retry", Some(Icon::Refresh), false).clicked() {
                self.load_insights(ctx);
            }
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading your top music…");
            });
        }
    }

    fn top_artist_cards(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        artists: &[spotify::catalog::Artist],
        max: usize,
    ) {
        let colours = appearance::palette(ctx);
        let cols = ((ui.available_width() + 12.0) / 150.0)
            .floor()
            .clamp(2.0, 6.0) as usize;
        let width = (ui.available_width() - 12.0 * (cols - 1) as f32) / cols as f32;
        for row in artists
            .iter()
            .take(max)
            .cloned()
            .collect::<Vec<_>>()
            .chunks(cols)
        {
            ui.horizontal(|ui| {
                for artist in row {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, width + 52.0), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        10.0,
                        if response.hovered() {
                            colours.hover
                        } else {
                            colours.panel
                        },
                    );
                    self.art_rounded(
                        ui,
                        Rect::from_min_size(
                            rect.min + Vec2::splat(10.0),
                            Vec2::splat(width - 20.0),
                        ),
                        artist.image_url.as_deref(),
                        &artist.name,
                        ctx,
                        255,
                    );
                    text_with_font(
                        ui,
                        rect.min + Vec2::new(11.0, width + 2.0),
                        &artist.name,
                        player_font(13.0, true),
                        colours.text,
                        width - 22.0,
                    );
                    text(
                        ui,
                        rect.min + Vec2::new(11.0, width + 24.0),
                        "Artist",
                        11.0,
                        colours.muted,
                        width - 22.0,
                    );
                    if response
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.open_artist_id(artist.name.clone(), artist.id.clone(), ctx);
                    }
                }
            });
        }
    }

    fn track_cards(&mut self, ui: &mut Ui, ctx: &egui::Context, tracks: &[Track], max: usize) {
        let colours = appearance::palette(ctx);
        let cols = ((ui.available_width() + 12.0) / 150.0)
            .floor()
            .clamp(2.0, 6.0) as usize;
        let width = (ui.available_width() - 12.0 * (cols - 1) as f32) / cols as f32;
        for row in tracks
            .iter()
            .take(max)
            .cloned()
            .collect::<Vec<_>>()
            .chunks(cols)
        {
            ui.horizontal(|ui| {
                for track in row {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, width + 60.0), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        11.0,
                        if response.hovered() {
                            colours.hover
                        } else {
                            colours.panel2
                        },
                    );
                    let art =
                        Rect::from_min_size(rect.min + Vec2::splat(9.0), Vec2::splat(width - 18.0));
                    self.art(ui, art, track.artwork_url.as_deref(), &track.name, ctx);
                    text(
                        ui,
                        rect.min + Vec2::new(11.0, width + 3.0),
                        &track.name,
                        12.5,
                        colours.text,
                        width - 22.0,
                    );
                    let artist_clicked = self.artist_links(
                        ui,
                        ctx,
                        track,
                        rect.min + Vec2::new(11.0, width + 25.0),
                        11.0,
                        width - 22.0,
                    );
                    if response.hovered() {
                        let play = Rect::from_center_size(
                            art.right_bottom() - Vec2::splat(22.0),
                            Vec2::splat(34.0),
                        );
                        ui.painter()
                            .circle_filled(play.center(), 17.0, colours.green);
                        icon_at(ui, play, Icon::Play, colours.button_foreground);
                    }
                    if response.clicked() && !artist_clicked {
                        self.play_track(track.clone(), ctx);
                    }
                    response.on_hover_text(format!("{} — {}", track.name, track.artist));
                }
            });
        }
    }

    fn playlist_cards(&mut self, ui: &mut Ui, ctx: &egui::Context, playlists: Vec<Playlist>) {
        let colours = appearance::palette(ctx);
        let cols = ((ui.available_width() + 12.0) / 150.0)
            .floor()
            .clamp(2.0, 6.0) as usize;
        let width = (ui.available_width() - 12.0 * (cols - 1) as f32) / cols as f32;
        for row in playlists.chunks(cols) {
            ui.horizontal(|ui| {
                for playlist in row {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, width + 60.0), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        11.0,
                        if response.hovered() {
                            colours.hover
                        } else {
                            colours.panel2
                        },
                    );
                    self.art(
                        ui,
                        Rect::from_min_size(rect.min + Vec2::splat(9.0), Vec2::splat(width - 18.0)),
                        playlist.image_url.as_deref(),
                        &playlist.name,
                        ctx,
                    );
                    text(
                        ui,
                        rect.min + Vec2::new(11.0, width + 3.0),
                        &playlist.name,
                        12.5,
                        colours.text,
                        width - 22.0,
                    );
                    text(
                        ui,
                        rect.min + Vec2::new(11.0, width + 25.0),
                        if playlist.owner.is_empty() {
                            "Playlist"
                        } else {
                            &playlist.owner
                        },
                        11.0,
                        colours.muted,
                        width - 22.0,
                    );
                    let playing =
                        self.is_playlist_actively_playing(&playlist.id) && self.is_playing;
                    let hovered = ui.input(|input| {
                        input
                            .pointer
                            .hover_pos()
                            .is_some_and(|pos| rect.contains(pos))
                    });
                    let play_clicked = if hovered || playing {
                        let button = Rect::from_center_size(
                            Pos2::new(rect.right() - 34.0, rect.top() + width - 34.0),
                            Vec2::splat(42.0),
                        );
                        playlist_play_at(ui, button, (&playlist.id, "card-play"), playing).clicked()
                    } else {
                        false
                    };
                    if play_clicked {
                        self.play_playlist(playlist, ctx);
                    } else if response
                        .clone()
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.open_playlist(playlist.clone(), ctx);
                    }
                    response.on_hover_text(&playlist.name);
                }
            });
        }
    }

    pub(super) fn draw_recent_search_tracks(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        if self.config.private_search {
            return;
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Recent searches").size(12.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Clear").size(11.0).color(colours.muted))
                            .frame(false),
                    )
                    .clicked()
                {
                    self.clear_search_history();
                }
            });
        });
        ui.add_space(4.0);
        let tracks = self.config.recent_search_tracks.clone();
        ScrollArea::vertical()
            .id_salt("recent-search-songs")
            .max_height(340.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for track in tracks {
                    let (r, response) = ui
                        .allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::click());
                    if response.hovered() {
                        ui.painter().rect_filled(r, 7.0, colours.hover);
                    }
                    let art = Rect::from_min_size(r.min + Vec2::new(6.0, 7.0), Vec2::splat(42.0));
                    self.art(ui, art, track.artwork_url.as_deref(), &track.name, ctx);
                    if response.hovered() {
                        ui.painter().rect_filled(
                            art,
                            6.0,
                            Color32::from_rgba_unmultiplied(0, 0, 0, 145),
                        );
                        icon_at(ui, art, Icon::Play, colours.text);
                    }
                    text(
                        ui,
                        r.min + Vec2::new(61.0, 7.0),
                        &track.name,
                        13.0,
                        colours.text,
                        r.width() - 104.0,
                    );
                    text(
                        ui,
                        r.min + Vec2::new(61.0, 31.0),
                        "Song ·",
                        11.0,
                        colours.muted,
                        34.0,
                    );
                    let artist_clicked = self.artist_links(
                        ui,
                        ctx,
                        &track,
                        r.min + Vec2::new(97.0, 31.0),
                        11.0,
                        r.width() - 140.0,
                    );
                    let queue = action_at(
                        ui,
                        Rect::from_center_size(
                            r.right_center() - Vec2::new(52.0, 0.0),
                            Vec2::splat(30.0),
                        ),
                        (&track.uri, "queue-recent"),
                        Icon::Queue,
                        "Add to queue",
                        false,
                    );
                    let remove = action_at(
                        ui,
                        Rect::from_center_size(
                            r.right_center() - Vec2::new(19.0, 0.0),
                            Vec2::splat(30.0),
                        ),
                        (&track.uri, "remove-recent"),
                        Icon::Close,
                        "Remove from recent searches",
                        false,
                    );
                    if queue.clicked() {
                        self.add_to_queue(track.clone(), ctx);
                    } else if remove.clicked() {
                        self.config
                            .recent_search_tracks
                            .retain(|item| item.uri != track.uri);
                        self.config.search_history_migrated = true;
                        self.save_session();
                    } else if !artist_clicked
                        && response.clicked()
                        && response
                            .interact_pointer_pos()
                            .is_some_and(|pos| pos.x < r.right() - 38.0)
                    {
                        self.play_search_track(track.clone(), ctx);
                    }
                }
            });
    }

    fn draw_search_results(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let lang = self.config.language;
        if self.last_sent_query.is_empty() && !self.search_loading {
            heading(
                ui,
                i18n::tr(lang, i18n::Text::Search),
                i18n::tr(lang, i18n::Text::SearchPlaceholder),
            );
            ScrollArea::vertical()
                .id_salt("search-empty-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if !self.config.recent_search_tracks.is_empty() {
                        self.draw_recent_search_tracks(ui, ctx);
                        ui.add_space(20.0);
                    }
                    ui.label(RichText::new("Browse moods & genres").size(19.0).strong());
                    ui.add_space(10.0);
                    let categories = [
                        (
                            "Deep Focus",
                            "instrumental ambient",
                            Color32::from_rgb(55, 115, 165),
                        ),
                        (
                            "Late-Night R&B",
                            "late night r&b",
                            Color32::from_rgb(132, 74, 156),
                        ),
                        (
                            "Workout Hype",
                            "workout hip hop",
                            Color32::from_rgb(196, 72, 62),
                        ),
                        (
                            "Chill Acoustic",
                            "acoustic chill",
                            Color32::from_rgb(48, 134, 96),
                        ),
                        (
                            "Synthwave Drive",
                            "synthwave night drive",
                            Color32::from_rgb(76, 86, 178),
                        ),
                        ("Jazz & Soul", "soul jazz", Color32::from_rgb(164, 108, 46)),
                        (
                            "Indie & Alt",
                            "indie alternative",
                            Color32::from_rgb(44, 128, 138),
                        ),
                        (
                            "Sleep & Rain",
                            "peaceful piano sleep",
                            Color32::from_rgb(70, 82, 110),
                        ),
                    ];
                    let cols = ((ui.available_width() + 12.0) / 210.0)
                        .floor()
                        .clamp(2.0, 4.0) as usize;
                    let tile_w = (ui.available_width() - 12.0 * (cols - 1) as f32) / cols as f32;
                    for row in categories.chunks(cols) {
                        ui.horizontal(|ui| {
                            for &(title, query, tint) in row {
                                let (rect, resp) =
                                    ui.allocate_exact_size(Vec2::new(tile_w, 72.0), Sense::click());
                                let fill = if resp.hovered() {
                                    tint.gamma_multiply(0.95)
                                } else {
                                    tint.gamma_multiply(0.78)
                                };
                                ui.painter().rect_filled(rect, 10.0, fill);
                                text_with_font(
                                    ui,
                                    rect.min + Vec2::new(16.0, 16.0),
                                    title,
                                    player_font(15.0, true),
                                    Color32::WHITE,
                                    tile_w - 32.0,
                                );
                                text(
                                    ui,
                                    rect.min + Vec2::new(16.0, 42.0),
                                    query,
                                    11.0,
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 195),
                                    tile_w - 32.0,
                                );
                                if resp
                                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                                    .clicked()
                                {
                                    self.search_query = query.into();
                                    self.run_search(query.into(), ctx);
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            return;
        }
        heading(
            ui,
            &format!(
                "{} “{}”",
                i18n::tr(lang, i18n::Text::SearchResults),
                self.last_sent_query
            ),
            "",
        );
        if self.search_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Searching…").color(colours.muted));
            });
            return;
        }
        if let Some(error) = self.search_error.clone() {
            self.error_notice(ui, &error);
            return;
        }
        let catalog = self.search_catalog.clone();
        if catalog.is_some() {
            ui.horizontal_wrapped(|ui| {
                for (filter, label) in [
                    (SearchFilter::All, "All"),
                    (SearchFilter::Songs, i18n::tr(lang, i18n::Text::Songs)),
                    (SearchFilter::Artists, i18n::tr(lang, i18n::Text::Artists)),
                    (SearchFilter::Albums, "Albums"),
                    (
                        SearchFilter::Playlists,
                        i18n::tr(lang, i18n::Text::HomePlaylists),
                    ),
                ] {
                    if ui
                        .add(pill(label, self.search_filter == filter, ctx))
                        .clicked()
                    {
                        self.search_filter = filter;
                    }
                }
            });
            ui.add_space(12.0);
        }
        let Some(cat) = catalog else {
            if self.search_results.is_empty() {
                ui.label(RichText::new("No songs match this search.").color(colours.muted));
                return;
            }
            self.draw_track_table(ui, ctx, self.search_results.clone());
            return;
        };
        if cat.tracks.is_empty()
            && cat.artists.is_empty()
            && cat.albums.is_empty()
            && cat.playlists.is_empty()
        {
            ui.label(RichText::new("No results match this search.").color(colours.muted));
            return;
        }
        match self.search_filter {
            SearchFilter::Songs => {
                if cat.tracks.is_empty() {
                    ui.label(RichText::new("No songs match this search.").color(colours.muted));
                } else {
                    self.draw_track_table(ui, ctx, cat.tracks);
                }
            }
            SearchFilter::Artists => {
                ScrollArea::vertical()
                    .id_salt("search-artists-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if cat.artists.is_empty() {
                            ui.label(
                                RichText::new("No artists match this search.").color(colours.muted),
                            );
                        } else {
                            self.top_artist_cards(ui, ctx, &cat.artists, 20);
                        }
                    });
            }
            SearchFilter::Albums => {
                ScrollArea::vertical()
                    .id_salt("search-albums-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if cat.albums.is_empty() {
                            ui.label(
                                RichText::new("No albums match this search.").color(colours.muted),
                            );
                        } else {
                            self.search_album_cards(ui, ctx, &cat.albums, 20);
                        }
                    });
            }
            SearchFilter::Playlists => {
                ScrollArea::vertical()
                    .id_salt("search-playlists-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if cat.playlists.is_empty() {
                            ui.label(
                                RichText::new("No playlists match this search.")
                                    .color(colours.muted),
                            );
                        } else {
                            self.playlist_cards(ui, ctx, cat.playlists);
                        }
                    });
            }
            SearchFilter::All => {
                ScrollArea::vertical()
                    .id_salt("search-all-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !cat.tracks.is_empty() || !cat.artists.is_empty() {
                            let avail = ui.available_width();
                            if avail > 660.0 && !cat.tracks.is_empty() {
                                let left_w = (avail * 0.37).clamp(240.0, 380.0);
                                let right_w = (avail - left_w - 20.0).max(200.0);
                                ui.horizontal_top(|ui| {
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(left_w, 232.0),
                                        Layout::top_down(Align::LEFT),
                                        |ui| {
                                            ui.label(
                                                RichText::new("Top result").size(18.0).strong(),
                                            );
                                            ui.add_space(6.0);
                                            let top = cat.tracks[0].clone();
                                            let (card_rect, resp) = ui.allocate_exact_size(
                                                Vec2::new(left_w, 188.0),
                                                Sense::click(),
                                            );
                                            ui.painter().rect_filled(
                                                card_rect,
                                                10.0,
                                                if resp.hovered() {
                                                    colours.hover
                                                } else {
                                                    colours.panel
                                                },
                                            );
                                            let art = Rect::from_min_size(
                                                card_rect.min + Vec2::splat(18.0),
                                                Vec2::splat(82.0),
                                            );
                                            self.art(
                                                ui,
                                                art,
                                                top.artwork_url.as_deref(),
                                                &top.name,
                                                ctx,
                                            );
                                            text_with_font(
                                                ui,
                                                Pos2::new(
                                                    card_rect.left() + 18.0,
                                                    art.bottom() + 14.0,
                                                ),
                                                &top.name,
                                                player_font(20.0, true),
                                                colours.text,
                                                left_w - 86.0,
                                            );
                                            let sub_y = art.bottom() + 44.0;
                                            let prefix = if top.explicit {
                                                "Song · E · "
                                            } else {
                                                "Song · "
                                            };
                                            let prefix_w = ui
                                                .painter()
                                                .layout_no_wrap(
                                                    prefix.into(),
                                                    FontId::proportional(12.5),
                                                    colours.muted,
                                                )
                                                .size()
                                                .x;
                                            text(
                                                ui,
                                                Pos2::new(card_rect.left() + 18.0, sub_y),
                                                prefix,
                                                12.5,
                                                colours.muted,
                                                prefix_w + 4.0,
                                            );
                                            let artist_clicked = self.artist_links(
                                                ui,
                                                ctx,
                                                &top,
                                                Pos2::new(
                                                    card_rect.left() + 18.0 + prefix_w,
                                                    sub_y,
                                                ),
                                                12.5,
                                                (left_w - 86.0 - prefix_w).max(40.0),
                                            );
                                            let play_rect = Rect::from_center_size(
                                                card_rect.right_bottom() - Vec2::splat(34.0),
                                                Vec2::splat(44.0),
                                            );
                                            let playing_top = self.current_track.uri == top.uri
                                                && self.is_playing;
                                            let play_resp = playlist_play_at(
                                                ui,
                                                play_rect,
                                                (&top.uri, "search-top-play"),
                                                playing_top,
                                            );
                                            if play_resp.clicked()
                                                || (!artist_clicked
                                                    && resp
                                                        .on_hover_cursor(
                                                            egui::CursorIcon::PointingHand,
                                                        )
                                                        .clicked())
                                            {
                                                if playing_top {
                                                    self.send_playback_action("pause", ctx);
                                                } else if self.current_track.uri == top.uri
                                                    && !self.is_playing
                                                {
                                                    self.send_playback_action("resume", ctx);
                                                } else {
                                                    self.play_search_track(top, ctx);
                                                }
                                            }
                                        },
                                    );
                                    ui.add_space(20.0);
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(right_w, 232.0),
                                        Layout::top_down(Align::LEFT),
                                        |ui| {
                                            ui.label(RichText::new("Songs").size(18.0).strong());
                                            ui.add_space(6.0);
                                            for (idx, track) in
                                                cat.tracks.iter().take(4).cloned().enumerate()
                                            {
                                                let (r, resp) = ui.allocate_exact_size(
                                                    Vec2::new(right_w, 46.0),
                                                    Sense::click(),
                                                );
                                                let current = track.uri == self.current_track.uri;
                                                if resp.hovered() {
                                                    ui.painter().rect_filled(r, 7.0, colours.hover);
                                                }
                                                let art = Rect::from_min_size(
                                                    r.min + Vec2::new(6.0, 5.0),
                                                    Vec2::splat(36.0),
                                                );
                                                self.art(
                                                    ui,
                                                    art,
                                                    track.artwork_url.as_deref(),
                                                    &track.name,
                                                    ctx,
                                                );
                                                text(
                                                    ui,
                                                    r.min + Vec2::new(52.0, 6.0),
                                                    &track.name,
                                                    13.0,
                                                    if current {
                                                        colours.green
                                                    } else {
                                                        colours.text
                                                    },
                                                    right_w - 165.0,
                                                );
                                                let artist_clicked = self.artist_links(
                                                    ui,
                                                    ctx,
                                                    &track,
                                                    r.min + Vec2::new(52.0, 24.0),
                                                    11.0,
                                                    right_w - 165.0,
                                                );
                                                text(
                                                    ui,
                                                    Pos2::new(
                                                        r.right() - 102.0,
                                                        r.center().y - 7.0,
                                                    ),
                                                    &format_time(track.duration_ms / 1000),
                                                    11.0,
                                                    colours.muted,
                                                    40.0,
                                                );
                                                if action_at(
                                                    ui,
                                                    Rect::from_min_size(
                                                        Pos2::new(
                                                            r.right() - 60.0,
                                                            r.center().y - 14.0,
                                                        ),
                                                        Vec2::splat(28.0),
                                                    ),
                                                    (idx, "search-top-queue"),
                                                    Icon::Queue,
                                                    "Add to queue",
                                                    false,
                                                )
                                                .clicked()
                                                {
                                                    self.add_to_queue(track.clone(), ctx);
                                                }
                                                let saved =
                                                    self.saved_track_ids.contains(&track.id);
                                                if action_at(
                                                    ui,
                                                    Rect::from_min_size(
                                                        Pos2::new(
                                                            r.right() - 30.0,
                                                            r.center().y - 14.0,
                                                        ),
                                                        Vec2::splat(28.0),
                                                    ),
                                                    (idx, "search-top-save"),
                                                    Icon::Heart,
                                                    "Like song",
                                                    saved,
                                                )
                                                .clicked()
                                                {
                                                    self.toggle_saved_track(track.clone(), ctx);
                                                }
                                                if !artist_clicked
                                                    && resp.clicked()
                                                    && resp
                                                        .interact_pointer_pos()
                                                        .is_some_and(|p| p.x < r.right() - 64.0)
                                                {
                                                    self.play_search_track(track, ctx);
                                                }
                                            }
                                        },
                                    );
                                });
                            } else if !cat.tracks.is_empty() {
                                ui.label(RichText::new("Songs").size(18.0).strong());
                                ui.add_space(6.0);
                                self.draw_track_table(
                                    ui,
                                    ctx,
                                    cat.tracks.iter().take(6).cloned().collect(),
                                );
                            }
                            ui.add_space(20.0);
                        }
                        if !cat.artists.is_empty() {
                            ui.label(
                                RichText::new(i18n::tr(lang, i18n::Text::Artists))
                                    .size(18.0)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            self.top_artist_cards(ui, ctx, &cat.artists, 6);
                            ui.add_space(20.0);
                        }
                        if !cat.albums.is_empty() {
                            ui.label(RichText::new("Albums").size(18.0).strong());
                            ui.add_space(8.0);
                            self.search_album_cards(ui, ctx, &cat.albums, 6);
                            ui.add_space(20.0);
                        }
                        if !cat.playlists.is_empty() {
                            ui.label(
                                RichText::new(i18n::tr(lang, i18n::Text::HomePlaylists))
                                    .size(18.0)
                                    .strong(),
                            );
                            ui.add_space(8.0);
                            self.playlist_cards(
                                ui,
                                ctx,
                                cat.playlists.into_iter().take(6).collect(),
                            );
                        }
                    });
            }
        }
    }

    fn search_album_cards(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        albums: &[catalog::Album],
        max: usize,
    ) {
        let colours = appearance::palette(ctx);
        let cols = ((ui.available_width() + 12.0) / 160.0)
            .floor()
            .clamp(2.0, 6.0) as usize;
        let width = (ui.available_width() - 12.0 * (cols - 1) as f32) / cols as f32;
        for row in albums
            .iter()
            .take(max)
            .cloned()
            .collect::<Vec<_>>()
            .chunks(cols)
        {
            ui.horizontal(|ui| {
                for album in row {
                    let (rect, response) =
                        ui.allocate_exact_size(Vec2::new(width, width + 58.0), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        10.0,
                        if response.hovered() {
                            colours.hover
                        } else {
                            colours.panel
                        },
                    );
                    let art =
                        Rect::from_min_size(rect.min + Vec2::splat(9.0), Vec2::splat(width - 18.0));
                    self.art(ui, art, album.artwork_url.as_deref(), &album.name, ctx);
                    text_with_font(
                        ui,
                        Pos2::new(rect.left() + 11.0, art.bottom() + 10.0),
                        &album.name,
                        player_font(13.0, true),
                        colours.text,
                        width - 22.0,
                    );
                    let artist = album.artist_names.join(", ");
                    let year = album.release_date.get(..4).unwrap_or(&album.release_date);
                    let sub = if year.is_empty() {
                        artist
                    } else if artist.is_empty() {
                        year.to_owned()
                    } else {
                        format!("{year} · {artist}")
                    };
                    text(
                        ui,
                        Pos2::new(rect.left() + 11.0, art.bottom() + 31.0),
                        &sub,
                        11.0,
                        colours.muted,
                        width - 22.0,
                    );
                    if response
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        self.open_album(album.as_track(), ctx);
                    }
                }
            });
        }
    }

    fn draw_library(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        if let Some(playlist) = self.selected_playlist.clone() {
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("← Back to your library")
                            .size(12.0)
                            .color(colours.muted),
                    )
                    .frame(false),
                )
                .clicked()
            {
                self.selected_playlist = None;
                return;
            }
            ui.add_space(6.0);
            let total_ms: u64 = self.playlist_tracks.iter().map(|t| t.duration_ms).sum();
            ui.horizontal(|ui| {
                let (art, _) = ui.allocate_exact_size(Vec2::splat(132.0), Sense::hover());
                self.art(ui, art, playlist.image_url.as_deref(), &playlist.name, ctx);
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("PLAYLIST")
                            .size(11.0)
                            .strong()
                            .color(colours.green),
                    );
                    ui.label(RichText::new(&playlist.name).font(player_font(30.0, true)));
                    let mut meta = format!("{} · {} songs", playlist.owner, playlist.total_items);
                    if total_ms > 0 {
                        let mins = (total_ms / 60_000).max(1);
                        if mins >= 60 {
                            meta.push_str(&format!(" · {} hr {} min", mins / 60, mins % 60));
                        } else {
                            meta.push_str(&format!(" · {mins} min"));
                        }
                    }
                    ui.label(RichText::new(meta).size(12.5).color(colours.muted));
                    if !playlist.description.is_empty() {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&playlist.description)
                                .size(12.0)
                                .color(colours.muted),
                        );
                    }
                });
            });
            ui.add_space(16.0);
            let mut filter_changed = false;
            ui.horizontal_wrapped(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(52.0), Sense::hover());
                let playing = self.is_playlist_actively_playing(&playlist.id) && self.is_playing;
                if playlist_play_at(ui, rect, (&playlist.id, "header-play"), playing).clicked() {
                    self.play_playlist(&playlist, ctx);
                }
                ui.add_space(4.0);
                let has_tracks = !self.playlist_tracks.is_empty();
                if has_tracks && action_button(ui, "Shuffle", Some(Icon::Shuffle), false).clicked()
                {
                    let tracks = self.playlist_tracks.clone();
                    self.play_tracks_shuffled(tracks, ctx);
                }
                if has_tracks && action_button(ui, "Queue all", Some(Icon::Queue), false).clicked()
                {
                    let tracks = self.playlist_tracks.clone();
                    self.queue_tracks_batch(tracks, ctx);
                }
                if self.can_edit_playlist(&playlist)
                    && ui
                        .add_enabled(
                            self.playlist_add_pending.is_none(),
                            egui::Button::new("Edit playlist"),
                        )
                        .clicked()
                {
                    self.open_playlist_editor(playlist.clone(), ctx);
                }
                if !self.playlist_tracks.is_empty() {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        filter_changed |= search_pill_input(
                            ui,
                            &mut self.playlist_track_filter,
                            ("playlist-track-filter", &playlist.id),
                            "Find in playlist",
                            ui.available_width().min(220.0),
                            "Clear playlist filter",
                        );
                    });
                }
            });
            ui.add_space(12.0);
            if self.playlist_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading playlist…");
                });
            } else if !playlist.items_accessible {
                self.error_notice(ui,"Spotify hasn't made this playlist's songs available to this app. You can browse playlists you own or collaborate on.");
            } else if let Some(error) = self.playlist_error.clone() {
                self.error_notice(ui, &error);
                if action_button(ui, "Retry", Some(Icon::Refresh), false).clicked() {
                    self.open_playlist(playlist, ctx);
                }
            } else if self.playlist_tracks.is_empty() {
                ui.label(RichText::new("This playlist has no songs.").color(colours.muted));
            } else {
                self.draw_track_rows(ui, ctx, self.playlist_tracks.clone(), filter_changed);
            }
            return;
        }
        let lang = self.config.language;
        heading(
            ui,
            i18n::tr(lang, i18n::Text::YourLibrary),
            i18n::tr(lang, i18n::Text::LibrarySubtitle),
        );
        ui.horizontal(|ui| {
            if ui
                .add(primary(
                    i18n::tr(lang, i18n::Text::CreatePlaylist),
                    ui.ctx(),
                ))
                .clicked()
            {
                if self.is_connected() {
                    self.show_create_playlist = true;
                } else {
                    self.open_spotify_setup(ctx);
                }
            }
            if icon_button(ui, Icon::Refresh, "Refresh library", false).clicked() {
                self.load_library(ctx, true);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                search_pill_input(
                    ui,
                    &mut self.library_filter,
                    "library-playlist-filter",
                    "Filter your playlists",
                    ui.available_width().min(220.0),
                    "Clear playlist filter",
                );
            });
        });
        ui.add_space(18.0);
        if let Some(error) = self.library_error.clone() {
            self.error_notice(ui, &error);
        }
        if self.library_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Updating your library…").color(colours.muted));
            });
        }
        let query = self.library_filter.to_lowercase();
        let playlists = self
            .playlists
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&query))
            .cloned()
            .collect::<Vec<_>>();
        if playlists.is_empty() && !self.library_loading {
            ui.label(
                RichText::new(if query.is_empty() {
                    "Your playlists will appear here when Spotify is connected."
                } else {
                    "No playlists match that filter."
                })
                .color(colours.muted),
            );
        }
        self.playlist_cards(ui, ctx, playlists);
    }

    fn draw_track_table(&mut self, ui: &mut Ui, ctx: &egui::Context, tracks: Vec<Track>) {
        self.draw_track_rows(ui, ctx, tracks, false);
    }

    fn draw_track_rows(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        tracks: Vec<Track>,
        reset_scroll: bool,
    ) {
        let colours = appearance::palette(ctx);
        if tracks.is_empty() {
            if !self.library_loading && !self.playlist_loading {
                ui.label(
                    RichText::new(
                        "Nothing here yet. Find a song you love and save it to your library.",
                    )
                    .color(colours.muted),
                );
            }
            return;
        }
        let playlist = self
            .selected_playlist
            .as_ref()
            .filter(|_| self.page == Page::Library);
        let query = playlist
            .map_or("", |_| self.playlist_track_filter.trim())
            .to_lowercase();
        let terms: Vec<_> = query.split_whitespace().collect();
        let filtered_indices = (!terms.is_empty()).then(|| {
            tracks
                .iter()
                .enumerate()
                .filter_map(|(index, track)| {
                    let searchable =
                        format!("{} {} {}", track.name, track.artist, track.album).to_lowercase();
                    terms
                        .iter()
                        .all(|term| searchable.contains(term))
                        .then_some(index)
                })
                .collect::<Vec<_>>()
        });
        let sort_context = match self.page {
            Page::Library => {
                playlist.map_or_else(|| "library".to_owned(), |p| format!("playlist:{}", p.id))
            }
            Page::Album => format!(
                "album:{}",
                self.browse
                    .album
                    .as_ref()
                    .map(|a| a.album.id.as_str())
                    .unwrap_or("")
            ),
            Page::Artist => format!("artist:{}", self.browse.artist_id),
            Page::Search => format!("search:{}", self.last_sent_query),
            Page::Liked => "liked".to_owned(),
            Page::Recent => "recent".to_owned(),
            other => format!("page:{}", other as u8),
        };
        if self.track_sort_context != sort_context {
            self.track_sort_context = sort_context;
            self.track_sort = crate::app::TrackSort::default();
        }
        let mut display_indices: Vec<usize> =
            filtered_indices.unwrap_or_else(|| (0..tracks.len()).collect());
        if self.track_sort.column != crate::app::TrackSortColumn::Default {
            let asc = self.track_sort.ascending;
            let col = self.track_sort.column;
            display_indices.sort_by(|&a, &b| {
                let ord = match col {
                    crate::app::TrackSortColumn::Default => a.cmp(&b),
                    crate::app::TrackSortColumn::Title => tracks[a]
                        .name
                        .to_lowercase()
                        .cmp(&tracks[b].name.to_lowercase())
                        .then_with(|| {
                            tracks[a]
                                .artist
                                .to_lowercase()
                                .cmp(&tracks[b].artist.to_lowercase())
                        }),
                    crate::app::TrackSortColumn::Album => tracks[a]
                        .album
                        .to_lowercase()
                        .cmp(&tracks[b].album.to_lowercase())
                        .then_with(|| {
                            tracks[a]
                                .name
                                .to_lowercase()
                                .cmp(&tracks[b].name.to_lowercase())
                        }),
                    crate::app::TrackSortColumn::Duration => tracks[a]
                        .duration_ms
                        .cmp(&tracks[b].duration_ms)
                        .then_with(|| {
                            tracks[a]
                                .name
                                .to_lowercase()
                                .cmp(&tracks[b].name.to_lowercase())
                        }),
                };
                let ordered = if asc { ord } else { ord.reverse() };
                ordered.then_with(|| a.cmp(&b))
            });
        }
        let row_count = display_indices.len();
        if row_count == 0 {
            ui.label(RichText::new("No songs match this filter.").color(colours.muted));
            return;
        }
        let album_column = self.page != Page::Album
            && ui.available_width() > 750.0
            && !self.config.hide_album_column;
        let (head, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::hover());
        let hash_rect = Rect::from_min_size(head.min + Vec2::new(8.0, 0.0), Vec2::new(34.0, 26.0));
        let hash_resp = ui
            .interact(hash_rect, ui.id().with("sort-hash"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Reset track order");
        text(
            ui,
            head.min + Vec2::new(12.0, 7.0),
            "#",
            10.0,
            if self.track_sort.column == crate::app::TrackSortColumn::Default || hash_resp.hovered()
            {
                colours.text
            } else {
                colours.muted
            },
            25.0,
        );
        if hash_resp.clicked() {
            self.track_sort = crate::app::TrackSort::default();
        }
        let current_sort = self.track_sort;
        let sort_label = |base: &str, col: crate::app::TrackSortColumn| -> String {
            if current_sort.column == col {
                if current_sort.ascending {
                    format!("{base} ↑")
                } else {
                    format!("{base} ↓")
                }
            } else {
                base.to_owned()
            }
        };
        let title_rect =
            Rect::from_min_size(head.min + Vec2::new(90.0, 0.0), Vec2::new(140.0, 26.0));
        let title_resp = ui
            .interact(title_rect, ui.id().with("sort-title"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Sort by title");
        let title_active = self.track_sort.column == crate::app::TrackSortColumn::Title;
        text(
            ui,
            head.min + Vec2::new(90.0, 7.0),
            &sort_label("TITLE", crate::app::TrackSortColumn::Title),
            10.0,
            if title_active || title_resp.hovered() {
                colours.text
            } else {
                colours.muted
            },
            140.0,
        );
        if title_resp.clicked() {
            self.track_sort = match self.track_sort.column {
                crate::app::TrackSortColumn::Title if self.track_sort.ascending => {
                    crate::app::TrackSort {
                        column: crate::app::TrackSortColumn::Title,
                        ascending: false,
                    }
                }
                crate::app::TrackSortColumn::Title => crate::app::TrackSort::default(),
                _ => crate::app::TrackSort {
                    column: crate::app::TrackSortColumn::Title,
                    ascending: true,
                },
            };
        }
        if album_column {
            let album_rect = Rect::from_min_size(
                Pos2::new(head.left() + head.width() * 0.55, head.top()),
                Vec2::new(130.0, 26.0),
            );
            let album_resp = ui
                .interact(album_rect, ui.id().with("sort-album"), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Sort by album");
            let album_active = self.track_sort.column == crate::app::TrackSortColumn::Album;
            text(
                ui,
                Pos2::new(head.left() + head.width() * 0.55, head.top() + 7.0),
                &sort_label("ALBUM", crate::app::TrackSortColumn::Album),
                10.0,
                if album_active || album_resp.hovered() {
                    colours.text
                } else {
                    colours.muted
                },
                130.0,
            );
            if album_resp.clicked() {
                self.track_sort = match self.track_sort.column {
                    crate::app::TrackSortColumn::Album if self.track_sort.ascending => {
                        crate::app::TrackSort {
                            column: crate::app::TrackSortColumn::Album,
                            ascending: false,
                        }
                    }
                    crate::app::TrackSortColumn::Album => crate::app::TrackSort::default(),
                    _ => crate::app::TrackSort {
                        column: crate::app::TrackSortColumn::Album,
                        ascending: true,
                    },
                };
            }
        }
        let dur_rect = Rect::from_min_size(
            Pos2::new((head.right() - 168.0).max(head.left() + 240.0), head.top()),
            Vec2::new(82.0, 26.0),
        );
        let dur_resp = ui
            .interact(dur_rect, ui.id().with("sort-duration"), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Sort by duration");
        let dur_active = self.track_sort.column == crate::app::TrackSortColumn::Duration;
        text(
            ui,
            dur_rect.min + Vec2::new(0.0, 7.0),
            &sort_label("DURATION", crate::app::TrackSortColumn::Duration),
            10.0,
            if dur_active || dur_resp.hovered() {
                colours.text
            } else {
                colours.muted
            },
            82.0,
        );
        if dur_resp.clicked() {
            self.track_sort = match self.track_sort.column {
                crate::app::TrackSortColumn::Duration if self.track_sort.ascending => {
                    crate::app::TrackSort {
                        column: crate::app::TrackSortColumn::Duration,
                        ascending: false,
                    }
                }
                crate::app::TrackSortColumn::Duration => crate::app::TrackSort::default(),
                _ => crate::app::TrackSort {
                    column: crate::app::TrackSortColumn::Duration,
                    ascending: true,
                },
            };
        }
        ui.painter().line_segment(
            [head.left_bottom(), head.right_bottom()],
            Stroke::new(1.0_f32, colours.border),
        );
        let row_h = if self.config.compact_rows { 50.0 } else { 64.0 };
        let mut scroll = ScrollArea::vertical()
            .id_salt((
                "track-table",
                self.page as u8,
                playlist.map(|p| p.id.as_str()),
            ))
            .auto_shrink([false, false]);
        if reset_scroll {
            scroll = scroll.vertical_scroll_offset(0.0);
        }
        scroll.show_rows(ui, row_h - 4.0, row_count, |ui, range| {
            for row in range {
                let i = display_indices[row];
                let track = &tracks[i];
                let (r, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), row_h - 4.0),
                    Sense::click(),
                );
                if response.hovered() {
                    ui.painter().rect_filled(r, 8.0, colours.hover);
                }
                let current = !track.id.is_empty() && track.id == self.current_track.id;
                let color = if current { colours.green } else { colours.text };
                let y = r.top() + (r.height() - 40.0) * 0.5;
                if response.hovered() {
                    icon_at(
                        ui,
                        Rect::from_min_size(Pos2::new(r.left() + 6.0, y + 10.0), Vec2::splat(20.0)),
                        Icon::Play,
                        colours.green,
                    );
                } else {
                    let display_num = track.context_position.map_or(i + 1, |pos| pos + 1);
                    text(
                        ui,
                        Pos2::new(r.left() + 11.0, y + 12.0),
                        &display_num.to_string(),
                        11.0,
                        if current {
                            colours.green
                        } else {
                            colours.muted
                        },
                        25.0,
                    );
                }
                self.art(
                    ui,
                    Rect::from_min_size(Pos2::new(r.left() + 39.0, y), Vec2::splat(40.0)),
                    track.artwork_url.as_deref(),
                    &track.name,
                    ctx,
                );
                let show_queue_action = true;
                let controls = if show_queue_action { 168.0 } else { 128.0 };
                let title_end = if album_column {
                    r.left() + r.width() * 0.53
                } else {
                    r.right() - controls
                };
                text(
                    ui,
                    Pos2::new(r.left() + 93.0, y + 2.0),
                    &track.name,
                    13.0,
                    color,
                    title_end - r.left() - 100.0,
                );
                let artist_x = if track.explicit {
                    let badge = Rect::from_min_size(
                        Pos2::new(r.left() + 93.0, y + 24.0),
                        Vec2::new(14.0, 14.0),
                    );
                    ui.painter().rect_filled(badge, 3.0, colours.hover);
                    ui.painter().text(
                        badge.center(),
                        Align2::CENTER_CENTER,
                        "E",
                        FontId::proportional(9.0),
                        colours.muted,
                    );
                    r.left() + 112.0
                } else {
                    r.left() + 93.0
                };
                let artist_clicked = self.artist_links(
                    ui,
                    ctx,
                    track,
                    Pos2::new(artist_x, y + 24.0),
                    11.0,
                    (title_end - artist_x - 8.0).max(20.0),
                );
                if album_column {
                    text(
                        ui,
                        Pos2::new(r.left() + r.width() * 0.55, y + 12.0),
                        &track.album,
                        11.5,
                        colours.muted,
                        r.width() * 0.45 - controls - 12.0,
                    );
                }
                let button_y = r.center().y - 15.0;
                text(
                    ui,
                    Pos2::new(r.right() - controls, button_y + 8.0),
                    &format_time(track.duration_ms / 1000),
                    11.0,
                    colours.muted,
                    44.0,
                );
                let saved = self.saved_track_ids.contains(&track.id);
                if show_queue_action
                    && action_at(
                        ui,
                        Rect::from_min_size(
                            Pos2::new(r.right() - 116.0, button_y),
                            Vec2::splat(30.0),
                        ),
                        (i, "queue"),
                        Icon::Queue,
                        "Add to queue",
                        false,
                    )
                    .clicked()
                {
                    self.add_to_queue(track.clone(), ctx);
                }
                if action_at(
                    ui,
                    Rect::from_min_size(Pos2::new(r.right() - 76.0, button_y), Vec2::splat(30.0)),
                    (i, "save"),
                    Icon::Heart,
                    "Like song",
                    saved,
                )
                .clicked()
                {
                    self.toggle_saved_track(track.clone(), ctx);
                }
                let more = action_at(
                    ui,
                    Rect::from_min_size(Pos2::new(r.right() - 36.0, button_y), Vec2::splat(30.0)),
                    (i, "more"),
                    Icon::More,
                    "More options",
                    false,
                );
                self.track_menu(ui, ctx, &more, track, i);
                if !artist_clicked
                    && (response.double_clicked()
                        || (response.clicked()
                            && response
                                .interact_pointer_pos()
                                .is_some_and(|p| p.x < r.left() + 36.0)))
                {
                    self.play_table_track(track.clone(), i, ctx);
                }
            }
        });
    }

    fn track_menu(
        &mut self,
        _parent: &mut Ui,
        ctx: &egui::Context,
        button: &egui::Response,
        track: &Track,
        index: usize,
    ) {
        egui::Popup::menu(button).show(|ui| {
            if ui.button("Play now").clicked() {
                self.play_table_track(track.clone(), index, ctx);
                ui.close();
            }
            if ui.button("Add to queue").clicked() {
                self.add_to_queue(track.clone(), ctx);
                ui.close();
            }
            if (!track.album_id.is_empty() || !track.album.is_empty())
                && ui.button("Go to album").clicked()
            {
                self.open_album(track.clone(), ctx);
                ui.close();
            }
            if let Some(artist_name) = track
                .artist_names
                .first()
                .filter(|s| !s.is_empty())
                .cloned()
                .or_else(|| (!track.artist.is_empty()).then(|| track.artist.clone()))
                && ui.button("Go to artist").clicked()
            {
                let id = track.artist_ids.first().cloned().unwrap_or_default();
                self.open_artist_id(artist_name, id, ctx);
                ui.close();
            }
            if !track.id.is_empty() && ui.button("Copy Spotify link").clicked() {
                ctx.copy_text(format!("https://open.spotify.com/track/{}", track.id));
                self.set_status("Copied Spotify track link");
                ui.close();
            }
            ui.separator();
            if self.page == Page::Library
                && let Some(playlist) = self.selected_playlist.clone()
                && self
                    .user_profile
                    .as_ref()
                    .is_some_and(|p| playlist.is_owned_by(&p.id))
            {
                if ui
                    .add_enabled(
                        self.playlist_add_pending.is_none(),
                        egui::Button::new("Remove from this playlist"),
                    )
                    .clicked()
                {
                    self.remove_track_from_playlist(playlist, track.clone(), ctx);
                    ui.close();
                }
            }
            if ui.button("Add to playlist").clicked() {
                self.open_playlist_picker(track.clone());
                self.playlist_picker_anchor = Some(button.rect);
                ui.close();
            }
        });
    }

    pub(crate) fn draw_playlist_picker(&mut self, ctx: &egui::Context) {
        self.ensure_playlist_membership(ctx, true);
        let colours = appearance::palette(ctx);
        let mut open = self.show_add_to_playlist;
        let track = self.track_to_add_to_playlist.clone().unwrap_or_default();
        let screen = ctx.content_rect();
        let anchor = self
            .playlist_picker_anchor
            .or(self.current_track_menu_rect)
            .unwrap_or_else(|| {
                Rect::from_min_size(
                    Pos2::new(screen.left() + 240.0, screen.bottom() - 64.0),
                    Vec2::splat(30.0),
                )
            });
        let user_id = self
            .user_profile
            .as_ref()
            .map(|profile| profile.id.clone())
            .unwrap_or_default();
        let owned_count = self
            .playlists
            .iter()
            .filter(|playlist| playlist.is_owned_by(&user_id))
            .count();
        let query = self.playlist_picker_filter.to_lowercase();
        let playlists: Vec<_> = self
            .playlists
            .iter()
            .filter(|playlist| {
                playlist.is_owned_by(&user_id) && playlist.name.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
        // Keep row order stable after an add. Use only confirmed membership; do not
        // scan every playlist or infer absence from a partial playlist page.
        let busy = self.playlist_add_pending.is_some();
        let popup = egui::Popup::new(
            Id::new("owned-playlist-picker"),
            ctx.clone(),
            anchor,
            egui::LayerId::new(Order::Middle, Id::new("playlist-picker-parent")),
        )
        .open_bool(&mut open)
        .align(egui::emath::RectAlign::TOP_START)
        .align_alternatives(&[egui::emath::RectAlign::BOTTOM_START])
        .gap(8.0)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .width(280.0)
        .frame(
            Frame::popup(&ctx.style())
                .fill(colours.panel2)
                .corner_radius(10)
                .inner_margin(12),
        )
        .show(|ui| {
            ui.set_width(256.0);
            ui.spacing_mut().interact_size.y = 24.0;
            ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            let (heading, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::hover());
            text(
                ui,
                heading.min + Vec2::new(0.0, 4.0),
                "Add to playlist",
                12.0,
                colours.muted,
                heading.width() - 30.0,
            );
            let close = Rect::from_center_size(
                Pos2::new(heading.right() - 11.0, heading.center().y),
                Vec2::splat(22.0),
            );
            if action_at(
                ui,
                close,
                "close-owned-playlists",
                Icon::Close,
                "Close playlist menu",
                false,
            )
            .clicked()
            {
                ui.close();
            }
            Frame::new()
                .fill(colours.panel)
                .corner_radius(6)
                .inner_margin(6)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                        icon_at(ui, rect, Icon::Search, colours.muted);
                        ui.add(
                            TextEdit::singleline(&mut self.playlist_picker_filter)
                                .frame(false)
                                .hint_text("Find a playlist")
                                .desired_width(ui.available_width()),
                        );
                    });
                });
            let (new_rect, new_response) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::click());
            if new_response.hovered() && !busy {
                ui.painter().rect_filled(new_rect, 6.0, colours.hover);
            }
            icon_at(
                ui,
                Rect::from_center_size(
                    Pos2::new(new_rect.left() + 12.0, new_rect.center().y),
                    Vec2::splat(20.0),
                ),
                Icon::Plus,
                colours.muted,
            );
            text(
                ui,
                new_rect.min + Vec2::new(30.0, 7.0),
                "New playlist",
                13.0,
                if busy { colours.muted } else { colours.text },
                new_rect.width() - 36.0,
            );
            if new_response.clicked() && !busy {
                self.show_add_to_playlist = false;
                self.show_create_playlist = true;
                ui.close();
            }
            ui.separator();
            ui.label(
                RichText::new("Save to")
                    .size(11.0)
                    .strong()
                    .color(colours.muted),
            );
            ui.spacing_mut().scroll.floating = false;
            ScrollArea::vertical()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .id_salt("owned-playlist-picker-list")
                .max_height((screen.height() - 260.0).clamp(100.0, 252.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    if "liked songs".contains(&query) {
                        let saved = self.saved_track_ids.contains(&track.id);
                        let pending = self.library_changes.contains_key(&track.id);
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 50.0),
                            Sense::click(),
                        );
                        if response.hovered() && !pending {
                            ui.painter().rect_filled(rect, 6.0, colours.hover);
                        }
                        let art =
                            Rect::from_min_size(rect.min + Vec2::new(4.0, 8.0), Vec2::splat(36.0));
                        ui.painter()
                            .rect_filled(art, 5.0, Color32::from_rgb(89, 66, 156));
                        icon_at(ui, art.shrink(5.0), Icon::Heart, colours.text);
                        text(
                            ui,
                            rect.min + Vec2::new(50.0, 8.0),
                            "Liked Songs",
                            13.0,
                            colours.text,
                            rect.width() - 80.0,
                        );
                        text(
                            ui,
                            rect.min + Vec2::new(50.0, 29.0),
                            if pending {
                                "Updating…"
                            } else if saved {
                                "Saved in your library"
                            } else {
                                "Save in your library"
                            },
                            11.0,
                            if saved { colours.green } else { colours.muted },
                            rect.width() - 80.0,
                        );
                        let check = Rect::from_center_size(
                            Pos2::new(rect.right() - 13.0, rect.center().y),
                            Vec2::splat(18.0),
                        );
                        if pending {
                            ui.put(check, egui::Spinner::new().size(16.0));
                        } else {
                            icon_at(
                                ui,
                                check,
                                if saved {
                                    Icon::CircleCheck
                                } else {
                                    Icon::CirclePlus
                                },
                                if saved { colours.green } else { colours.muted },
                            );
                        }
                        response.widget_info(|| {
                            egui::WidgetInfo::labeled(
                                egui::WidgetType::Button,
                                !pending,
                                "Liked Songs",
                            )
                        });
                        if response
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                            && !pending
                        {
                            self.toggle_saved_track(track.clone(), ctx);
                        }
                        ui.separator();
                    }
                    if playlists.is_empty() {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(if user_id.is_empty() {
                                "Loading your account…"
                            } else if self.library_loading {
                                "Loading your playlists…"
                            } else if self.library_error.is_some() {
                                "Couldn't load your playlists."
                            } else if owned_count == 0 {
                                "Create your first playlist above."
                            } else {
                                "No matching playlists."
                            })
                            .color(colours.muted),
                        );
                        if user_id.is_empty() && self.profile_error.is_some() {
                            ui.label(
                                RichText::new("Couldn't verify your playlist ownership.")
                                    .color(colours.muted),
                            );
                            if compact_pill_button(ui, "Try again", Some(Icon::Refresh), false)
                                .clicked()
                            {
                                self.load_home_data(ctx);
                            }
                        }
                        if !user_id.is_empty()
                            && self.library_error.is_some()
                            && compact_pill_button(ui, "Try again", Some(Icon::Refresh), false)
                                .clicked()
                        {
                            self.load_library(ctx, true);
                        }
                        ui.add_space(8.0);
                    }
                    for playlist in playlists {
                        let added = self.playlist_picker_added.contains(&playlist.id);
                        let pending = self.playlist_add_pending.as_deref() == Some(&playlist.id)
                            || (self.membership_loading
                                && !self.playlist_membership.contains_key(&playlist.id));
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 50.0),
                            Sense::click(),
                        );
                        if response.hovered() && !busy {
                            ui.painter().rect_filled(rect, 6.0, colours.hover);
                        }
                        let art =
                            Rect::from_min_size(rect.min + Vec2::new(4.0, 8.0), Vec2::splat(36.0));
                        self.art(ui, art, playlist.image_url.as_deref(), &playlist.name, ctx);
                        text(
                            ui,
                            rect.min + Vec2::new(50.0, 8.0),
                            &playlist.name,
                            13.0,
                            colours.text,
                            rect.width() - 80.0,
                        );
                        let subtitle = if added {
                            "Added".to_owned()
                        } else if playlist.items_accessible {
                            format!(
                                "{} {}",
                                playlist.total_items,
                                if playlist.total_items == 1 {
                                    "song"
                                } else {
                                    "songs"
                                }
                            )
                        } else {
                            "Playlist".into()
                        };
                        text(
                            ui,
                            rect.min + Vec2::new(50.0, 29.0),
                            &subtitle,
                            11.0,
                            if added { colours.green } else { colours.muted },
                            rect.width() - 80.0,
                        );
                        let check = Rect::from_center_size(
                            Pos2::new(rect.right() - 13.0, rect.center().y),
                            Vec2::splat(18.0),
                        );
                        if pending {
                            ui.put(check, egui::Spinner::new().size(16.0));
                        } else if added {
                            ui.painter()
                                .circle_filled(check.center(), 8.0, colours.green);
                            icon_at(ui, check, Icon::Check, colours.panel);
                        } else {
                            ui.painter().circle_stroke(
                                check.center(),
                                7.0,
                                Stroke::new(1.0_f32, colours.muted),
                            );
                            icon_at(ui, check.shrink(3.0), Icon::Plus, colours.muted);
                        }
                        let response = response
                            .on_hover_text(if added {
                                format!("Remove from {}", playlist.name)
                            } else {
                                format!("Add to {}", playlist.name)
                            })
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if response.clicked() && !busy && !pending {
                            if added {
                                self.remove_track_from_playlist(playlist, track.clone(), ctx);
                            } else {
                                self.add_track_to_playlist(playlist, track.clone(), ctx);
                            }
                        }
                    }
                });
            if let Some(error) = self.playlist_add_error.clone() {
                ui.add(
                    egui::Label::new(
                        RichText::new(error)
                            .size(11.0)
                            .color(Color32::from_rgb(246, 158, 149)),
                    )
                    .wrap(),
                );
            }
            ui.separator();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if compact_pill_button(ui, "Done", Some(Icon::Check), false).clicked() {
                    ui.close();
                }
                ui.menu_button("Song options", |ui| {
                    if ui.button("Play now").clicked() {
                        self.play_track(track.clone(), ctx);
                        ui.close();
                    }
                    if ui.button("Add to queue").clicked() {
                        self.add_to_queue(track.clone(), ctx);
                        ui.close();
                    }
                });
                if busy {
                    ui.label(RichText::new("Adding…").size(11.0).color(colours.muted));
                }
            });
        });
        self.playlist_picker_rect = popup.map(|popup| popup.response.rect);
        self.show_add_to_playlist = open && self.show_add_to_playlist;
    }

    fn draw_profile(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        heading(ui, "Your profile", "Your Spotify account and library.");
        card(ui.ctx()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (r, _) = ui.allocate_exact_size(Vec2::splat(96.0), Sense::hover());
                self.avatar(ui, r, ctx);
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("SPOTIFY ACCOUNT")
                            .size(10.0)
                            .strong()
                            .color(colours.green),
                    );
                    ui.label(
                        RichText::new(
                            self.user_name
                                .as_deref()
                                .unwrap_or("Your music starts here"),
                        )
                        .size(30.0)
                        .strong(),
                    );
                    ui.label(
                        RichText::new(if self.is_connected() {
                            "Connected to Spotify"
                        } else {
                            "Connect your account to personalize Aurafy."
                        })
                        .size(13.0)
                        .color(colours.muted),
                    );
                });
            });
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                if self.is_connected() {
                    if action_button(ui, "Manage account", None, false).clicked() {
                        self.settings_tab = SettingsTab::Account;
                        self.navigate(Page::Settings, ctx);
                    }
                    if let Some(url) = self
                        .user_profile
                        .as_ref()
                        .and_then(|p| p.profile_url.clone())
                    {
                        if action_button(ui, "View on Spotify", None, false).clicked() {
                            if let Err(e) = browser::open_url(&url) {
                                self.set_status(e);
                            }
                        }
                    }
                } else if ui.add(primary("Set up Spotify", ui.ctx())).clicked() {
                    self.open_spotify_setup(ctx);
                }
            });
        });
        if let Some(error) = self.profile_error.clone() {
            ui.add_space(12.0);
            self.error_notice(ui, &error);
        }
        ui.add_space(18.0);
        let width = (ui.available_width() - 24.0) / 3.0;
        ui.horizontal(|ui| {
            for (n, label, page) in [
                (self.playlists.len(), "Playlists loaded", Page::Library),
                (self.saved_tracks.len(), "Liked songs loaded", Page::Liked),
                (
                    self.recent_tracks.len(),
                    "Recent tracks loaded",
                    Page::Recent,
                ),
            ] {
                let (r, response) = ui.allocate_exact_size(Vec2::new(width, 88.0), Sense::click());
                ui.painter().rect_filled(
                    r,
                    10.0,
                    if response.hovered() {
                        colours.hover
                    } else {
                        colours.panel2
                    },
                );
                text(
                    ui,
                    r.min + Vec2::new(18.0, 12.0),
                    &n.to_string(),
                    27.0,
                    colours.text,
                    width - 36.0,
                );
                text(
                    ui,
                    r.min + Vec2::new(18.0, 54.0),
                    label,
                    11.0,
                    colours.muted,
                    width - 36.0,
                );
                if response.clicked() {
                    self.navigate(page, ctx);
                }
            }
        });
        if !self.playlists.is_empty() {
            ui.add_space(24.0);
            ui.label(RichText::new("Your playlists").size(21.0).strong());
            ui.add_space(8.0);
            self.playlist_cards(ui, ctx, self.playlists.iter().take(6).cloned().collect());
        }
    }
}
