use super::*;

impl AurafyApp {
    pub(super) fn account_menu(&mut self, ctx: &egui::Context, response: egui::Response) {
        let colours = appearance::palette(ctx);
        let lang = self.config.language;
        egui::Popup::menu(&response)
            .frame(
                Frame::popup(&ctx.style())
                    .fill(colours.panel2)
                    .inner_margin(Margin::same(10)),
            )
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(260.0);
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.label(
                    RichText::new(self.user_name.as_deref().unwrap_or("Aurafy"))
                        .size(16.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(if self.is_connected() {
                        i18n::tr(lang, i18n::Text::SpotifyConnected)
                    } else {
                        i18n::tr(lang, i18n::Text::SpotifyAccount)
                    })
                    .size(11.5)
                    .color(colours.muted),
                );
                ui.separator();
                for (label, page, tab) in [
                    (
                        i18n::tr(lang, i18n::Text::Profile),
                        Page::Profile,
                        SettingsTab::General,
                    ),
                    (
                        i18n::tr(lang, i18n::Text::Settings),
                        Page::Settings,
                        SettingsTab::General,
                    ),
                    (
                        i18n::tr(lang, i18n::Text::SleepTimer),
                        Page::Settings,
                        SettingsTab::Listening,
                    ),
                    (
                        i18n::tr(lang, i18n::Text::Account),
                        Page::Settings,
                        SettingsTab::Account,
                    ),
                ] {
                    if account_action(ui, label).clicked() {
                        self.settings_tab = tab;
                        self.navigate(page, ctx);
                        ui.close();
                    }
                }
                ui.separator();
                let mut background = !self.config.quit_on_close;
                if ui
                    .checkbox(
                        &mut background,
                        i18n::tr(lang, i18n::Text::KeepRunningOnClose),
                    )
                    .changed()
                {
                    self.config.quit_on_close = !background;
                    self.persist_preferences();
                }
                let mut resume = !self.config.pause_on_start;
                if ui
                    .checkbox(&mut resume, i18n::tr(lang, i18n::Text::ResumeOnOpen))
                    .changed()
                {
                    self.config.pause_on_start = !resume;
                    self.persist_preferences();
                }
                ui.separator();
                if self.is_connected()
                    && account_action(ui, i18n::tr(lang, i18n::Text::LogOut)).clicked()
                {
                    self.disconnect_spotify();
                    ui.close();
                }
                if account_action(ui, i18n::tr(lang, i18n::Text::QuitAurafy)).clicked() {
                    self.handle_desktop_action(desktop::Action::Quit, ctx);
                    ui.close();
                }
            });
    }

    pub(crate) fn draw_sidebar(&mut self, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let lang = self.config.language;
        SidePanel::left("sidebar")
            .exact_width(216.0)
            .resizable(false)
            .frame(Frame::new().fill(colours.panel).inner_margin(Margin {
                left: 14,
                right: 14,
                top: 18,
                bottom: 0,
            }))
            .show(ctx, |ui| {
                for (page, kind, label) in [
                    (Page::Home, Icon::Home, i18n::tr(lang, i18n::Text::Home)),
                    (
                        Page::Search,
                        Icon::Search,
                        i18n::tr(lang, i18n::Text::Search),
                    ),
                    (
                        Page::Library,
                        Icon::Library,
                        i18n::tr(lang, i18n::Text::YourLibrary),
                    ),
                ] {
                    self.nav_item(ui, ctx, page, kind, label);
                }
                ui.add_space(20.0);
                let available = ui.available_rect_before_wrap();
                let list_rect = available;
                let playlists = self.playlists.clone();
                if list_rect.height() >= 48.0 {
                    ui.scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
                        ui.set_clip_rect(list_rect);
                        ScrollArea::vertical()
                            .id_salt("sidebar-collection")
                            .max_height(list_rect.height())
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(i18n::tr(lang, i18n::Text::Collection))
                                            .size(10.0)
                                            .strong()
                                            .color(colours.muted),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if icon_button(
                                            ui,
                                            Icon::Plus,
                                            i18n::tr(lang, i18n::Text::CreatePlaylist),
                                            false,
                                        )
                                        .clicked()
                                        {
                                            if self.is_connected() {
                                                self.show_create_playlist = true;
                                            } else {
                                                self.start_auth(ctx);
                                            }
                                        }
                                    });
                                });
                                self.nav_item(
                                    ui,
                                    ctx,
                                    Page::Liked,
                                    Icon::Heart,
                                    i18n::tr(lang, i18n::Text::LikedSongs),
                                );
                                self.nav_item(
                                    ui,
                                    ctx,
                                    Page::Recent,
                                    Icon::Clock,
                                    i18n::tr(lang, i18n::Text::RecentlyPlayed),
                                );
                                ui.add_space(12.0);
                                for playlist in playlists {
                                    let selected = self.page == Page::Library
                                        && self
                                            .selected_playlist
                                            .as_ref()
                                            .is_some_and(|p| p.id == playlist.id);
                                    let (rect, response) = ui.allocate_exact_size(
                                        Vec2::new(
                                            ui.available_width(),
                                            if self.config.display.compact_sidebar {
                                                30.0
                                            } else {
                                                48.0
                                            },
                                        ),
                                        Sense::click(),
                                    );
                                    if response.hovered() || selected {
                                        ui.painter().rect_filled(
                                            rect,
                                            8.0,
                                            if selected {
                                                colours.selected
                                            } else {
                                                colours.hover
                                            },
                                        );
                                    }
                                    if !self.config.display.compact_sidebar {
                                        let art = Rect::from_min_size(
                                            rect.min + Vec2::new(7.0, 6.0),
                                            Vec2::splat(36.0),
                                        );
                                        self.art(
                                            ui,
                                            art,
                                            playlist.image_url.as_deref(),
                                            &playlist.name,
                                            ctx,
                                        );
                                    }
                                    let text_x = if self.config.display.compact_sidebar {
                                        9.0
                                    } else {
                                        53.0
                                    };
                                    text(
                                        ui,
                                        rect.min + Vec2::new(text_x, 8.0),
                                        &playlist.name,
                                        12.5,
                                        colours.text,
                                        rect.width() - text_x - 7.0,
                                    );
                                    if !self.config.display.compact_sidebar {
                                        let sub = if playlist.total_items > 0 {
                                            format!("Playlist · {} songs", playlist.total_items)
                                        } else {
                                            "Playlist".into()
                                        };
                                        text(
                                            ui,
                                            rect.min + Vec2::new(53.0, 27.0),
                                            &sub,
                                            10.5,
                                            colours.muted,
                                            rect.width() - text_x - 7.0,
                                        );
                                    }
                                    if response
                                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                                        .clicked()
                                    {
                                        self.open_playlist(playlist, ctx);
                                    }
                                }
                                if self.playlists.is_empty() {
                                    ui.label(
                                        RichText::new(if self.library_loading {
                                            "Loading playlists…"
                                        } else {
                                            "Your playlists live here."
                                        })
                                        .size(12.0)
                                        .color(colours.muted),
                                    );
                                }
                                ui.add_space(12.0);
                            });
                    });
                }
            });
    }

    fn nav_item(&mut self, ui: &mut Ui, ctx: &egui::Context, page: Page, kind: Icon, label: &str) {
        let colours = appearance::palette(ctx);
        let selected =
            self.page == page && !(page == Page::Library && self.selected_playlist.is_some());
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 43.0), Sense::click());
        if selected || response.hovered() {
            ui.painter().rect_filled(
                rect,
                8.0,
                if selected {
                    colours.selected
                } else {
                    colours.hover
                },
            );
        }
        let color = if selected || response.hovered() {
            colours.text
        } else {
            colours.muted
        };
        icon_at(
            ui,
            Rect::from_min_size(rect.min + Vec2::new(12.0, 12.0), Vec2::splat(20.0)),
            kind,
            color,
        );
        text(
            ui,
            rect.min + Vec2::new(44.0, 12.0),
            label,
            13.5,
            color,
            rect.width() - 50.0,
        );
        if response.clicked() {
            self.navigate(page, ctx);
        }
    }

    pub(crate) fn draw_header(&mut self, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let mut search = None;
        let mut focused = false;
        TopBottomPanel::top("app-header")
            .exact_height(64.0)
            .show_separator_line(false)
            .frame(Frame::new().fill(colours.bg).inner_margin(0))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                let drag = ui.interact(full, ui.id().with("window-drag"), Sense::click_and_drag());
                if drag.double_clicked() {
                    self.toggle_maximized(ctx);
                } else if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.painter().rect_filled(
                    Rect::from_min_size(full.min, Vec2::new(216.0, full.height())),
                    0,
                    colours.panel,
                );
                ui.painter().image(
                    self.logo_texture.id(),
                    Rect::from_center_size(
                        Pos2::new(full.left() + 34.0, full.center().y),
                        Vec2::splat(32.0),
                    ),
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                text(
                    ui,
                    Pos2::new(full.left() + 60.0, full.center().y - 13.0),
                    "aurafy",
                    20.0,
                    colours.text,
                    140.0,
                );
                let avatar_center = Pos2::new(
                    full.right()
                        - if self.config.display.native_titlebar {
                            34.0
                        } else {
                            154.0
                        },
                    full.center().y,
                );
                let avatar_ring = Rect::from_center_size(avatar_center, Vec2::splat(40.0));
                let avatar = Rect::from_center_size(avatar_center, Vec2::splat(32.0));
                let back_rect = Rect::from_center_size(
                    Pos2::new(full.left() + 148.0, full.center().y),
                    Vec2::splat(32.0),
                );
                let forward_rect = Rect::from_center_size(
                    Pos2::new(full.left() + 186.0, full.center().y),
                    Vec2::splat(32.0),
                );
                for (rect, id_key, enabled, dir, tip) in [
                    (
                        back_rect,
                        "nav-back",
                        self.can_go_back(),
                        -1.0_f32,
                        "Go back",
                    ),
                    (
                        forward_rect,
                        "nav-forward",
                        self.can_go_forward(),
                        1.0_f32,
                        "Go forward",
                    ),
                ] {
                    let resp = ui.interact(
                        rect,
                        ui.id().with(id_key),
                        if enabled {
                            Sense::click()
                        } else {
                            Sense::hover()
                        },
                    );
                    let resp = if enabled {
                        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text(tip)
                    } else {
                        resp.on_hover_text(tip)
                    };
                    let p = ui.painter();
                    let bg = if enabled && resp.hovered() {
                        colours.hover
                    } else {
                        colours.panel
                    };
                    p.circle_filled(rect.center(), 16.0, bg);
                    let stroke_col = if enabled { colours.text } else { colours.muted };
                    let cx = rect.center().x;
                    let cy = rect.center().y;
                    let s = Stroke::new(1.7_f32, stroke_col);
                    p.line_segment(
                        [
                            Pos2::new(cx - dir * 2.5, cy - 5.0),
                            Pos2::new(cx + dir * 3.0, cy),
                        ],
                        s,
                    );
                    p.line_segment(
                        [
                            Pos2::new(cx + dir * 3.0, cy),
                            Pos2::new(cx - dir * 2.5, cy + 5.0),
                        ],
                        s,
                    );
                    if enabled && resp.clicked() {
                        if dir < 0.0 {
                            self.go_back(ctx);
                        } else {
                            self.go_forward(ctx);
                        }
                    }
                }
                let sleep_label = self.sleep_timer.as_ref().map(|timer| match timer {
                    preferences::SleepTimer::Deadline(deadline) => {
                        let secs = deadline.saturating_duration_since(Instant::now()).as_secs();
                        let mins = ((secs + 59) / 60).max(1);
                        format!("Sleep {mins}m ×")
                    }
                    preferences::SleepTimer::EndOfTrack { .. } => "End of song ×".to_owned(),
                });
                if let Some(label) = &sleep_label {
                    let badge_w = 104.0;
                    let badge_rect = Rect::from_center_size(
                        Pos2::new(avatar_ring.left() - 12.0 - badge_w * 0.5, full.center().y),
                        Vec2::new(badge_w, 28.0),
                    );
                    let resp = ui
                        .interact(
                            badge_rect,
                            ui.id().with("header-sleep-pill"),
                            Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Cancel sleep timer");
                    ui.painter().rect_filled(
                        badge_rect,
                        14.0,
                        if resp.hovered() {
                            colours.hover
                        } else {
                            colours.panel
                        },
                    );
                    ui.painter().rect_stroke(
                        badge_rect,
                        14.0,
                        Stroke::new(1.0_f32, colours.green),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        badge_rect.center(),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(11.5),
                        colours.green,
                    );
                    if resp.clicked() {
                        self.sleep_timer = None;
                        self.set_status("Sleep timer cancelled");
                    }
                }
                let search_min_left = full.left() + 236.0;
                let search_max_right =
                    avatar_ring.left() - if sleep_label.is_some() { 128.0 } else { 20.0 };
                let search_w = (search_max_right - search_min_left).max(220.0).min(480.0);
                let search_left = (full.center().x - search_w * 0.5).clamp(
                    search_min_left,
                    (search_max_right - search_w).max(search_min_left),
                );
                let pill_rect = Rect::from_center_size(
                    Pos2::new(search_left + search_w * 0.5, full.center().y),
                    Vec2::new(search_w, 42.0),
                );
                let cy = pill_rect.center().y;
                let border_col = if ctx.memory(|m| m.has_focus(Id::new("global-search"))) {
                    colours.muted
                } else {
                    colours.border
                };
                ui.painter().rect_filled(pill_rect, 21.0, colours.panel2);
                ui.painter().rect_stroke(
                    pill_rect,
                    21.0,
                    Stroke::new(1.0_f32, border_col),
                    egui::StrokeKind::Inside,
                );
                let icon_rect = Rect::from_center_size(
                    Pos2::new(pill_rect.left() + 25.0, cy),
                    Vec2::splat(20.0),
                );
                icon_at(ui, icon_rect, Icon::Search, colours.muted);
                let right_reserve = if self.search_query.is_empty() {
                    58.0
                } else {
                    46.0
                };
                let edit_rect = Rect::from_min_max(
                    Pos2::new(icon_rect.right() + 8.0, pill_rect.top() + 2.0),
                    Pos2::new(
                        (pill_rect.right() - right_reserve).max(icon_rect.right() + 80.0),
                        pill_rect.bottom() - 2.0,
                    ),
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(edit_rect), |ui| {
                    let response = ui.add_sized(
                        edit_rect.size(),
                        TextEdit::singleline(&mut self.search_query)
                            .id(Id::new("global-search"))
                            .frame(false)
                            .margin(Margin::ZERO)
                            .vertical_align(Align::Center)
                            .font(FontId::proportional(14.0))
                            .hint_text(i18n::tr(
                                self.config.language,
                                i18n::Text::SearchPlaceholder,
                            )),
                    );
                    focused = response.has_focus();
                    if response.gained_focus() && self.page != Page::Search {
                        self.search_popup_open = true;
                    }
                    if response.changed() {
                        if self.search_query.trim().is_empty() {
                            self.clear_search();
                        } else {
                            self.search_edited_at = Some(Instant::now());
                            self.search_popup_open = false;
                        }
                    }
                    if response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        search = Some(self.search_query.clone());
                    }
                });
                if !self.search_query.is_empty() {
                    let clear_rect = Rect::from_center_size(
                        Pos2::new(pill_rect.right() - 26.0, cy),
                        Vec2::splat(28.0),
                    );
                    let clear_resp = ui
                        .interact(
                            clear_rect,
                            ui.id().with("clear-global-search"),
                            Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Clear search");
                    if clear_resp.hovered() {
                        ui.painter()
                            .circle_filled(clear_rect.center(), 13.0, colours.hover);
                    }
                    icon_at(
                        ui,
                        clear_rect.shrink(6.0),
                        Icon::Close,
                        if clear_resp.hovered() {
                            colours.text
                        } else {
                            colours.muted
                        },
                    );
                    if clear_resp.clicked() {
                        self.search_query.clear();
                        self.clear_search();
                        ctx.memory_mut(|m| m.request_focus(Id::new("global-search")));
                    }
                } else {
                    ui.painter().text(
                        Pos2::new(pill_rect.right() - 18.0, cy),
                        Align2::RIGHT_CENTER,
                        "Ctrl K",
                        FontId::proportional(11.0),
                        colours.muted,
                    );
                }
                self.search_rect = Some(pill_rect);
                let avatar_response =
                    ui.interact(avatar_ring, ui.id().with("account-button"), Sense::click());
                ui.painter()
                    .circle_filled(avatar_center, 20.0, colours.panel2);
                self.avatar(ui, avatar, ctx);
                self.account_menu(ctx, avatar_response.on_hover_text("Profile and settings"));
                let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                for (index, kind, label) in [
                    (0, Icon::Minimize, "Minimize"),
                    (
                        1,
                        if maximized {
                            Icon::Restore
                        } else {
                            Icon::Maximize
                        },
                        if maximized {
                            "Restore window"
                        } else {
                            "Maximize"
                        },
                    ),
                    (
                        2,
                        Icon::Close,
                        if self.config.quit_on_close {
                            "Quit Aurafy"
                        } else {
                            "Close to tray"
                        },
                    ),
                ]
                .into_iter()
                .filter(|_| !self.config.display.native_titlebar)
                {
                    let rect = Rect::from_center_size(
                        Pos2::new(full.right() - 106.0 + index as f32 * 40.0, full.center().y),
                        Vec2::new(40.0, 32.0),
                    );
                    if caption_button(ui, rect, kind, label).clicked() {
                        match index {
                            0 => ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                            1 => self.toggle_maximized(ctx),
                            _ => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        }
                    }
                }
            });
        let pointer_in_popup = self.search_popup_rect.is_some_and(|rect| {
            ctx.input(|i| i.pointer.hover_pos().is_some_and(|pos| rect.contains(pos)))
        });
        if self.search_popup_open
            && self.search_query.trim().is_empty()
            && (focused || pointer_in_popup)
            && !self.config.recent_search_tracks.is_empty()
        {
            if let Some(anchor) = self.search_rect {
                let popup = egui::Area::new(Id::new("search-suggestions"))
                    .order(Order::Foreground)
                    .fade_in(false)
                    .fixed_pos(anchor.left_bottom() + Vec2::new(0.0, 6.0))
                    .show(ctx, |ui| {
                        card(ui.ctx())
                            .stroke(Stroke::new(1.0_f32, colours.border))
                            .show(ui, |ui| {
                                ui.set_width((anchor.width() - 40.0).max(200.0));
                                self.draw_recent_search_tracks(ui, ctx);
                            });
                    });
                self.search_popup_rect = Some(popup.response.rect);
            }
        } else {
            self.search_popup_rect = None;
        }
        if let Some(edited) = self.search_edited_at {
            if edited.elapsed() >= Duration::from_millis(450) {
                self.search_edited_at = None;
                if !self.search_query.trim().is_empty()
                    && self.search_query.trim() != self.last_sent_query
                {
                    search = Some(self.search_query.clone());
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        if let Some(query) = search {
            self.search_edited_at = None;
            self.search_query = query.clone();
            self.run_search(query, ctx);
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.search_popup_open = false;
        }
        if ctx.input(|i| i.modifiers.ctrl && (i.key_pressed(Key::K) || i.key_pressed(Key::L))) {
            ctx.memory_mut(|m| m.request_focus(Id::new("global-search")));
            self.search_popup_open = true;
        }
    }

    fn toggle_maximized(&self, ctx: &egui::Context) {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }

    pub(crate) fn clear_search(&mut self) {
        self.search_results.clear();
        self.search_catalog = None;
        self.last_sent_query.clear();
        self.search_error = None;
        self.search_loading = false;
        self.search_edited_at = None;
        self.search_popup_open = true;
    }

    pub(crate) fn draw_resize_handles(&self, ctx: &egui::Context) {
        if self.config.display.native_titlebar {
            return;
        }
        if ctx.input(|i| {
            i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
        }) {
            return;
        }
        let screen = ctx.content_rect();
        let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) else {
            return;
        };
        if !screen.contains(pos) {
            return;
        }
        use egui::ResizeDirection::*;
        let left = pos.x < screen.left() + 6.0;
        let right = pos.x > screen.right() - 6.0;
        let top = pos.y < screen.top() + 6.0;
        let bottom = pos.y > screen.bottom() - 6.0;
        let (direction, cursor, rect) = match (left, right, top, bottom) {
            (true, _, true, _) => (
                NorthWest,
                egui::CursorIcon::ResizeNwSe,
                Rect::from_min_size(screen.min, Vec2::splat(12.0)),
            ),
            (_, true, true, _) => (
                NorthEast,
                egui::CursorIcon::ResizeNeSw,
                Rect::from_min_size(screen.right_top() - Vec2::new(12.0, 0.0), Vec2::splat(12.0)),
            ),
            (true, _, _, true) => (
                SouthWest,
                egui::CursorIcon::ResizeNeSw,
                Rect::from_min_size(
                    screen.left_bottom() - Vec2::new(0.0, 12.0),
                    Vec2::splat(12.0),
                ),
            ),
            (_, true, _, true) => (
                SouthEast,
                egui::CursorIcon::ResizeNwSe,
                Rect::from_min_size(screen.max - Vec2::splat(12.0), Vec2::splat(12.0)),
            ),
            (true, _, _, _) => (
                West,
                egui::CursorIcon::ResizeHorizontal,
                Rect::from_min_size(screen.min, Vec2::new(6.0, screen.height())),
            ),
            (_, true, _, _) => (
                East,
                egui::CursorIcon::ResizeHorizontal,
                Rect::from_min_size(
                    screen.right_top() - Vec2::new(6.0, 0.0),
                    Vec2::new(6.0, screen.height()),
                ),
            ),
            (_, _, true, _) => (
                North,
                egui::CursorIcon::ResizeVertical,
                Rect::from_min_size(screen.min, Vec2::new(screen.width(), 6.0)),
            ),
            (_, _, _, true) => (
                South,
                egui::CursorIcon::ResizeVertical,
                Rect::from_min_size(
                    screen.left_bottom() - Vec2::new(0.0, 6.0),
                    Vec2::new(screen.width(), 6.0),
                ),
            ),
            _ => return,
        };
        egui::Area::new(Id::new("window-resize"))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .movable(false)
            .show(ctx, |ui| {
                let (_, response) = ui.allocate_exact_size(rect.size(), Sense::click_and_drag());
                if response.is_pointer_button_down_on()
                    && ctx.input(|i| i.pointer.primary_pressed())
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                }
                response.on_hover_cursor(cursor);
            });
    }
}
