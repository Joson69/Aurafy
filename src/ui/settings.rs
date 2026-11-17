use super::*;

impl AurafyApp {
    pub(super) fn draw_settings_page(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        let lang = self.config.language;
        ui.set_max_width(820.0);
        heading(
            ui,
            i18n::tr(lang, i18n::Text::Settings),
            i18n::tr(lang, i18n::Text::SettingsSubtitle),
        );
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                (SettingsTab::General, i18n::tr(lang, i18n::Text::TabGeneral)),
                (
                    SettingsTab::Listening,
                    i18n::tr(lang, i18n::Text::TabListening),
                ),
                (
                    SettingsTab::Appearance,
                    i18n::tr(lang, i18n::Text::TabAppearance),
                ),
                (SettingsTab::Privacy, i18n::tr(lang, i18n::Text::TabPrivacy)),
                (SettingsTab::Account, i18n::tr(lang, i18n::Text::TabAccount)),
            ] {
                if ui.add(pill(label, self.settings_tab == tab, ctx)).clicked() {
                    self.settings_tab = tab;
                    self.persist_preferences();
                }
            }
        });
        ui.add_space(16.0);
        match self.settings_tab {
            SettingsTab::General => {
                let mut changed = false;
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(i18n::tr(lang, i18n::Text::StartupAndBackground))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(i18n::tr(lang, i18n::Text::LanguageLabel));
                        egui::ComboBox::from_id_salt("ui-language")
                            .selected_text(self.config.language.label())
                            .show_ui(ui, |ui| {
                                for value in i18n::Language::ALL {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.config.language,
                                            value,
                                            value.label(),
                                        )
                                        .changed();
                                }
                            });
                    });
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(i18n::tr(lang, i18n::Text::OpenAurafyWith));
                        let current = self.config.start_page;
                        egui::ComboBox::from_id_salt("start-page")
                            .selected_text(start_page_label(current))
                            .show_ui(ui, |ui| {
                                for page in [
                                    preferences::StartPage::LastView,
                                    preferences::StartPage::Home,
                                    preferences::StartPage::Search,
                                    preferences::StartPage::Library,
                                ] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.config.start_page,
                                            page,
                                            start_page_label(page),
                                        )
                                        .changed();
                                }
                            });
                    });
                    ui.add_space(8.0);
                    #[cfg(target_os = "windows")]
                    {
                        let mut enabled = self.startup_enabled;
                        if preference(ui, &mut enabled, "Start Aurafy with Windows", "Open automatically when you sign in to Windows.") {
                            match startup::set(enabled, self.config.startup_in_tray) {
                                Ok(()) => { self.startup_enabled = enabled; self.startup_error = None; }
                                Err(error) => self.startup_error = Some(error),
                            }
                        }
                        let mut in_tray = self.config.startup_in_tray;
                        ui.add_enabled_ui(self.startup_enabled, |ui| {
                            if preference(ui, &mut in_tray, "Start in the tray", "Windows startup stays in the background. Opening Aurafy yourself still shows the window.") {
                                match startup::set(true, in_tray) {
                                    Ok(()) => { self.config.startup_in_tray = in_tray; self.startup_error = None; changed = true; }
                                    Err(error) => self.startup_error = Some(error),
                                }
                            }
                        });
                        if let Some(error) = &self.startup_error { ui.label(RichText::new(error).color(Color32::LIGHT_RED)); }
                    }
                    let mut resume = !self.config.pause_on_start;
                    if preference(
                        ui,
                        &mut resume,
                        i18n::tr(lang, i18n::Text::ResumeOnOpen),
                        i18n::tr(lang, i18n::Text::ResumeOnOpenDesc),
                    ) {
                        self.config.pause_on_start = !resume;
                        changed = true;
                    }
                    let mut background = !self.config.quit_on_close;
                    if preference(
                        ui,
                        &mut background,
                        i18n::tr(lang, i18n::Text::KeepRunningOnClose),
                        i18n::tr(lang, i18n::Text::KeepRunningOnCloseDesc),
                    ) {
                        self.config.quit_on_close = !background;
                        changed = true;
                    }
                    let mut custom = !self.config.display.native_titlebar;
                    if preference(
                        ui,
                        &mut custom,
                        i18n::tr(lang, i18n::Text::CustomTitleBar),
                        i18n::tr(lang, i18n::Text::CustomTitleBarDesc),
                    ) {
                        self.config.display.native_titlebar = !custom;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(!custom));
                        changed = true;
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(i18n::tr(lang, i18n::Text::TrayIcon));
                        let previous = self.config.display.tray_style;
                        let mut style = previous;
                        egui::ComboBox::from_id_salt("tray-style").selected_text(style.label()).show_ui(ui, |ui| {
                            for value in [appearance::TrayStyle::Colour,appearance::TrayStyle::White,appearance::TrayStyle::Black] { ui.selectable_value(&mut style,value,value.label()); }
                        });
                        if style != previous {
                            let result = self.tray.as_ref().map(|tray|tray.set_style(style)).unwrap_or(Ok(()));
                            match result { Ok(()) => { self.config.display.tray_style=style;changed=true;self.tray_error=None; }, Err(error) => self.tray_error=Some(error) }
                        }
                    });
                    if let Some(error) = &self.tray_error {
                        ui.label(RichText::new(error).size(12.0).color(colours.muted));
                    }
                });
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(i18n::tr(lang, i18n::Text::KeyboardShortcuts))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Skip within a song");
                        let mut step = self.config.seek_step() as u8;
                        egui::ComboBox::from_id_salt("seek-step")
                            .selected_text(format!("{step} seconds"))
                            .show_ui(ui, |ui| {
                                for value in [5, 10, 15, 30] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut step,
                                            value,
                                            format!("{value} seconds"),
                                        )
                                        .changed();
                                }
                            });
                        self.config.seek_step_seconds = step;
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Volume key step");
                        let mut step = (self.config.volume_step() * 100.0).round() as u8;
                        egui::ComboBox::from_id_salt("volume-step")
                            .selected_text(format!("{step}%"))
                            .show_ui(ui, |ui| {
                                for value in [1, 2, 5, 10] {
                                    changed |= ui
                                        .selectable_value(&mut step, value, format!("{value}%"))
                                        .changed();
                                }
                            });
                        self.config.volume_step_percent = step;
                    });
                    ui.add_space(10.0);
                    for (key, action) in [
                        ("Ctrl K / Ctrl L", "Search"),
                        ("Space", "Play or pause"),
                        ("Ctrl Left / Ctrl Right", "Previous or next song"),
                        ("Alt Left / Alt Right", "Seek backward or forward"),
                        ("Ctrl Up / Ctrl Down", "Volume up or down"),
                        ("Ctrl Q", "Quit Aurafy"),
                    ] {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_sized(
                                [190.0, 24.0],
                                egui::Label::new(RichText::new(key).monospace().size(12.0)),
                            );
                            ui.label(RichText::new(action).color(colours.muted));
                        });
                    }
                });
                if changed {
                    self.persist_preferences();
                }
            }
            SettingsTab::Listening => {
                let mut changed = false;
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(i18n::tr(lang, i18n::Text::SoundAndStorage))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    #[cfg(target_os = "windows")]
                    if ui.button("Open Windows sound settings").clicked() {
                        if let Err(error) = browser::open_sound_settings() { self.set_status(error); }
                    }
                    ui.label(RichText::new("Choose your speakers or headphones in Windows.").size(12.0).color(colours.muted));
                    ui.add_space(12.0);
                    ui.label(RichText::new("Streaming quality").strong());
                    ui.horizontal_wrapped(|ui| {
                        egui::ComboBox::from_id_salt("stream-quality").selected_text(self.config.audio.quality.map(audio_prefs::Quality::label).unwrap_or("Spotify default")).show_ui(ui, |ui| {
                            for quality in [audio_prefs::Quality::Normal, audio_prefs::Quality::High, audio_prefs::Quality::VeryHigh, audio_prefs::Quality::Lossless] {
                                changed |= ui.selectable_value(&mut self.config.audio.quality, Some(quality), quality.label()).changed();
                            }
                        });
                    });
                    ui.label(RichText::new("Higher quality uses more data. Lossless requires Premium and availability on your account.").size(12.0).color(colours.muted));
                    ui.add_space(12.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Normalize volume");
                        egui::ComboBox::from_id_salt("audio-normalization").selected_text(match self.config.audio.normalize { Some(true) => "On", Some(false) => "Off", None => "Spotify default" }).show_ui(ui, |ui| {
                            changed |= ui.selectable_value(&mut self.config.audio.normalize, Some(true), "On").changed();
                            changed |= ui.selectable_value(&mut self.config.audio.normalize, Some(false), "Off").changed();
                        });
                    });
                    ui.add_space(10.0);
                    changed |= preference(
                        ui,
                        &mut self.config.ad_blocker,
                        "Ad blocker (auto-mute & skip ads)",
                        "Automatically mutes and skips detected ad breaks when controlling Spotify playback.",
                    );
                    ui.label(RichText::new("Keep loud and quiet songs at a similar volume.").size(12.0).color(colours.muted));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Quit and reopen Aurafy to apply audio changes.").size(11.5).color(colours.muted));
                    ui.add_space(12.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Playback cache on disk");
                        let mut limit = self.config.cache_mb();
                        egui::ComboBox::from_id_salt("cache-limit").selected_text(format!("{limit} MB")).show_ui(ui, |ui| {
                            for mb in [100,250,500,1000] { if ui.selectable_value(&mut limit,mb,format!("{mb} MB")).changed() { self.config.cache_limit_mb = mb; changed = true; } }
                        });
                    });
                    ui.label(RichText::new("Reuse recently streamed audio. The limit applies on the next launch; 100 MB keeps storage use small.").size(11.5).color(colours.muted));
                });
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Sleep timer").size(18.0).strong());
                    ui.label(RichText::new("Pause the active playback device after a while, or when this song ends.").size(12.0).color(colours.muted));
                    ui.add_space(12.0);
                    if let Some(timer) = &self.sleep_timer {
                        ui.label(RichText::new(timer.label()).color(colours.green).strong());
                        if ui.button("Cancel timer").clicked() { self.sleep_timer = None; }
                    } else {
                        ui.horizontal_wrapped(|ui| {
                            egui::ComboBox::from_id_salt("sleep-minutes").selected_text(format!("{} minutes",self.sleep_minutes)).show_ui(ui, |ui| {
                                for minutes in [15,30,45,60,90,120] { ui.selectable_value(&mut self.sleep_minutes, minutes, format!("{minutes} minutes")); }
                            });
                            if ui.add(primary("Start timer", ui.ctx())).clicked() { self.sleep_timer = Some(preferences::SleepTimer::minutes(self.sleep_minutes)); }
                            if ui.add_enabled(self.duration_ms > 1 && !self.current_track.id.is_empty(), egui::Button::new("End of this song")).clicked() {
                                self.sleep_timer = Some(preferences::SleepTimer::EndOfTrack { uri:if self.current_track.uri.is_empty() { self.current_track.id.clone() } else { self.current_track.uri.clone() },last_position:self.current_progress_ms(), duration:self.duration_ms });
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.label(RichText::new("Timers keep working in the tray and reset when you quit Aurafy.").size(11.5).color(colours.muted));
                });
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Volume at launch").size(18.0).strong());
                    ui.add_space(10.0);
                    let mut remember = self.config.startup_volume.is_none();
                    if preference(ui, &mut remember, "Remember this computer’s volume", "Phone volume stays separate. Changes below apply the next time Aurafy opens.") {
                        self.config.startup_volume = if remember { None } else { Some(self.config.last_local_volume.unwrap_or(40)) };
                        changed = true;
                    }
                    if let Some(volume) = &mut self.config.startup_volume {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Start at");
                            changed |= ui.add(egui::Slider::new(volume,0..=100).suffix("%")).changed();
                        });
                    }
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);
                    ui.label(RichText::new("Connect device name").strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.device_name_draft)
                                .hint_text(local_player::DEVICE_NAME)
                                .desired_width(220.0),
                        );
                        let candidate = AppConfig::sanitize_device_name(&self.device_name_draft);
                        let current = self.local_device_name();
                        if ui
                            .add_enabled(candidate != current, primary("Apply name", ui.ctx()))
                            .clicked()
                        {
                            self.device_name_draft = candidate.clone();
                            self.config.connect_device_name = if candidate == local_player::DEVICE_NAME {
                                String::new()
                            } else {
                                candidate.clone()
                            };
                            self.persist_preferences();
                            if self.local_player.is_some() {
                                self.start_local_playback(ctx, false);
                                self.set_status(format!("Restarting local player as {candidate}…"));
                            } else {
                                self.set_status(format!("Device name set to {candidate}"));
                            }
                        }
                        if current != local_player::DEVICE_NAME && ui.button("Reset").clicked() {
                            self.device_name_draft = local_player::DEVICE_NAME.into();
                            self.config.connect_device_name.clear();
                            self.persist_preferences();
                            if self.local_player.is_some() {
                                self.start_local_playback(ctx, false);
                                self.set_status("Restarting local player as Aurafy…");
                            } else {
                                self.set_status("Device name reset to Aurafy");
                            }
                        }
                    });
                    ui.label(
                        RichText::new("Name shown in Spotify’s Connect device picker for this computer.")
                            .size(12.0)
                            .color(colours.muted),
                    );
                });
                if changed {
                    self.persist_preferences();
                }
            }
            SettingsTab::Appearance => {
                let mut changed = false;
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(i18n::tr(lang, i18n::Text::Display))
                            .size(18.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(i18n::tr(lang, i18n::Text::Theme));
                        egui::ComboBox::from_id_salt("display-theme")
                            .selected_text(self.config.display.theme.label())
                            .show_ui(ui, |ui| {
                                for theme in appearance::Theme::ALL {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.config.display.theme,
                                            theme,
                                            theme.label(),
                                        )
                                        .changed();
                                }
                            });
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Accent colour");
                        let base_green = appearance::resolve(
                            appearance::Settings {
                                custom_accent: None,
                                ..self.config.display
                            },
                            None,
                            None,
                        )
                        .green;
                        let mut color = self
                            .config
                            .display
                            .custom_accent
                            .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                            .unwrap_or(base_green);
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            self.config.display.custom_accent =
                                Some([color.r(), color.g(), color.b()]);
                            changed = true;
                        }
                        if self.config.display.custom_accent.is_some()
                            && ui.button("Reset accent").clicked()
                        {
                            self.config.display.custom_accent = None;
                            changed = true;
                        }
                    });
                    changed |= preference(
                        ui,
                        &mut self.config.display.album_colours,
                        "Colour from album art",
                        "Use the current cover's colour on pages and the player.",
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.label(i18n::tr(lang, i18n::Text::InterfaceZoom));
                        egui::ComboBox::from_id_salt("interface-zoom")
                            .selected_text(format!("{}%", self.config.display.zoom_percent))
                            .show_ui(ui, |ui| {
                                for zoom in [80, 90, 100, 110, 125] {
                                    changed |= ui
                                        .selectable_value(
                                            &mut self.config.display.zoom_percent,
                                            zoom,
                                            format!("{zoom}%"),
                                        )
                                        .changed();
                                }
                            });
                        if ui.button(i18n::tr(lang, i18n::Text::Reset)).clicked() {
                            self.config.display.zoom_percent = 100;
                            changed = true;
                        }
                    });
                });
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Your library & player").size(18.0).strong());
                    ui.add_space(10.0);
                    changed |= preference(ui,&mut self.config.display.compact_sidebar,"Compact library sidebar","Show playlist names without covers.");
                    changed |= preference(ui,&mut self.config.compact_rows,"Compact song lists","Fit more songs into the same space.");
                    let mut artwork = !self.config.hide_artwork;
                    if preference(ui,&mut artwork,"Show album and playlist artwork","Turn off to reduce image downloads and memory use. Your profile picture stays visible.") { self.set_artwork_visible(artwork); }
                    let mut albums = !self.config.hide_album_column;
                    if preference(ui,&mut albums,"Show albums in song lists","The album column appears when there’s enough room.") { self.config.hide_album_column = !albums; changed = true; }
                    changed |= preference(ui,&mut self.show_queue,"Keep the queue open","See what’s coming up beside your music.");
                });
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Lyrics").size(18.0).strong());
                    ui.add_space(10.0);
                    let mut size = self.config.lyric_size(false) as u8;
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Text size");
                        if ui.add(egui::Slider::new(&mut size, 24..=44).suffix(" px")).changed() {
                            self.config.lyrics_font_size = size;
                            self.lyrics_last_line = None;
                            changed = true;
                        }
                    });
                    ui.label(RichText::new("Your music, your words.").font(player_font(f32::from(size), true)));
                    let mut follow = !self.config.lyrics_manual_by_default;
                    if preference(ui, &mut follow, "Follow lyrics automatically", "Start following the current line when opening lyrics or changing songs. You can still scroll freely.") {
                        self.config.lyrics_manual_by_default = !follow;
                        self.lyrics_follow = follow;
                        self.lyrics_last_line = None;
                        changed = true;
                    }
                });
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Motion & performance").size(18.0).strong());
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Playback refresh rate");
                        let mut fps = self.config.display.fps();
                        egui::ComboBox::from_id_salt("playback-fps")
                            .selected_text(format!("{fps} fps"))
                            .show_ui(ui, |ui| {
                                for value in [15, 30, 60] {
                                    if ui
                                        .selectable_value(&mut fps, value, format!("{value} fps"))
                                        .changed()
                                    {
                                        self.config.display.playback_fps = fps;
                                        changed = true;
                                    }
                                }
                            });
                    });
                    ui.label(
                        RichText::new(
                            "Applies to smoother playback progress. Lower rates use less CPU.",
                        )
                        .size(12.0)
                        .color(colours.muted),
                    );
                    if preference(
                        ui,
                        &mut self.config.reduce_motion,
                        "Reduce animations",
                        "Remove menu fades and keep progress updates light.",
                    ) {
                        ctx.style_mut(|style| {
                            style.animation_time =
                                if self.config.reduce_motion { 0.0 } else { 0.10 }
                        });
                        changed = true;
                    }
                    ui.add_enabled_ui(!self.config.reduce_motion, |ui| {
                        changed |= preference(
                            ui,
                            &mut self.config.smooth_progress,
                            "Smoother playback progress",
                            "Uses a little more CPU. Leave off for a lighter experience.",
                        );
                    });
                });
                if changed {
                    self.config.queue_open = self.show_queue;
                    self.persist_preferences();
                }
            }
            SettingsTab::Privacy => {
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Search history").size(18.0).strong());
                    ui.add_space(10.0);
                    let mut save = !self.config.private_search;
                    if preference(ui,&mut save,"Save recent searches on this computer","Turning this off clears local search history and stops saving new queries and selected songs.") {
                        self.set_search_history_enabled(save);
                    }
                    let count = self.config.recent_search_tracks.len();
                    ui.label(RichText::new(format!("{count} songs in recent searches")).size(12.0).color(colours.muted));
                    ui.add_space(8.0);
                    if ui.button("Clear search history").clicked() { self.clear_search_history(); self.set_status("Search history cleared on this computer"); }
                });
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new("Your listening data").size(18.0).strong());
                    ui.label(RichText::new("Search privacy applies to Aurafy. Spotify keeps its own account and listening history. Saved songs and playlists stay in your Spotify library.").size(12.0).color(colours.muted));
                });
            }
            SettingsTab::Account => {
                self.account_settings(ui, ctx);
                ui.add_space(16.0);
                card(ui.ctx()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(i18n::tr(lang, i18n::Text::Updates))
                            .size(18.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Aurafy {} · GitLab {}",
                            env!("CARGO_PKG_VERSION"),
                            updates::DEFAULT_PROJECT
                        ))
                        .size(12.0)
                        .color(colours.muted),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let check = ui.add_enabled(
                            !self.updates.checking,
                            egui::Button::new(i18n::tr(lang, i18n::Text::CheckForUpdates)),
                        );
                        if check.clicked() {
                            self.check_for_updates(ctx);
                        }
                        let release_url = match &self.updates.outcome {
                            Some(updates::CheckOutcome::UpdateAvailable(info))
                            | Some(updates::CheckOutcome::UpToDate(info)) => info.url.as_str(),
                            _ => updates::RELEASES_URL,
                        };
                        if ui.button("Open releases page").clicked() {
                            if let Err(error) = browser::open_url(release_url) {
                                self.updates.error = Some(error);
                            }
                        }
                        if self.updates.checking {
                            ui.spinner();
                            ui.label(
                                RichText::new("Checking GitLab Releases…")
                                    .size(12.0)
                                    .color(colours.muted),
                            );
                        }
                    });
                    if let Some(error) = &self.updates.error {
                        ui.add_space(6.0);
                        ui.label(RichText::new(error).size(12.0).color(Color32::LIGHT_RED));
                    } else if let Some(outcome) = &self.updates.outcome {
                        ui.add_space(6.0);
                        match outcome {
                            updates::CheckOutcome::UpdateAvailable(info) => {
                                ui.label(
                                    RichText::new(format!(
                                        "Update available: {} ({})",
                                        info.name, info.tag
                                    ))
                                    .size(12.5)
                                    .color(colours.green),
                                );
                                if !info.notes.is_empty() {
                                    ui.label(
                                        RichText::new(&info.notes).size(11.5).color(colours.muted),
                                    );
                                }
                            }
                            updates::CheckOutcome::UpToDate(info) => {
                                ui.label(
                                    RichText::new(format!(
                                        "You're up to date (latest release: {}).",
                                        info.tag
                                    ))
                                    .size(12.0)
                                    .color(colours.muted),
                                );
                            }
                            updates::CheckOutcome::NoReleases => {
                                ui.label(
                                    RichText::new("No published releases found on GitLab yet.")
                                        .size(12.0)
                                        .color(colours.muted),
                                );
                            }
                            updates::CheckOutcome::PrivateOrUnavailable(message) => {
                                ui.label(RichText::new(message).size(12.0).color(colours.muted));
                            }
                        }
                    }
                });
                ui.add_space(16.0);
                if let Some(error) = self.playback_error.clone() {
                    self.error_notice(ui, &error);
                    ui.add_space(12.0);
                }
                egui::CollapsingHeader::new("Local player connection")
                    .default_open(!self.local_status.paired)
                    .show(ui, |ui| self.draw_local_playback_settings(ui, ctx));
                egui::CollapsingHeader::new("Font credits").show(ui, |ui| {
                    ui.label("Lato by the Lato Project Authors · SIL Open Font License 1.1");
                    ScrollArea::vertical()
                        .id_salt("font-license")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(include_str!("../../assets/fonts/OFL.txt"))
                                    .size(11.0),
                            );
                        });
                });
            }
        }
        ui.add_space(20.0);
        ui.label(
            RichText::new(concat!(
                "Aurafy ",
                env!("CARGO_PKG_VERSION"),
                " · Made for the music."
            ))
            .size(11.0)
            .color(colours.muted),
        );
    }

    fn draw_local_playback_settings(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        card(ui.ctx()).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Listen on this computer").size(19.0).strong());
            ui.label(RichText::new("Make Aurafy a Spotify Connect device, so your music plays here.").size(12.0).color(colours.muted));
            ui.add_space(12.0);
            if self.local_starting {
                ui.horizontal(|ui| { ui.spinner(); ui.label("Starting local playback…"); });
            } else if self.local_status.running {
                ui.label(RichText::new(&self.local_status.message).color(if self.local_status.active { colours.green } else { colours.text }));
                if !self.local_status.paired {
                    ui.label(RichText::new("Pair once: keep your phone on the same Wi-Fi, open Spotify’s device picker, and choose Aurafy. After pairing, you can switch between devices normally.").size(12.0).color(colours.muted));
                }
                ui.add_space(8.0);
                egui::CollapsingHeader::new("Connection troubleshooting").show(ui, |ui| {
                    if ui.button("Reconnect player").clicked() { self.reconnect_local_player(ctx); }
                    if ui.button("Disable local player").clicked() { self.stop_local_playback(); }
                    ui.label(RichText::new("Resetting pairing disconnects this player and requires choosing Aurafy from your phone again.").size(12.0).color(colours.muted));
                    if ui.button("Reset saved pairing").clicked() { self.restart_local_pairing(ctx); }
                });
            } else {
                ui.label(RichText::new("Enter your personal Spotify Soloist API key. It is saved in Windows Credential Manager, separately from your Spotify sign-in.").size(12.0).color(colours.muted));
                ui.add_space(6.0);
                ui.add(TextEdit::singleline(&mut self.local_key_input).password(true).hint_text("Spotify Soloist API key").desired_width(460.0));
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.add(primary("Save key & enable playback", ui.ctx())).clicked() { self.enable_local_playback(ctx); }
                    if ui.button("Get a Soloist API key").clicked() { if let Err(error) = browser::open_url(local_player::KEY_PAGE) { self.playback_error = Some(error); } }
                });
                ui.label(RichText::new("Spotify’s official Windows playback engine is downloaded if needed. No Spotify desktop window needs to stay open.").size(11.0).color(colours.muted));
            }
        });
    }

    fn account_settings(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        if self.is_connected() {
            card(ui.ctx()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                    self.avatar(ui, r, ctx);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(
                                self.user_name.as_deref().unwrap_or("Your Spotify account"),
                            )
                            .size(19.0)
                            .strong(),
                        );
                        ui.label(
                            RichText::new("Connected to Spotify")
                                .size(12.0)
                                .color(colours.green),
                        );
                    });
                });
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    if ui.button("View profile").clicked() {
                        self.navigate(Page::Profile, ctx);
                    }
                    if ui.button("Disconnect account").clicked() {
                        self.disconnect_spotify();
                    }
                });
            });
            ui.add_space(16.0);
        }
        if let Some(error) = self.auth_error.clone() {
            self.error_notice(ui, &error);
            ui.add_space(12.0);
        }
        if self.auth_in_progress {
            card(ui.ctx()).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Finish connecting in your browser")
                            .size(17.0)
                            .strong(),
                    );
                });
                ui.label(
                    RichText::new("Keep Aurafy open until Spotify sends you back.")
                        .color(colours.muted),
                );
                if self.auth_uses_fallback_port {
                    ui.label(
                        RichText::new("Using fallback port 43821.")
                            .size(11.0)
                            .color(colours.muted),
                    );
                }
                if let Some(url) = self.auth_url.clone() {
                    ui.horizontal(|ui| {
                        if ui.button("Open sign-in page").clicked() {
                            self.auth_error = browser::open_url(&url).err();
                        }
                        if ui.button("Copy sign-in link").clicked() {
                            ui.ctx().copy_text(url);
                        }
                    });
                }
                if let Some(cancel) = &self.auth_cancel {
                    if ui
                        .add_enabled(
                            !cancel.load(Ordering::Relaxed),
                            egui::Button::new("Cancel sign-in"),
                        )
                        .clicked()
                    {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
            });
            ui.add_space(16.0);
        }
        egui::CollapsingHeader::new(if self.is_connected(){"Developer connection settings"}else{"Connect your Spotify account"}).default_open(!self.is_connected()).show(ui,|ui|{
            card(ui.ctx()).show(ui,|ui|{
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Spotify app setup").size(18.0).strong());
                ui.label(RichText::new("Use the Client ID from your Spotify developer app. Your sign-in is saved securely on this computer.").size(12.0).color(colours.muted));ui.add_space(10.0);
                ui.label(RichText::new("Client ID").size(12.0).strong());let enabled=!self.auth_in_progress&&!self.is_connected();ui.add_enabled(enabled,TextEdit::singleline(&mut self.config.client_id).hint_text("Spotify Client ID").desired_width(f32::INFINITY));ui.add_space(10.0);
                ui.label(RichText::new("Register both callback addresses in your developer app:").size(12.0).color(colours.muted));
                for (label,uri) in [("Standard",spotify::REDIRECT_URI),("Fallback",spotify::FALLBACK_REDIRECT_URI)]{ui.horizontal_wrapped(|ui|{ui.label(RichText::new(uri).monospace().size(12.0));if ui.small_button(format!("Copy {label}")).clicked(){ui.ctx().copy_text(uri.into());}});}
                ui.add_space(12.0);ui.horizontal(|ui|{if ui.add_enabled(enabled,primary("Save and connect Spotify", ui.ctx())).clicked(){self.start_auth(ctx);}if ui.button("Developer dashboard").clicked(){if let Err(e)=browser::open_url("https://developer.spotify.com/dashboard"){self.auth_error=Some(e);}}});
            });
        });
    }
}
