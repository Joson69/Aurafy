use super::*;

impl AurafyApp {
    fn device_picker(&mut self, ctx: &egui::Context, button: &egui::Response) {
        let colours = appearance::palette(ctx);
        self.device_button_rect = Some(button.rect);
        if button.clicked() {
            self.device_picker_open = !self.device_picker_open;
            if self.device_picker_open && self.is_connected() {
                self.load_devices(ctx);
            }
        }
        let mut open = self.device_picker_open;
        let mut selected = None;
        let popup = egui::Popup::from_response(button)
            .id(Id::new("device-picker"))
            .open_bool(&mut open)
            .align(egui::emath::RectAlign::TOP_END)
            .align_alternatives(&[egui::emath::RectAlign::TOP_END])
            .gap(10.0)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .frame(
                Frame::popup(&ctx.style())
                    .fill(colours.panel2)
                    .inner_margin(Margin::same(12)),
            )
            .show(|ui| {
                ui.set_width(288.0);
                ui.spacing_mut().item_spacing.y = 4.0;
                let lang = self.config.language;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(i18n::tr(lang, i18n::Text::ConnectToDevice))
                            .size(14.0)
                            .strong()
                            .color(colours.text),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let refresh = ui
                            .add_enabled_ui(!self.devices_loading && self.is_connected(), |ui| {
                                icon_button(
                                    ui,
                                    Icon::Refresh,
                                    i18n::tr(lang, i18n::Text::RefreshDevices),
                                    false,
                                )
                            })
                            .inner;
                        if refresh.clicked() {
                            self.load_devices(ctx);
                        }
                    });
                });
                let devices = self.playback_devices();
                ScrollArea::vertical()
                    .id_salt("device-picker-list")
                    .max_height(270.0)
                    .show(ui, |ui| {
                        ui.set_width(288.0);
                        for device in devices {
                            let (r, response) =
                                ui.allocate_exact_size(Vec2::new(288.0, 54.0), Sense::click());
                            if response.hovered() {
                                ui.painter().rect_filled(r, 6.0, colours.hover);
                            }
                            let color = if device.is_active {
                                colours.green
                            } else {
                                colours.text
                            };
                            let kind = match device.device_type.to_ascii_lowercase().as_str() {
                                "smartphone" | "tablet" => Icon::Phone,
                                "speaker" | "speakers" => Icon::Volume,
                                _ => Icon::Device,
                            };
                            icon_at(
                                ui,
                                Rect::from_min_size(
                                    r.min + Vec2::new(6.0, 15.0),
                                    Vec2::splat(23.0),
                                ),
                                kind,
                                color,
                            );
                            text(
                                ui,
                                r.min + Vec2::new(42.0, 9.0),
                                &device.name,
                                13.0,
                                color,
                                r.width() - 64.0,
                            );
                            let subtitle = if device.is_active {
                                if self.is_playing {
                                    "Listening on this device"
                                } else {
                                    "Selected for playback"
                                }
                            } else if device.id == "auralis-local" {
                                "This computer · Play here"
                            } else {
                                &device.device_type
                            };
                            text(
                                ui,
                                r.min + Vec2::new(42.0, 30.0),
                                subtitle,
                                11.0,
                                if device.is_active {
                                    colours.green
                                } else {
                                    colours.muted
                                },
                                r.width() - 64.0,
                            );
                            if device.is_active {
                                ui.painter().circle_filled(
                                    r.right_center() - Vec2::new(9.0, 0.0),
                                    3.0,
                                    colours.green,
                                );
                            }
                            if response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                                && !device.is_active
                            {
                                selected = Some(device);
                            }
                        }
                    });
                if let Some(error) = &self.devices_error {
                    ui.label(RichText::new(error).size(11.0).color(colours.muted));
                }
                if self.playback_devices().is_empty() && !self.devices_loading {
                    ui.label(
                        RichText::new("Open Spotify on your other device, then refresh.")
                            .size(12.0)
                            .color(colours.muted),
                    );
                }
                if !self.is_connected() {
                    ui.label(
                        RichText::new("Sign in to find your other devices.")
                            .size(11.0)
                            .color(colours.muted),
                    );
                }
                ui.separator();
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                    icon_at(ui, r, Icon::Volume, colours.muted);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(i18n::tr(lang, i18n::Text::AudioOutput))
                                .size(11.5)
                                .color(colours.text),
                        );
                        ui.label(
                            RichText::new(i18n::tr(lang, i18n::Text::SystemDefault))
                                .size(11.0)
                                .color(colours.muted),
                        );
                    });
                });
            });
        self.device_popup_rect = popup.map(|popup| popup.response.rect);
        self.device_picker_open = open;
        if let Some(device) = selected {
            self.transfer_to_device(device, ctx);
            self.device_picker_open = false;
        }
    }
    pub(crate) fn draw_queue(&mut self, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        if !self.show_queue {
            return;
        }
        SidePanel::right("queue")
            .exact_width(284.0)
            .resizable(false)
            .frame(Frame::new().fill(colours.panel).inner_margin(Margin {
                left: 18,
                right: 18,
                top: 18,
                bottom: 0,
            }))
            .show(ctx, |ui| {
                let lang = self.config.language;
                ui.horizontal(|ui| {
                    if ui
                        .add(pill(
                            i18n::tr(lang, i18n::Text::Queue),
                            !self.queue_show_recent,
                            ctx,
                        ))
                        .clicked()
                    {
                        self.queue_show_recent = false;
                    }
                    if ui
                        .add(pill(
                            i18n::tr(lang, i18n::Text::RecentlyPlayed),
                            self.queue_show_recent,
                            ctx,
                        ))
                        .clicked()
                    {
                        self.queue_show_recent = true;
                        self.load_home_data(ctx);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if icon_button(ui, Icon::Close, "Close queue", false).clicked() {
                            self.show_queue = false;
                            self.config.queue_open = false;
                            self.persist_preferences();
                        }
                    });
                });
                ui.add_space(17.0);
                if self.queue_show_recent {
                    let count = self.recent_tracks.len();
                    if count == 0 {
                        if self.recent_loading {
                            ui.spinner();
                        } else if self.recent_error.is_some() {
                            ui.label(
                                RichText::new("Couldn't load listening history.")
                                    .size(12.0)
                                    .color(colours.muted),
                            );
                        } else {
                            ui.label(
                                RichText::new("No listening history available.")
                                    .size(12.0)
                                    .color(colours.muted),
                            );
                        }
                    } else {
                        ScrollArea::vertical()
                            .id_salt("queue-recent-list")
                            .show_rows(ui, 60.0, count, |ui, rows| {
                                for i in rows {
                                    let track = self.recent_tracks[i].clone();
                                    let _ = self.queue_row(ui, ctx, &track, false, None);
                                }
                            });
                    }
                    return;
                }
                ui.label(
                    RichText::new("NOW PLAYING")
                        .size(10.0)
                        .strong()
                        .color(colours.muted),
                );
                ui.add_space(5.0);
                let current = self.current_track.clone();
                let _ = self.queue_row(ui, ctx, &current, true, None);
                ui.add_space(22.0);
                ui.horizontal(|ui| {
                    let queue_title = if self.repeat_state == "track" {
                        "AFTER SONG REPEAT"
                    } else {
                        i18n::tr(lang, i18n::Text::UpNext)
                    };
                    let total_count = self.priority_queue.len() + self.queue.len();
                    let header_text = queue_title.to_owned();
                    if total_count > 0 {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Clear").size(11.0).color(colours.muted),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text("Clear Up next queue")
                                .clicked()
                            {
                                let _ = self.clear_up_next_queue();
                            }
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(header_text)
                                            .size(10.0)
                                            .strong()
                                            .color(colours.muted),
                                    )
                                    .truncate(),
                                );
                            });
                        });
                    } else {
                        ui.label(
                            RichText::new(header_text)
                                .size(10.0)
                                .strong()
                                .color(colours.muted),
                        );
                    }
                });
                let manual_len = self.manual_queue_len.min(self.priority_queue.len());
                let collection_len = self.priority_queue.len().saturating_sub(manual_len);
                if !self.priority_queue.is_empty() {
                    ui.add_space(6.0);
                    let label = if manual_len > 0 && collection_len > 0 {
                        if self.active_collection_uri.is_some() {
                            format!("Queued next ({manual_len}) · Next from playlist")
                        } else {
                            format!("Queued next ({manual_len}) · Autoplay")
                        }
                    } else if manual_len > 0 {
                        format!("Queued next ({manual_len})")
                    } else if self.active_collection_uri.is_some() {
                        "Next from playlist".to_owned()
                    } else {
                        "Autoplay".to_owned()
                    };
                    ui.label(RichText::new(label).size(11.0).color(colours.muted));
                }
                if let Some(timer) = &self.sleep_timer {
                    let sleep_text = match timer {
                        crate::preferences::SleepTimer::Deadline(deadline) => {
                            let secs = deadline.saturating_duration_since(Instant::now()).as_secs();
                            let mins = ((secs + 59) / 60).max(1);
                            format!("Sleep timer · {mins}m left")
                        }
                        crate::preferences::SleepTimer::EndOfTrack { .. } => {
                            "Sleep timer · End of song".to_owned()
                        }
                    };
                    ui.add_space(6.0);
                    if compact_pill_button(ui, &sleep_text, Some(Icon::Clock), false)
                        .on_hover_text("Click to cancel sleep timer")
                        .clicked()
                    {
                        self.sleep_timer = None;
                        self.set_status("Sleep timer cancelled");
                    }
                }
                ui.add_space(5.0);
                let priority_len = self.priority_queue.len();
                let count = priority_len + self.queue.len();
                let mut remove_priority = None;
                let mut play_priority = None;
                if count == 0 {
                    ui.label(
                        RichText::new("Find a song and add it to your queue.")
                            .size(12.0)
                            .color(colours.muted),
                    );
                } else {
                    ScrollArea::vertical().id_salt("queue-list").show_rows(
                        ui,
                        60.0,
                        count,
                        |ui, rows| {
                            for i in rows {
                                if i < priority_len {
                                    if let Some(track) = self.priority_queue.get(i).cloned()
                                        && let Some(action) =
                                            self.queue_row(ui, ctx, &track, false, Some(i))
                                    {
                                        if action {
                                            play_priority = Some(i);
                                        } else {
                                            remove_priority = Some(i);
                                        }
                                    }
                                } else if let Some(track) =
                                    self.queue.get(i - priority_len).cloned()
                                {
                                    self.queue_row(ui, ctx, &track, false, None);
                                }
                            }
                        },
                    );
                }
                if let Some(idx) = play_priority
                    && idx < self.priority_queue.len()
                {
                    let track = self.priority_queue.remove(idx);
                    if idx < self.manual_queue_len {
                        self.manual_queue_len = self.manual_queue_len.saturating_sub(1);
                    }
                    let collection = self.active_collection_uri.take();
                    self.dispatch_track_play(track, ctx);
                    self.active_collection_uri = collection;
                } else if let Some(idx) = remove_priority
                    && idx < self.priority_queue.len()
                {
                    self.priority_queue.remove(idx);
                    if idx < self.manual_queue_len {
                        self.manual_queue_len = self.manual_queue_len.saturating_sub(1);
                    }
                }
            });
    }

    fn queue_row(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        track: &Track,
        current: bool,
        priority_index: Option<usize>,
    ) -> Option<bool> {
        let colours = appearance::palette(ctx);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 60.0), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(rect, 8.0, colours.hover);
        }
        let art_rect = Rect::from_min_size(rect.min + Vec2::new(4.0, 8.0), Vec2::splat(42.0));
        self.art(ui, art_rect, track.artwork_url.as_deref(), &track.name, ctx);
        if response.hovered() && !current {
            ui.painter()
                .rect_filled(art_rect, 6.0, Color32::from_rgba_unmultiplied(0, 0, 0, 145));
            icon_at(ui, art_rect, Icon::Play, Color32::WHITE);
        }
        let mut remove_clicked = false;
        let text_trim = if priority_index.is_some() { 86.0 } else { 61.0 };
        if let Some(idx) = priority_index {
            let rm_rect = Rect::from_center_size(
                Pos2::new(rect.right() - 14.0, rect.center().y),
                Vec2::splat(22.0),
            );
            let rm_resp = ui.interact(rm_rect, ui.id().with(("q-rm", idx)), Sense::click());
            if rm_resp.hovered() {
                ui.painter().rect_filled(rm_rect, 6.0, colours.hover);
            }
            icon_at(
                ui,
                rm_rect,
                Icon::Close,
                if rm_resp.hovered() {
                    colours.text
                } else {
                    colours.muted
                },
            );
            remove_clicked = rm_resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked();
        }
        text(
            ui,
            rect.min + Vec2::new(57.0, 9.0),
            &track.name,
            12.5,
            if current { colours.green } else { colours.text },
            (rect.width() - text_trim).max(20.0),
        );
        let artist_clicked = self.artist_links(
            ui,
            ctx,
            track,
            rect.min + Vec2::new(57.0, 31.0),
            11.0,
            (rect.width() - text_trim).max(20.0),
        );
        if remove_clicked {
            return Some(false);
        }
        let art_clicked = response.clicked()
            && response
                .interact_pointer_pos()
                .is_some_and(|p| art_rect.contains(p));
        if !artist_clicked && (response.double_clicked() || (art_clicked && !current)) {
            if priority_index.is_some() {
                return Some(true);
            }
            self.play_track(track.clone(), ctx);
        }
        None
    }

    pub(crate) fn draw_player(&mut self, ctx: &egui::Context) {
        self.apply_display_preferences(ctx);
        self.ensure_playlist_membership(ctx, false);
        let colours = appearance::palette(ctx);
        TopBottomPanel::bottom("player")
            .exact_height(player_layout::Geometry::HEIGHT)
            .show_separator_line(false)
            .frame(Frame::new().fill(colours.player).inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                let full = ui.available_rect_before_wrap();
                let track = self.current_track.clone();
                let measure = |value: &str, font| {
                    ui.painter()
                        .layout_no_wrap(value.to_owned(), font, colours.text)
                        .size()
                        .x
                };
                let title_width = measure(&track.name, player_font(13.0, true));
                let artist_width = if track.artist_names.is_empty() {
                    measure(&track.artist, player_font(11.0, false))
                } else {
                    track
                        .artist_names
                        .iter()
                        .map(|name| measure(name, player_font(11.0, false)))
                        .sum::<f32>()
                        + track.artist_names.len().saturating_sub(1) as f32 * 9.0
                };
                let layout =
                    player_layout::Geometry::new(full, title_width.max(artist_width) + 2.0);
                self.player_layout = Some(layout);
                self.art_rounded(
                    ui,
                    layout.artwork,
                    track.artwork_url.as_deref(),
                    &track.name,
                    ctx,
                    6,
                );
                let title_rect = layout.title;
                text_with_font(
                    ui,
                    title_rect.min,
                    &track.name,
                    player_font(13.0, true),
                    colours.player_text,
                    title_rect.width(),
                );
                let title = ui
                    .interact(
                        title_rect,
                        ui.id().with("current-song-title"),
                        Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if title.hovered() {
                    let width = ui
                        .painter()
                        .layout_no_wrap(track.name.clone(), player_font(13.0, true), colours.text)
                        .size()
                        .x
                        .min(title_rect.width());
                    ui.painter().line_segment(
                        [
                            title_rect.min + Vec2::new(0.0, 15.0),
                            title_rect.min + Vec2::new(width, 15.0),
                        ],
                        Stroke::new(0.7_f32, colours.text),
                    );
                }
                if title.clicked() {
                    self.open_album(track.clone(), ctx);
                }
                self.artist_links_with_font(
                    ui,
                    ctx,
                    &track,
                    layout.artist.min,
                    player_font(11.0, false),
                    layout.artist.width(),
                );
                let saved = self.saved_track_ids.contains(&track.id);
                let pending_save = self.library_changes.get(&track.id).map(|(_, save)| *save);
                let in_playlist = !self.known_memberships(&track.uri).is_empty()
                    || (self.playlist_picker_track_uri == track.uri
                        && !self.playlist_picker_added.is_empty());
                let collection = player_action(
                    ui,
                    layout.collection,
                    "current-collection",
                    if pending_save.is_some() {
                        Icon::Clock
                    } else if saved || in_playlist {
                        Icon::CircleCheck
                    } else {
                        Icon::CirclePlus
                    },
                    "Save to your music",
                    saved || in_playlist,
                );
                self.current_track_menu_rect = Some(collection.rect);
                if collection.clicked() {
                    self.open_playlist_picker(track.clone());
                }
                if player_action(
                    ui,
                    layout.queue_current,
                    "current-track-queue",
                    Icon::Queue,
                    "Add current song to queue",
                    false,
                )
                .clicked()
                {
                    self.add_to_queue(track.clone(), ctx);
                }
                collection.context_menu(|ui| {
                    if ui.button("Play now").clicked() {
                        self.play_track(track.clone(), ctx);
                        ui.close();
                    }
                    if ui.button("Add to queue").clicked() {
                        self.add_to_queue(track.clone(), ctx);
                        ui.close();
                    }
                });
                let lang = self.config.language;
                if player_action(
                    ui,
                    layout.transport[0],
                    "player-shuffle",
                    Icon::Shuffle,
                    i18n::tr(lang, i18n::Text::Shuffle),
                    self.shuffle_enabled,
                )
                .clicked()
                {
                    self.shuffle_enabled = !self.shuffle_enabled;
                    let enabled = self.shuffle_enabled;
                    if self.is_connected() || self.use_local_playback() {
                        self.start_spotify_action(
                            "shuffle".into(),
                            move |t| spotify::set_shuffle(t, enabled),
                            ctx,
                        );
                    }
                }
                if player_action(
                    ui,
                    layout.transport[1],
                    "player-previous",
                    Icon::Previous,
                    &format!("{} · Ctrl+Left", i18n::tr(lang, i18n::Text::Previous)),
                    false,
                )
                .clicked()
                {
                    self.send_playback_action("previous", ctx);
                }
                let play = layout.transport[2];
                let response = ui.interact(play, ui.id().with("player-play"), Sense::click());
                let play_label = if self.is_playing {
                    i18n::tr(lang, i18n::Text::Pause)
                } else {
                    i18n::tr(lang, i18n::Text::Play)
                };
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, play_label)
                });
                ui.painter().circle_filled(
                    play.center(),
                    play.width() * 0.5,
                    if response.is_pointer_button_down_on() {
                        colours.player_muted
                    } else {
                        colours.player_text
                    },
                );
                icon_at(
                    ui,
                    play.shrink(6.0),
                    if self.is_playing {
                        Icon::Pause
                    } else {
                        Icon::Play
                    },
                    colours.player,
                );
                if response
                    .on_hover_text(format!("{play_label} · Space"))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    self.send_playback_action(
                        if self.is_playing { "pause" } else { "resume" },
                        ctx,
                    );
                }
                if player_action(
                    ui,
                    layout.transport[3],
                    "player-next",
                    Icon::Next,
                    &format!("{} · Ctrl+Right", i18n::tr(lang, i18n::Text::Next)),
                    false,
                )
                .clicked()
                {
                    self.send_playback_action("next", ctx);
                }
                let repeat = layout.transport[4];
                if player_action(
                    ui,
                    repeat,
                    "player-repeat",
                    Icon::Repeat,
                    &format!(
                        "{}: {}",
                        i18n::tr(lang, i18n::Text::Repeat),
                        self.repeat_state
                    ),
                    self.repeat_state != "off",
                )
                .clicked()
                {
                    self.repeat_state = match self.repeat_state.as_str() {
                        "off" => "context",
                        "context" => "track",
                        _ => "off",
                    }
                    .into();
                    let state = self.repeat_state.clone();
                    if self.is_connected() || self.use_local_playback() {
                        self.start_spotify_action(
                            "repeat".into(),
                            move |t| spotify::set_repeat(t, &state),
                            ctx,
                        );
                    }
                }
                if self.repeat_state == "track" {
                    ui.painter().text(
                        repeat.center(),
                        Align2::CENTER_CENTER,
                        "1",
                        FontId::proportional(8.0),
                        colours.green,
                    );
                }
                ui.painter().text(
                    layout.elapsed,
                    Align2::RIGHT_CENTER,
                    format_time(self.current_progress_ms() / 1000),
                    player_font(11.0, false),
                    colours.player_muted,
                );
                let seek_rect = layout.seek;
                let seek = ui
                    .scope_builder(egui::UiBuilder::new().max_rect(seek_rect), |ui| {
                        player_slider(ui, &mut self.progress, seek_rect.width(), "Seek")
                    })
                    .inner;
                let seeking = seek.is_pointer_button_down_on();
                let seek_finished = self.seek_interacting && !seeking;
                self.seek_interacting = seeking;
                if seek_finished || (seek.changed() && !seeking) {
                    self.seek_playback((self.progress * self.duration_ms as f32) as u64, ctx);
                }
                ui.painter().text(
                    layout.duration,
                    Align2::LEFT_CENTER,
                    format_time(self.duration_ms / 1000),
                    player_font(11.0, false),
                    colours.player_muted,
                );

                if player_action(
                    ui,
                    layout.utilities[0],
                    "player-lyrics",
                    Icon::Lyrics,
                    i18n::tr(lang, i18n::Text::Lyrics),
                    self.page == Page::Lyrics,
                )
                .clicked()
                {
                    self.toggle_lyrics();
                }
                if player_action(
                    ui,
                    layout.utilities[1],
                    "player-queue",
                    Icon::Queue,
                    "Show queue",
                    self.show_queue,
                )
                .clicked()
                {
                    self.show_queue = !self.show_queue;
                    self.config.queue_open = self.show_queue;
                    self.persist_preferences();
                }
                let output = self
                    .selected_output_name()
                    .map(|n| {
                        if self.is_playing {
                            format!("Playing on {n}")
                        } else {
                            format!("Output: {n}")
                        }
                    })
                    .unwrap_or_else(|| {
                        if self.is_demo_mode() {
                            "Demo · no audio".into()
                        } else if self.local_starting {
                            "Restoring Aurafy…".into()
                        } else {
                            "Choose a playback device".into()
                        }
                    });
                let device = player_action(
                    ui,
                    layout.utilities[2],
                    "player-device",
                    Icon::Device,
                    &format!("{output} · Choose playback device"),
                    self.device_picker_open,
                );
                if self.selected_output_name().is_some() {
                    ui.painter().circle_filled(
                        Pos2::new(device.rect.center().x, device.rect.bottom() - 1.5),
                        1.6,
                        colours.green,
                    );
                }
                self.device_picker(ctx, &device);
                let muted = self.volume <= 0.0;
                let mut volume_changed = false;
                if player_action(
                    ui,
                    layout.utilities[3],
                    "player-mute",
                    if muted { Icon::Mute } else { Icon::Volume },
                    if muted {
                        i18n::tr(lang, i18n::Text::Unmute)
                    } else {
                        i18n::tr(lang, i18n::Text::Mute)
                    },
                    false,
                )
                .clicked()
                {
                    self.volume = if muted {
                        self.unmuted_volume.max(0.01)
                    } else {
                        self.unmuted_volume = self.volume;
                        0.0
                    };
                    volume_changed = true;
                }
                let volume_rect = layout.volume;
                let volume = ui
                    .scope_builder(egui::UiBuilder::new().max_rect(volume_rect), |ui| {
                        player_slider(ui, &mut self.volume, volume_rect.width(), "Volume")
                    })
                    .inner;
                if self.volume > 0.0 {
                    self.unmuted_volume = self.volume;
                }
                let held = volume.is_pointer_button_down_on();
                let finished = volume_changed
                    || (self.volume_control.interacting && !held)
                    || (volume.changed() && !held);
                self.volume_control.interacting = held;
                if volume_changed || volume.changed() || finished {
                    self.queue_volume(finished, ctx);
                }
                self.flush_volume(ctx);
                let fullscreen = ctx.input(|input| input.viewport().fullscreen.unwrap_or(false));
                if player_action(
                    ui,
                    layout.utilities[4],
                    "player-fullscreen",
                    Icon::Fullscreen,
                    if fullscreen {
                        "Exit fullscreen"
                    } else {
                        i18n::tr(lang, i18n::Text::Fullscreen)
                    },
                    fullscreen,
                )
                .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!fullscreen));
                }
            });
    }

    pub(super) fn draw_lyrics(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        self.request_lyrics(ctx);
        let track = self.current_track.clone();
        let key = lyrics::TrackKey::for_track(&track);
        let lang = self.config.language;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(i18n::tr(lang, i18n::Text::Lyrics))
                    .size(30.0)
                    .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(
                    ui,
                    Icon::Close,
                    i18n::tr(lang, i18n::Text::CloseLyrics),
                    false,
                )
                .clicked()
                {
                    self.toggle_lyrics();
                }
            });
        });
        ui.add(egui::Label::new(RichText::new(&track.name).size(15.0).strong()).truncate())
            .on_hover_text(&track.name);
        ui.label(RichText::new(&track.artist).size(12.0).color(colours.muted));
        ui.add_space(12.0);
        let Some(key) = key else {
            ui.label(RichText::new("Play a song to see its lyrics.").color(colours.muted));
            return;
        };
        let result = self.lyrics.get(&key).cloned();
        let synced = result
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .and_then(|r| r.as_ref())
            .is_some_and(|lyrics| !lyrics.lines.is_empty());
        if self
            .lyrics_last_line
            .as_ref()
            .is_some_and(|(previous, _)| previous != &key)
        {
            self.lyrics_follow = !self.config.lyrics_manual_by_default;
            self.lyrics_last_line = None;
        }
        let copy_text = match &result {
            Some(Ok(Some(lyrics))) if !lyrics.instrumental => {
                if !lyrics.lines.is_empty() {
                    Some(
                        lyrics
                            .lines
                            .iter()
                            .map(|l| l.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                } else if !lyrics.plain.trim().is_empty() {
                    Some(lyrics.plain.clone())
                } else {
                    None
                }
            }
            _ => None,
        };
        ui.horizontal(|ui| {
            if compact_pill_button(ui, "LRCLIB ↗", None, false)
                .on_hover_text("Lyrics provided by LRCLIB. Looks up this song's title, artist, album and duration.")
                .clicked()
            {
                let _ = browser::open_url("https://lrclib.net");
            }
            if synced {
                ui.label(RichText::new("· Synced lyrics").size(11.5).color(colours.muted));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if synced
                    && ui
                        .add(pill(
                            i18n::tr(lang, i18n::Text::FollowSong),
                            self.lyrics_follow,
                            ctx,
                        ))
                        .clicked()
                {
                    self.lyrics_follow = !self.lyrics_follow;
                    self.lyrics_last_line = None;
                }
                if let Some(text_to_copy) = &copy_text
                    && compact_pill_button(ui, "Copy", None, false)
                        .on_hover_text("Copy lyrics")
                        .clicked()
                {
                    ctx.copy_text(text_to_copy.clone());
                    self.set_status("Copied lyrics to clipboard");
                }
                let size = self.config.lyrics_font_size.clamp(24, 44);
                if ui
                    .add_enabled_ui(size < 44, |ui| compact_pill_button(ui, "A+", None, false))
                    .inner
                    .on_hover_text("Larger lyrics")
                    .clicked()
                {
                    self.config.lyrics_font_size = (size + 2).min(44);
                    self.persist_preferences();
                }
                if ui
                    .add_enabled_ui(size > 24, |ui| compact_pill_button(ui, "A-", None, false))
                    .inner
                    .on_hover_text("Smaller lyrics")
                    .clicked()
                {
                    self.config.lyrics_font_size = (size - 2).max(24);
                    self.persist_preferences();
                }
            });
        });
        ui.add_space(10.0);
        let panel_height = ui.available_height().max(80.0);
        let mut seek_to = None;
        Frame::new()
            .fill(colours.lyrics)
            .corner_radius(12)
            .inner_margin(Margin {
                left: 24,
                right: 12,
                top: 18,
                bottom: 18,
            })
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(
                    ui.available_width(),
                    (panel_height - 36.0).max(32.0),
                ));
                match result {
                    None if self.lyrics.wait_seconds() > 0 && self.lyrics.pending.is_none() => {
                        ui.label(RichText::new("Lyrics will load shortly…").color(colours.text));
                    }
                    None => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(RichText::new("Finding lyrics…").color(colours.text));
                        });
                    }
                    Some(Ok(Some(lyrics))) if !lyrics.instrumental => {
                        let active = lyrics.active_line(self.current_progress_ms());
                        let position_changed =
                            self.lyrics_last_line.as_ref() != Some(&(key.clone(), active));
                        let scroll_rect = ui.available_rect_before_wrap();
                        let manual_scroll = ui.input(|i| {
                            i.pointer
                                .hover_pos()
                                .is_some_and(|pos| scroll_rect.contains(pos))
                                && (i.raw_scroll_delta.y != 0.0
                                    || (i.pointer.primary_down()
                                        && i.pointer.delta().y.abs() > 1.0))
                        });
                        if manual_scroll {
                            self.lyrics_follow = false;
                        }
                        ScrollArea::vertical()
                            .id_salt(("lyrics-lines", &key.title, &key.artist, key.seconds))
                            .auto_shrink([false, false])
                            .max_height((panel_height - 36.0).max(32.0))
                            .show(ui, |ui| {
                                ui.set_width((ui.available_width() - 16.0).max(1.0));
                                ui.spacing_mut().item_spacing.y = 14.0;
                                let lyric_size =
                                    self.config.lyric_size(ui.available_width() < 600.0);
                                if synced {
                                    ui.add_space(12.0);
                                    for (index, line) in lyrics.lines.iter().enumerate() {
                                        let color = if active == Some(index) {
                                            colours.text
                                        } else if active.is_some_and(|i| index < i) {
                                            colours.lyrics_past
                                        } else {
                                            colours.lyrics_future
                                        };
                                        let response = ui
                                            .add(
                                                egui::Label::new(
                                                    RichText::new(if line.text.is_empty() {
                                                        "···"
                                                    } else {
                                                        &line.text
                                                    })
                                                    .font(player_font(lyric_size, true))
                                                    .color(color),
                                                )
                                                .wrap()
                                                .sense(Sense::click()),
                                            )
                                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                                        if self.lyrics_follow
                                            && position_changed
                                            && (active == Some(index)
                                                || (active.is_none() && index == 0))
                                        {
                                            response.scroll_to_me(Some(Align::Center));
                                        }
                                        if response.clicked() {
                                            seek_to = Some(line.time_ms);
                                        }
                                    }
                                    ui.add_space((panel_height * 0.35).min(180.0));
                                } else {
                                    for line in lyrics.plain.lines() {
                                        ui.label(
                                            RichText::new(if line.is_empty() { " " } else { line })
                                                .font(player_font(lyric_size * 0.75, false))
                                                .color(colours.text),
                                        );
                                    }
                                    ui.add_space(24.0);
                                }
                            });
                        self.lyrics_last_line = Some((key.clone(), active));
                    }
                    value => {
                        ui.add_space(18.0);
                        let (title, message) = match &value {
                            Some(Ok(Some(_))) => ("Instrumental", "This song has no lyrics."),
                            Some(Err(error)) => ("Couldn't load lyrics", error.message.as_str()),
                            _ => (
                                "No lyrics found",
                                "LRCLIB doesn't have lyrics for this song yet.",
                            ),
                        };
                        ui.label(RichText::new(title).size(24.0).strong());
                        ui.label(RichText::new(message).color(colours.lyrics_future));
                        if !matches!(value, Some(Ok(Some(_)))) {
                            let wait = self.lyrics.wait_seconds();
                            let label = if wait > 0 {
                                format!("Try again in {wait}s")
                            } else {
                                "Try again".into()
                            };
                            if ui
                                .add_enabled_ui(wait == 0 && self.lyrics.pending.is_none(), |ui| {
                                    action_button(ui, &label, Some(Icon::Refresh), false)
                                })
                                .inner
                                .clicked()
                            {
                                self.lyrics.retry(&key);
                                self.request_lyrics(ctx);
                            }
                        }
                    }
                }
            });
        if let Some(position) = seek_to {
            self.seek_playback(position, ctx);
            self.lyrics_follow = !self.config.lyrics_manual_by_default;
            self.lyrics_last_line = None;
        }
    }
}
