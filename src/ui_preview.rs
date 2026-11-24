//! Render the real egui views offscreen for layout review, without opening an
//! account, reading credentials, launching a window, or making network requests.
use super::*;
use crate::appearance::{BG, GREEN};
use egui::epaint::{ClippedPrimitive, ImageData, Primitive};
use std::collections::HashMap;

#[cfg(target_os = "windows")]
#[test]
fn korean_glyphs_available_in_ui_and_lyrics() {
    let ctx = egui::Context::default();
    ui::configure(&ctx);
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        ctx.fonts_mut(|fonts| {
            for family in [
                egui::FontFamily::Proportional,
                egui::FontFamily::Name("aurafy-regular".into()),
                egui::FontFamily::Name("aurafy-bold".into()),
            ] {
                let font = egui::FontId::new(32.0, family);
                for ch in "한국어가나다라마바사아자차카타파하힣".chars() {
                    assert!(fonts.has_glyph(&font, ch), "Missing Korean glyph {ch}");
                }
            }
        });
    });
}

#[cfg(target_os = "windows")]
#[test]
fn japanese_glyphs_available_in_ui_and_lyrics() {
    let ctx = egui::Context::default();
    ui::configure(&ctx);
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        ctx.fonts_mut(|fonts| {
            for family in [
                egui::FontFamily::Proportional,
                egui::FontFamily::Name("aurafy-regular".into()),
                egui::FontFamily::Name("aurafy-bold".into()),
            ] {
                let font = egui::FontId::new(32.0, family);
                for ch in "日本語あいうえおカキクケコ設定再生".chars() {
                    assert!(fonts.has_glyph(&font, ch), "Missing Japanese glyph {ch}");
                }
            }
        });
    });
}

#[test]
fn new_preferences_migrate_without_changing_existing_startup_behavior() {
    let config: AppConfig =
        serde_json::from_str(r#"{"pause_on_start":true,"local_playback":true}"#).unwrap();
    assert!(config.pause_on_start && config.local_playback);
    assert!(!config.startup_in_tray);
    assert_eq!(config.lyric_size(false), 32.0);
    assert_eq!(config.lyric_size(true), 28.0);
    assert_eq!(config.seek_step(), 10);
    let ctx = egui::Context::default();
    let mut config = config;
    config.lyrics_font_size = 40;
    config.lyrics_manual_by_default = true;
    config.seek_step_seconds = 30;
    let mut app = AurafyApp::build(&ctx, config);
    app.toggle_lyrics();
    assert!(!app.lyrics_follow);
    assert_eq!(app.config.lyric_size(false), 40.0);
    let saved = serde_json::to_string(&app.config).unwrap();
    let restored: AppConfig = serde_json::from_str(&saved).unwrap();
    assert_eq!(restored.seek_step(), 30);
    assert!(restored.lyrics_manual_by_default);
}

#[test]
fn seek_and_volume_steps_use_native_controls_and_clamp_to_bounds() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.config.seek_step_seconds = 15;
    app.is_playing = false;
    app.duration_ms = 180_000;
    app.progress_ms = 5_000;
    app.seek_relative(false, &ctx);
    assert_eq!(app.progress_ms, 0);
    assert_eq!(rx.try_recv().unwrap().requests[0]["position_ms"], 0);
    app.seek_relative(true, &ctx);
    assert_eq!(app.progress_ms, 15_000);
    assert_eq!(rx.try_recv().unwrap().requests[0]["position_ms"], 15_000);
    app.config.volume_step_percent = 10;
    app.volume = 0.95;
    app.step_volume(true, &ctx);
    assert_eq!(app.volume, 1.0);
    assert_eq!(rx.try_recv().unwrap().requests[0]["volume"], 100);
}

#[test]
fn tray_playback_actions_keep_the_selected_local_player_and_hidden_window() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.config.pause_on_start = true;
    app.window_state.hidden = true;
    app.handle_desktop_action(desktop::Action::TogglePlayback, &ctx);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
    // A native state update changes the next tray action from Play to Pause.
    app.local_status.active = true;
    app.is_playing = true;
    app.handle_desktop_action(desktop::Action::TogglePlayback, &ctx);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("pause")]
    );
    app.handle_desktop_action(desktop::Action::Next, &ctx);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("skip_next")]
    );
    app.handle_desktop_action(desktop::Action::Previous, &ctx);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("skip_prev")]
    );
    assert!(app.window_state.hidden);
    app.handle_desktop_action(desktop::Action::Open, &ctx);
    assert!(!app.window_state.hidden);
    app.handle_desktop_action(desktop::Action::Quit, &ctx);
    assert!(app.window_state.quitting);
}

fn fixture(ctx: &egui::Context) -> AurafyApp {
    let mut app = AurafyApp::build(ctx, AppConfig::default());
    app.token = Some(TokenState {
        access_token: "preview-only".into(),
        refresh_token: None,
        expires_at: Instant::now() + Duration::from_secs(3600),
    });
    app.user_name = Some("grid".into());
    app.user_profile = Some(spotify::UserProfile {
        id: "preview-owner".into(),
        display_name: "grid".into(),
        image_url: None,
        profile_url: None,
    });
    app.status_until = Instant::now();
    app.recent_tracks = app.demo_tracks.clone();
    app.saved_tracks = app.demo_tracks.clone();
    app.saved_track_ids = app.saved_tracks.iter().map(|t| t.id.clone()).collect();
    app.device_name = Some("GRID".into());
    app.devices = vec![Device {
        id: "preview-device".into(),
        name: "GRID".into(),
        device_type: "Computer".into(),
        is_active: true,
        volume_percent: Some(72),
    }];
    app.current_track = app.demo_tracks[0].clone();
    app.is_playing = false;
    for (i, name) in [
        "faves",
        "God is a woman",
        "Peak Kanye",
        "fwoosh",
        "vibez dupe",
        "BEST of METRO BOOMIN",
        "Psychedelic Indie",
        "2000s Pop Hit Throwback",
    ]
    .iter()
    .enumerate()
    {
        app.playlists.push(Playlist {
            id: format!("preview-{i}"),
            name: (*name).into(),
            owner: "grid".into(),
            owner_id: "preview-owner".into(),
            total_items: 32 + i * 9,
            items_accessible: true,
            ..Default::default()
        });
    }
    // Deterministic artwork fixtures exercise texture clipping and image layout.
    for i in 0..app.recent_tracks.len() {
        let url = format!("preview://art/{i}");
        let texture = ctx.load_texture(&url, fixture_art(i), egui::TextureOptions::LINEAR);
        app.artwork_cache.insert(url.clone(), texture);
        app.recent_tracks[i].artwork_url = Some(url.clone());
        app.saved_tracks[i].artwork_url = Some(url.clone());
        if i < app.playlists.len() {
            app.playlists[i].image_url = Some(url.clone());
        }
        if i < app.queue.len() {
            app.queue[i].artwork_url = Some(url.clone());
        }
        if i == 0 {
            app.current_track.artwork_url = Some(url);
        }
    }
    app.insights.attempted = true;
    app.insights.cache.insert(
        app.insights.range,
        spotify::insights::Data {
            tracks: app.recent_tracks.iter().take(6).cloned().collect(),
            artists: app
                .recent_tracks
                .iter()
                .take(4)
                .enumerate()
                .map(|(i, t)| spotify::catalog::Artist {
                    id: format!("preview-artist-{i}"),
                    name: t.artist.clone(),
                    image_url: t.artwork_url.clone(),
                })
                .collect(),
        },
    );
    app
}

fn fixture_art(index: usize) -> egui::ColorImage {
    let palettes = [
        ([173, 87, 65], [36, 24, 38]),
        ([45, 108, 122], [13, 28, 47]),
        ([169, 136, 72], [39, 34, 30]),
        ([113, 102, 169], [29, 33, 57]),
        ([63, 127, 95], [15, 32, 34]),
        ([184, 108, 133], [52, 27, 40]),
    ];
    let (a, b) = palettes[index % palettes.len()];
    let mut pixels = Vec::with_capacity(128 * 128);
    for y in 0..128 {
        for x in 0..128 {
            let dx = x as f32 - 70.0;
            let dy = y as f32 - 51.0;
            let inside = (dx * dx + dy * dy).sqrt() < 39.0;
            let t = if inside { 0.75 } else { (x + y) as f32 / 280.0 };
            let channel = |c: usize| (a[c] as f32 * (1.0 - t) + b[c] as f32 * t) as u8;
            pixels.push(Color32::from_rgb(channel(0), channel(1), channel(2)));
        }
    }
    egui::ColorImage::new([128, 128], pixels)
}

fn draw(app: &mut AurafyApp, ctx: &egui::Context) {
    app.artwork_cache.begin_frame();
    app.draw_player(ctx);
    app.draw_header(ctx);
    app.draw_sidebar(ctx);
    app.draw_queue(ctx);
    app.draw_main(ctx);
    app.draw_playlist_dialogs(ctx);
    app.draw_resize_handles(ctx);
    app.artwork_cache.finish_frame();
}

fn render(
    page: Page,
    tab: SettingsTab,
    width: u32,
    height: u32,
    queue: bool,
    connected: bool,
) -> image::RgbaImage {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = page;
    app.settings_tab = tab;
    app.show_queue = queue;
    if page == Page::Library && queue {
        app.selected_playlist = app.playlists.first().cloned();
        app.playlist_tracks = app.saved_tracks.clone();
    }
    if !connected {
        app.token = None;
        app.user_name = None;
        app.user_profile = None;
        app.config.client_id.clear();
    }
    if page == Page::Search {
        app.search_query = "Kanye West".into();
        app.last_sent_query = "Kanye West".into();
        app.search_results = app.saved_tracks.clone();
        app.search_results[0].name="A very long song title (feat. several artists) that must never overlap the album or actions".into();
    }
    render_app(&mut app, &ctx, width, height)
}

fn render_app(
    app: &mut AurafyApp,
    ctx: &egui::Context,
    width: u32,
    height: u32,
) -> image::RgbaImage {
    render_app_inner(app, ctx, width, height, false)
}

fn render_app_inner(
    app: &mut AurafyApp,
    ctx: &egui::Context,
    width: u32,
    height: u32,
    open_menu: bool,
) -> image::RgbaImage {
    let mut textures = HashMap::new();
    let mut shapes = Vec::new();
    for pass in 0..if open_menu || app.device_picker_open {
        16
    } else {
        6
    } {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(width as f32, height as f32),
            )),
            events: if open_menu && matches!(pass, 2 | 3) {
                let pos = Pos2::new(width as f32 - 154.0, 32.0);
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: pass == 2,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]
            } else {
                vec![]
            },
            ..Default::default()
        };
        let output = ctx.run(input, |ctx| draw(app, ctx));
        for (id, delta) in output.textures_delta.set {
            let ImageData::Color(image) = delta.image;
            if let Some([x, y]) = delta.pos {
                let target: &mut egui::ColorImage = textures.get_mut(&id).unwrap();
                for row in 0..image.size[1] {
                    let start = (y + row) * target.size[0] + x;
                    target.pixels[start..start + image.size[0]].copy_from_slice(
                        &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                    );
                }
            } else {
                textures.insert(id, (*image).clone());
            }
        }
        shapes = output.shapes;
    }
    let primitives = ctx.tessellate(shapes, 1.0);
    rasterize(width, height, &primitives, &textures)
}

fn rasterize(
    width: u32,
    height: u32,
    primitives: &[ClippedPrimitive],
    textures: &HashMap<egui::TextureId, egui::ColorImage>,
) -> image::RgbaImage {
    let mut out =
        image::RgbaImage::from_pixel(width, height, image::Rgba([BG.r(), BG.g(), BG.b(), 255]));
    let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
    for primitive in primitives {
        let Primitive::Mesh(mesh) = &primitive.primitive else {
            continue;
        };
        let texture = &textures[&mesh.texture_id];
        for tri in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [
                mesh.vertices[tri[0] as usize],
                mesh.vertices[tri[1] as usize],
                mesh.vertices[tri[2] as usize],
            ];
            let area = cross(b.pos - a.pos, c.pos - a.pos);
            if area.abs() < 0.0001 {
                continue;
            }
            let min_x = a
                .pos
                .x
                .min(b.pos.x)
                .min(c.pos.x)
                .max(primitive.clip_rect.left())
                .max(0.0)
                .floor() as u32;
            let max_x = a
                .pos
                .x
                .max(b.pos.x)
                .max(c.pos.x)
                .min(primitive.clip_rect.right())
                .min(width as f32)
                .ceil() as u32;
            let min_y = a
                .pos
                .y
                .min(b.pos.y)
                .min(c.pos.y)
                .max(primitive.clip_rect.top())
                .max(0.0)
                .floor() as u32;
            let max_y = a
                .pos
                .y
                .max(b.pos.y)
                .max(c.pos.y)
                .min(primitive.clip_rect.bottom())
                .min(height as f32)
                .ceil() as u32;
            for y in min_y..max_y {
                for x in min_x..max_x {
                    let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                    if !primitive.clip_rect.contains(p) {
                        continue;
                    }
                    let w0 = cross(b.pos - p, c.pos - p) / area;
                    let w1 = cross(c.pos - p, a.pos - p) / area;
                    let w2 = 1.0 - w0 - w1;
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let uv = a.uv.to_vec2() * w0 + b.uv.to_vec2() * w1 + c.uv.to_vec2() * w2;
                    let tx = (uv.x * texture.size[0] as f32 - 0.5)
                        .clamp(0.0, (texture.size[0] - 1) as f32);
                    let ty = (uv.y * texture.size[1] as f32 - 0.5)
                        .clamp(0.0, (texture.size[1] - 1) as f32);
                    let x0 = tx.floor() as usize;
                    let y0 = ty.floor() as usize;
                    let x1 = (x0 + 1).min(texture.size[0] - 1);
                    let y1 = (y0 + 1).min(texture.size[1] - 1);
                    let fx = tx - x0 as f32;
                    let fy = ty - y0 as f32;
                    let samples = [
                        texture.pixels[y0 * texture.size[0] + x0],
                        texture.pixels[y0 * texture.size[0] + x1],
                        texture.pixels[y1 * texture.size[0] + x0],
                        texture.pixels[y1 * texture.size[0] + x1],
                    ];
                    let mut src = [0.0; 4];
                    for ch in 0..4 {
                        let tex = (samples[0][ch] as f32 * (1.0 - fx) + samples[1][ch] as f32 * fx)
                            * (1.0 - fy)
                            + (samples[2][ch] as f32 * (1.0 - fx) + samples[3][ch] as f32 * fx)
                                * fy;
                        let vertex = a.color[ch] as f32 * w0
                            + b.color[ch] as f32 * w1
                            + c.color[ch] as f32 * w2;
                        src[ch] = tex * vertex / 255.0;
                    }
                    let dst = out.get_pixel_mut(x, y);
                    let remaining = 1.0 - src[3] / 255.0;
                    for ch in 0..3 {
                        dst[ch] = (src[ch] + dst[ch] as f32 * remaining).clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    out
}

#[test]
fn old_config_loads_without_losing_client_id_or_searches() {
    let cfg: AppConfig =
        serde_json::from_str(r#"{"client_id":"existing-client","recent_searches":["Daft Punk"]}"#)
            .unwrap();
    assert_eq!(cfg.client_id, "existing-client");
    assert_eq!(cfg.recent_searches, vec!["Daft Punk"]);
    assert!(!cfg.queue_open);
}

fn real_track(id: &str) -> Track {
    Track {
        id: id.into(),
        uri: format!("spotify:track:{id}"),
        name: format!("Song {id}"),
        artist: "Test artist".into(),
        duration_ms: 180000,
        ..Default::default()
    }
}

fn local_fixture(ctx: &egui::Context) -> (AurafyApp, Receiver<local_player::Job>) {
    let mut app = AurafyApp::build(
        ctx,
        AppConfig {
            client_id: "test-client".into(),
            local_playback: true,
            ..Default::default()
        },
    );
    let (player, rx) = local_player::Player::test_channel();
    app.local_player = Some(player);
    app.local_status = local_player::Status {
        running: true,
        paired: true,
        active: false,
        ..Default::default()
    };
    (app, rx)
}

#[test]
fn configured_startup_never_uses_demo_music() {
    let ctx = egui::Context::default();
    let app = AurafyApp::build(
        &ctx,
        AppConfig {
            client_id: "test-client".into(),
            ..Default::default()
        },
    );
    assert!(app.current_track.uri.is_empty());
    assert_eq!(app.current_track.name, "Restoring playback…");
    assert!(!app.is_demo_mode());
    assert!(app.queue.is_empty());
    assert!(!app.is_playing);
}

#[test]
fn play_here_uses_the_restored_native_player_without_web_device_discovery() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.token = Some(TokenState {
        access_token: "test-only".into(),
        refresh_token: None,
        expires_at: Instant::now() + Duration::from_secs(300),
    });
    app.current_track = real_track("phone");
    app.device_name = Some("Phone".into());
    app.duration_ms = 180000;
    app.progress_ms = 42000;
    app.progress = 42000.0 / 180000.0;
    app.is_playing = true;
    app.follow_remote_device = true;
    assert!(app.devices.is_empty());
    app.listen_here(&ctx);
    let job = rx.try_recv().unwrap();
    assert_eq!(
        job.requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
    assert!(app.pending_handoff.is_some());
    assert!(!app.follow_remote_device);
    assert!(
        !app.local_status.active,
        "wait for actual activation confirmation"
    );
}

#[test]
fn fresh_resume_uses_auralis_but_an_explicit_phone_handoff_is_respected() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.device_name = Some("Phone".into());
    app.send_playback_action("resume", &ctx);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
    app.follow_remote_device = true;
    assert!(!app.use_local_for_play());
}

#[test]
fn device_list_keeps_the_ready_local_receiver_and_background_refresh_is_quiet() {
    let ctx = egui::Context::default();
    let (mut app, _) = local_fixture(&ctx);
    app.page = Page::Settings;
    app.token = Some(TokenState {
        access_token: "test-only".into(),
        refresh_token: None,
        expires_at: Instant::now() + Duration::from_secs(300),
    });
    app.last_devices_poll = Instant::now() - Duration::from_secs(31);
    assert!(
        !app.should_refresh_devices(),
        "settings must not poll devices"
    );
    app.device_picker_open = true;
    assert!(app.should_refresh_devices());
    app.local_status.active = true;
    assert!(!app.should_refresh_devices());
    app.status_message = "Listening".into();
    app.tx
        .send(AppEvent::DevicesLoaded {
            devices: Ok(vec![Device {
                name: "Phone".into(),
                id: "phone".into(),
                is_active: true,
                ..Default::default()
            }]),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    let devices = app.playback_devices();
    assert_eq!(devices[0].name, "Aurafy");
    assert!(devices[0].is_active && !devices[1].is_active);
    assert_eq!(app.status_message, "Listening");
    app.tx
        .send(AppEvent::DevicesLoaded {
            devices: Err("Offline".into()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert_eq!(app.status_message, "Listening");
    assert_eq!(app.devices_error.as_deref(), Some("Offline"));
    assert_eq!(app.playback_devices()[0].name, "Aurafy");
}

#[test]
fn events_from_a_replaced_player_cannot_show_stale_pairing_prompts() {
    let ctx = egui::Context::default();
    let (mut app, _) = local_fixture(&ctx);
    app.local_player_generation = 2;
    app.tx
        .send(AppEvent::LocalPlayer {
            generation: 1,
            event: local_player::Event::Status(local_player::Status {
                running: true,
                message: "Finish pairing: obsolete worker".into(),
                ..Default::default()
            }),
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.local_status.paired);
}

#[test]
fn startup_selects_local_once_and_does_not_reclaim_a_phone_handoff() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    let ready = local_player::Status {
        running: true,
        paired: true,
        ..Default::default()
    };
    app.handle_local_event(local_player::Event::Status(ready.clone()));
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
    assert!(!app.select_local_on_ready);
    app.handle_local_event(local_player::Event::Status(local_player::Status {
        active: true,
        ..ready.clone()
    }));
    app.handle_local_event(local_player::Event::Status(ready));
    app.tx
        .send(AppEvent::PlayerSynced {
            playback: Ok(Some(PlaybackState {
                track: Some(real_track("phone")),
                is_playing: true,
                device_name: Some("Phone".into()),
                device_id: Some("phone".into()),
                ..Default::default()
            })),
            queue: None,
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.follow_remote_device);
    assert!(
        rx.try_recv().is_err(),
        "a later phone handoff must not reactivate the laptop"
    );
}

#[test]
fn closed_remote_device_falls_back_to_aurafy_on_play() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.select_local_on_ready = false;
    for event in [
        AppEvent::PlayerSynced {
            playback: Ok(None),
            queue: None,
            token: None,
        },
        AppEvent::ActionFinished {
            action: "resume".into(),
            result: Err(spotify::PLAYBACK_NOT_FOUND.into()),
            token: None,
        },
    ] {
        app.follow_remote_device = true;
        app.device_name = Some("Spotify Desktop".into());
        app.device_id = Some("desktop".into());
        let missed_play = matches!(&event, AppEvent::ActionFinished { .. });
        app.tx.send(event).unwrap();
        app.handle_events(&ctx);
        assert!(!app.follow_remote_device);
        assert!(app.selected_output_name().is_some());
        if !missed_play {
            assert!(
                rx.try_recv().is_err(),
                "no automatic playback when Spotify closes"
            );
            app.send_playback_action("resume", &ctx);
        }
        assert_eq!(
            rx.try_recv().unwrap().requests,
            vec![
                local_player::request("activate"),
                local_player::request("play")
            ]
        );
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn play_during_paused_start_resumes_once_when_receiver_is_ready() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.config.pause_on_start = true;
    app.local_starting = true;
    app.local_status.paired = false;
    app.send_playback_action("resume", &ctx);
    assert!(rx.try_recv().is_err());
    app.handle_local_event_with_ctx(
        local_player::Event::Status(local_player::Status {
            running: true,
            paired: true,
            ..Default::default()
        }),
        Some(&ctx),
    );
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
    assert!(rx.try_recv().is_err());
    assert!(app.config.pause_on_start);
}

#[test]
fn main_play_uses_selected_auralis_after_a_paused_start_releases_activity() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.config.pause_on_start = true;
    let ready = local_player::Status {
        running: true,
        paired: true,
        ..Default::default()
    };
    app.handle_local_event(local_player::Event::Status(ready.clone()));
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("activate")]
    );
    app.handle_local_event(local_player::Event::Status(local_player::Status {
        active: true,
        ..ready.clone()
    }));
    app.handle_local_event(local_player::Event::Status(ready));
    app.tx
        .send(AppEvent::PlayerSynced {
            playback: Ok(None),
            queue: None,
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(!app.follow_remote_device);
    assert_eq!(app.selected_output_name(), Some("Aurafy"));
    assert!(
        app.playback_devices()[0].is_active,
        "the paused local output stays selected"
    );
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, Vec::new());
    }
    let play = app.player_layout.unwrap().transport[2].center();
    ui_click(&mut app, &ctx, play);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
    assert!(
        app.config.pause_on_start,
        "pressing Play must not change the startup preference"
    );
    app.start_spotify_action(
        "volume".into(),
        |_| panic!("selected local volume must not use the Web API"),
        &ctx,
    );
    assert_eq!(rx.try_recv().unwrap().requests[0]["command"], "set_volume");
}

#[test]
fn default_local_selection_keeps_paused_bookmark_until_a_later_resume() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.config.pause_on_start = true;
    app.current_track = real_track("resume");
    app.duration_ms = 180000;
    app.progress_ms = 42000;
    app.progress = 42000.0 / 180000.0;
    app.handle_local_event(local_player::Event::Status(local_player::Status {
        running: true,
        paired: true,
        ..Default::default()
    }));
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("activate")]
    );
    app.handle_local_event(local_player::Event::Status(local_player::Status {
        running: true,
        paired: true,
        active: true,
        ..Default::default()
    }));
    app.handle_local_event(local_player::Event::Playback(PlaybackState {
        track: Some(real_track("resume")),
        device_name: Some("Aurafy".into()),
        ..Default::default()
    }));
    assert_eq!(app.current_progress_ms(), 42000);
    app.pending_handoff.as_mut().unwrap().captured = Instant::now() - Duration::from_secs(60);
    app.send_playback_action("resume", &ctx);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("play")]
    );
    app.handle_local_event(local_player::Event::Playback(PlaybackState {
        track: Some(real_track("resume")),
        is_playing: true,
        device_name: Some("Aurafy".into()),
        ..Default::default()
    }));
    assert_eq!(rx.try_recv().unwrap().requests[0]["position_ms"], 42000);
}

#[test]
fn startup_also_resumes_a_restored_already_active_device() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.handle_local_event(local_player::Event::Status(local_player::Status {
        running: true,
        paired: true,
        active: true,
        ..Default::default()
    }));
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("play")]
    );
    app.handle_local_event(local_player::Event::Status(local_player::Status {
        running: true,
        paired: true,
        active: true,
        ..Default::default()
    }));
    assert!(
        rx.try_recv().is_err(),
        "automatic resume is only for initial startup"
    );
}

#[test]
fn saved_song_position_and_search_view_restore_before_network() {
    let ctx = egui::Context::default();
    let app = AurafyApp::build(
        &ctx,
        AppConfig {
            client_id: "test-client".into(),
            last_page: Page::Search,
            last_query: "candy".into(),
            last_playback: Some(PlaybackBookmark {
                track: real_track("candy"),
                position_ms: 37000,
            }),
            ..Default::default()
        },
    );
    assert!(app.page == Page::Search);
    assert_eq!(app.search_query, "candy");
    assert!(app.search_loading);
    assert_eq!(app.current_track.uri, "spotify:track:candy");
    assert_eq!(app.current_progress_ms(), 37000);
    assert!(
        !app.is_playing,
        "cached state must not pretend audio is playing"
    );
}

#[test]
fn config_replacement_preserves_session_and_search_metadata() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/session-tests");
    let path = directory.join(format!("config-{}.json", std::process::id()));
    let mut config = AppConfig {
        client_id: "test-client".into(),
        compact_rows: true,
        last_page: Page::Search,
        last_query: "candy".into(),
        last_playback: Some(PlaybackBookmark {
            track: real_track("candy"),
            position_ms: 37000,
        }),
        recent_search_tracks: vec![real_track("candy")],
        ..Default::default()
    };
    config.save_to(&path).unwrap();
    config.last_playback.as_mut().unwrap().position_ms = 42000;
    config.save_to(&path).unwrap();
    let restored: AppConfig = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(restored.client_id, "test-client");
    assert!(restored.compact_rows && restored.last_page == Page::Search);
    assert_eq!(restored.last_query, "candy");
    assert_eq!(restored.last_playback.unwrap().position_ms, 42000);
    assert_eq!(restored.recent_search_tracks[0].uri, "spotify:track:candy");
}

#[test]
fn native_snapshot_restores_song_before_play_without_overriding_phone() {
    let ctx = egui::Context::default();
    let (mut app, _) = local_fixture(&ctx);
    let state = PlaybackState {
        track: Some(real_track("native")),
        progress_ms: 42000,
        ..Default::default()
    };
    app.handle_local_event(local_player::Event::Playback(state.clone()));
    assert_eq!(app.current_track.uri, "spotify:track:native");
    assert_eq!(app.progress_ms, 42000);
    assert!(app.device_name.is_none());
    assert!(!app.is_playing);
    app.config.last_playback = Some(PlaybackBookmark {
        track: real_track("native"),
        position_ms: 37000,
    });
    app.handle_local_event(local_player::Event::Playback(state.clone()));
    assert_eq!(
        app.progress_ms, 42000,
        "fresh native position should supersede an older bookmark"
    );
    app.current_track = real_track("phone");
    app.device_name = Some("Phone".into());
    app.handle_local_event(local_player::Event::Playback(state));
    assert_eq!(app.current_track.uri, "spotify:track:phone");
}

#[test]
fn resume_uses_existing_context_and_corrects_only_a_reset_position() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.current_track = real_track("resume");
    app.duration_ms = 180000;
    app.progress_ms = 37000;
    app.progress = 37000.0 / 180000.0;
    app.device_name = Some("Aurafy".into());
    app.send_playback_action("resume", &ctx);
    let job = rx.try_recv().unwrap();
    assert_eq!(job.action, "resume");
    assert_eq!(
        job.requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
    app.handle_local_event(local_player::Event::Status(local_player::Status {
        running: true,
        paired: true,
        active: true,
        ..Default::default()
    }));
    app.handle_local_event(local_player::Event::Playback(PlaybackState {
        track: Some(real_track("resume")),
        progress_ms: 37000,
        device_name: Some("Aurafy".into()),
        ..Default::default()
    }));
    assert!(
        app.pending_handoff.is_some(),
        "activation's paused snapshot must not consume the resume bookmark"
    );
    app.handle_local_event(local_player::Event::Playback(PlaybackState {
        track: Some(real_track("resume")),
        is_playing: true,
        progress_ms: 0,
        device_name: Some("Aurafy".into()),
        ..Default::default()
    }));
    let correction = rx.try_recv().unwrap();
    assert_eq!(correction.action, "restore_position");
    assert_eq!(correction.requests[0]["command"], "seek");
    assert_eq!(correction.requests[0]["position_ms"], 37000);
    assert!(
        rx.try_recv().is_err(),
        "handoff must never restart a track URI"
    );
}

#[test]
fn handoff_keeps_valid_positions_and_respects_new_tracks_and_user_seeks() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.current_track = real_track("old");
    app.duration_ms = 180000;
    app.progress_ms = 37000;
    app.progress = 37000.0 / 180000.0;
    app.capture_handoff();
    let pending = app.pending_handoff.as_mut().unwrap();
    assert!(
        pending
            .correction(&PlaybackState {
                track: Some(real_track("new")),
                ..Default::default()
            })
            .is_none()
    );
    assert!(
        pending
            .correction(&PlaybackState {
                track: Some(real_track("old")),
                progress_ms: 37000,
                ..Default::default()
            })
            .is_none()
    );
    pending.captured = Instant::now() - Duration::from_secs(11);
    assert!(
        pending
            .correction(&PlaybackState {
                track: Some(real_track("old")),
                ..Default::default()
            })
            .is_none()
    );
    app.local_status.active = true;
    app.start_spotify_action(
        "seek".into(),
        |_| panic!("local seek must not call the Web API"),
        &ctx,
    );
    assert!(app.pending_handoff.is_none());
    assert_eq!(rx.try_recv().unwrap().requests[0]["command"], "seek");
}

#[test]
fn paused_zero_snapshot_cannot_erase_the_startup_bookmark() {
    let ctx = egui::Context::default();
    let mut app = AurafyApp::build(
        &ctx,
        AppConfig {
            local_playback: true,
            last_playback: Some(PlaybackBookmark {
                track: real_track("resume"),
                position_ms: 42000,
            }),
            ..Default::default()
        },
    );
    app.apply_playback_state(PlaybackState {
        track: Some(real_track("resume")),
        progress_ms: 0,
        ..Default::default()
    });
    assert_eq!(app.progress_ms, 42000);
    assert_eq!(
        app.config.last_playback.as_ref().unwrap().position_ms,
        42000
    );
    app.apply_playback_state(PlaybackState {
        track: Some(real_track("resume")),
        progress_ms: 1500,
        is_playing: true,
        ..Default::default()
    });
    assert_eq!(
        app.progress_ms, 1500,
        "an active session remains authoritative"
    );
    assert!(!app.restoring_playback);
}

#[test]
fn recent_songs_deduplicate_and_persist_in_play_order() {
    let ctx = egui::Context::default();
    let mut app = AurafyApp::build(&ctx, AppConfig::default());
    for id in 0..15 {
        app.remember_search_track(&real_track(&id.to_string()));
    }
    app.remember_search_track(&real_track("8"));
    let restored: AppConfig =
        serde_json::from_str(&serde_json::to_string(&app.config).unwrap()).unwrap();
    assert_eq!(restored.recent_search_tracks.len(), 12);
    assert_eq!(restored.recent_search_tracks[0].uri, "spotify:track:8");
    assert_eq!(
        restored
            .recent_search_tracks
            .iter()
            .filter(|t| t.uri == "spotify:track:8")
            .count(),
        1
    );
}

fn ui_frame(app: &mut AurafyApp, ctx: &egui::Context, events: Vec<egui::Event>) {
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            events,
            ..Default::default()
        },
        |ctx| draw(app, ctx),
    );
}

fn label_center(app: &mut AurafyApp, ctx: &egui::Context, label: &str) -> Pos2 {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            ..Default::default()
        },
        |ctx| draw(app, ctx),
    );
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.pos + text.galley.size() * 0.5)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing visible label: {label}"))
}

#[test]
fn account_menu_routes_profile_and_keeps_background_preferences_independent() {
    let ctx = egui::Context::default();
    let mut app = AurafyApp::build(&ctx, AppConfig::default());
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    ui_click(&mut app, &ctx, Pos2::new(1270.0, 41.0));
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let point = label_center(&mut app, &ctx, "Keep running when I close Aurafy");
    ui_click(&mut app, &ctx, point);
    assert!(app.config.quit_on_close);
    assert!(!app.config.pause_on_start);
    let point = label_center(&mut app, &ctx, "Resume music when Aurafy opens");
    ui_click(&mut app, &ctx, point);
    assert!(app.config.pause_on_start);
    assert!(app.config.quit_on_close);
    let point = label_center(&mut app, &ctx, "Profile");
    ui_click(&mut app, &ctx, point);
    assert!(app.page == Page::Profile);
}

#[test]
fn library_changes_wait_for_confirmation_and_old_snapshots_cannot_undo_a_save() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.saved_tracks.clear();
    app.saved_track_ids.clear();
    let track = real_track("SaveTrack123");
    let old_revision = app.library_revision;
    app.toggle_saved_track(track.clone(), &ctx);
    assert!(!app.saved_track_ids.contains(&track.id));
    assert_eq!(app.library_changes.len(), 1);
    app.toggle_saved_track(track.clone(), &ctx);
    assert_eq!(app.library_changes.len(), 1);
    let respond = |app: &mut AurafyApp, action: &str, result: Result<(), String>| {
        app.tx
            .send(AppEvent::ActionFinished {
                action: action.into(),
                result,
                token: None,
            })
            .unwrap();
        app.handle_events(&ctx);
    };
    respond(&mut app, "save:SaveTrack123", Err("Save failed".into()));
    assert!(!app.saved_track_ids.contains(&track.id));
    assert!(app.library_changes.is_empty());
    app.toggle_saved_track(track.clone(), &ctx);
    respond(&mut app, "save:SaveTrack123", Ok(()));
    assert!(app.saved_track_ids.contains(&track.id));
    assert_eq!(app.saved_tracks.len(), 1);
    app.tx
        .send(AppEvent::LibraryLoaded {
            revision: old_revision,
            saved_tracks: Ok(vec![]),
            playlists: Ok(vec![]),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.saved_track_ids.contains(&track.id));
    app.toggle_saved_track(track.clone(), &ctx);
    respond(&mut app, "unsave:SaveTrack123", Err("Remove failed".into()));
    assert!(app.saved_track_ids.contains(&track.id));
    app.toggle_saved_track(track.clone(), &ctx);
    respond(&mut app, "unsave:SaveTrack123", Ok(()));
    assert!(!app.saved_track_ids.contains(&track.id));
    assert!(app.saved_tracks.is_empty());
}

#[test]
fn playlist_picker_keeps_failed_adds_retryable_and_confirms_success() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    app.playlists = vec![Playlist {
        id: "Editable123".into(),
        name: "Editable target".into(),
        owner_id: "preview-owner".into(),
        items_accessible: true,
        total_items: 2,
        ..Default::default()
    }];
    let track = real_track("PlaylistTrack123");
    app.open_playlist_picker(track.clone());
    for _ in 0..4 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    // A unique label in the centered dialog, away from the sidebar duplicate.
    // Read the real rendered button, selecting the dialog's larger x coordinate.
    let locate = |app: &mut AurafyApp| {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                ..Default::default()
            },
            |ctx| draw(app, ctx),
        );
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.text == "Editable target" && text.pos.x > 216.0 =>
                {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
            .expect("The editable playlist button must be rendered inside the dialog")
    };
    let point = locate(&mut app);
    ui_click(&mut app, &ctx, point);
    assert_eq!(app.playlist_add_pending.as_deref(), Some("Editable123"));
    assert!(app.show_add_to_playlist);
    app.tx
        .send(AppEvent::ActionFinished {
            action: "playlist_add:Editable123:PlaylistTrack123".into(),
            result: Err("Spotify denied this write".into()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.playlist_add_pending.is_none());
    assert!(app.show_add_to_playlist);
    assert_eq!(
        app.track_to_add_to_playlist.as_ref().unwrap().uri,
        track.uri
    );
    assert!(app.playlist_add_error.is_some());
    for _ in 0..4 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let point = locate(&mut app);
    ui_click(&mut app, &ctx, point);
    assert_eq!(app.playlist_add_pending.as_deref(), Some("Editable123"));
    app.tx
        .send(AppEvent::ActionFinished {
            action: "playlist_add:Editable123:PlaylistTrack123".into(),
            result: Ok(()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.show_add_to_playlist);
    assert!(app.playlist_picker_added.contains("Editable123"));
    assert_eq!(app.playlists[0].total_items, 3);
    // A confirmed add cannot be submitted a second time from this menu.
    app.add_track_to_playlist(app.playlists[0].clone(), track.clone(), &ctx);
    assert!(app.playlist_add_pending.is_none());
    for _ in 0..4 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let done = label_center(&mut app, &ctx, "Done");
    ui_click(&mut app, &ctx, done);
    assert!(!app.show_add_to_playlist);
}

#[test]
fn playlist_menu_shows_only_owned_playlists_and_stays_compact_above_player() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    app.playlists = vec![
        Playlist {
            id: "Mine1".into(),
            name: "My own mix".into(),
            owner_id: "preview-owner".into(),
            owner: "Different display label".into(),
            total_items: 42,
            items_accessible: true,
            ..Default::default()
        },
        Playlist {
            id: "Other2".into(),
            name: "Followed mix".into(),
            owner_id: "other-owner".into(),
            owner: "grid".into(),
            items_accessible: true,
            ..Default::default()
        },
        Playlist {
            id: "Unknown3".into(),
            name: "Unknown ownership".into(),
            owner: "grid".into(),
            items_accessible: true,
            ..Default::default()
        },
    ];
    app.open_playlist_picker(real_track("OwnerTrack123"));
    for _ in 0..8 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let popup = app.playlist_picker_rect.unwrap();
    assert!(popup.width() <= 310.0 && popup.height() < 500.0);
    assert!(popup.bottom() < app.current_track_menu_rect.unwrap().top());
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            ..Default::default()
        },
        |ctx| draw(&mut app, ctx),
    );
    let labels: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if popup.contains(text.pos) => {
                Some(text.galley.job.text.as_str())
            }
            _ => None,
        })
        .collect();
    assert!(labels.contains(&"My own mix"));
    assert!(labels.contains(&"42 songs"));
    assert!(!labels.contains(&"Followed mix"));
    assert!(!labels.contains(&"Unknown ownership"));
    app.add_track_to_playlist(app.playlists[1].clone(), real_track("OwnerTrack123"), &ctx);
    assert!(app.playlist_add_pending.is_none());
    assert!(app.playlist_add_error.is_some());
    app.user_profile = None;
    ui_frame(&mut app, &ctx, vec![]);
    assert!(app.playlist_add_pending.is_none());
}

#[test]
fn artist_click_browses_credits_without_replacing_playback_or_search() {
    let ctx = egui::Context::default();
    let mut app = AurafyApp::build(&ctx, AppConfig::default());
    app.current_track.artist = "Kanye West, Jay-Z".into();
    app.current_track.artist_names = vec!["Kanye West".into(), "Jay-Z".into()];
    app.search_query = "my search".into();
    let original = app.current_track.id.clone();
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let artist = label_center(&mut app, &ctx, "Jay-Z");
    ui_click(&mut app, &ctx, artist);
    assert!(app.page == Page::Artist);
    assert_eq!(app.artist_name, "Jay-Z");
    assert_eq!(app.current_track.id, original);
    assert_eq!(app.search_query, "my search");
    assert!(!app.is_playing);
}

#[test]
fn recent_and_result_artists_navigate_without_playing_the_search_song() {
    for popup in [true, false] {
        let ctx = egui::Context::default();
        let (mut app, rx) = local_fixture(&ctx);
        app.local_status.active = true;
        let original = app.current_track.uri.clone();
        let mut track = real_track("artistlink");
        track.artist = "Search Artist".into();
        track.artist_names = vec!["Search Artist".into()];
        track.artist_ids = vec!["ArtistOne".into()];
        if popup {
            app.config.recent_search_tracks = vec![track];
        } else {
            app.page = Page::Search;
            app.search_query = "artist query".into();
            app.last_sent_query = app.search_query.clone();
            app.search_results = vec![track];
        }
        for _ in 0..3 {
            ui_frame(&mut app, &ctx, vec![]);
        }
        if popup {
            let search_center = app.search_rect.unwrap().center();
            ui_click(&mut app, &ctx, search_center);
        }
        let artist = label_center(&mut app, &ctx, "Search Artist");
        ui_click(&mut app, &ctx, artist);
        assert!(app.page == Page::Artist);
        assert_eq!(app.browse.artist_id, "ArtistOne");
        assert_eq!(app.current_track.uri, original);
        assert!(
            rx.try_recv().is_err(),
            "artist navigation must not start playback"
        );
        if !popup {
            assert_eq!(app.search_query, "artist query");
        }
    }
}

#[test]
fn player_title_opens_its_album_without_changing_the_track_position_or_output() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.local_status.active = true;
    app.current_track = real_track("albumlink");
    app.current_track.name = "Album navigation song".into();
    app.current_track.album = "The actual album".into();
    app.current_track.album_id = "AlbumOne".into();
    app.progress_ms = 45000;
    let title = app.current_track.name.clone();
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let point = label_center(&mut app, &ctx, &title);
    ui_click(&mut app, &ctx, point);
    assert!(app.page == Page::Album);
    assert_eq!(app.browse.album_seed.as_ref().unwrap().album_id, "AlbumOne");
    assert_eq!(app.current_track.uri, "spotify:track:albumlink");
    assert_eq!(app.progress_ms, 45000);
    assert!(app.local_status.active);
    assert!(rx.try_recv().is_err());
}

fn catalogue_fixture(app: &mut AurafyApp) -> catalog::ArtistOverview {
    catalog::ArtistOverview {
        artist: catalog::Artist {
            id: "ArtistOne".into(),
            name: "Preview Artist".into(),
            image_url: None,
        },
        songs: catalog::SongPage {
            items: (0..23)
                .map(|i| {
                    let mut track = real_track(&format!("catalogue{i}"));
                    track.name = format!("Song {}", i + 1);
                    track.artist = "Preview Artist".into();
                    track.artist_names = vec!["Preview Artist".into()];
                    track.artist_ids = vec!["ArtistOne".into()];
                    track.album_id = "AlbumOne".into();
                    track.artwork_url = app
                        .demo_tracks
                        .get(i % app.demo_tracks.len())
                        .and_then(|track| track.artwork_url.clone());
                    track
                })
                .collect(),
            next: Some(catalog::SongCursor {
                group: "album".into(),
                offset: 1,
            }),
        },
        albums: catalog::Page {
            items: (0..12)
                .map(|i| catalog::Album {
                    id: format!("Album{i}"),
                    name: format!("Release {}", i + 1),
                    kind: if i % 3 == 0 { "single" } else { "album" }.into(),
                    release_date: format!("{}-06-01", 2026 - i),
                    total_tracks: 23,
                    artist_names: vec!["Preview Artist".into()],
                    artist_ids: vec!["ArtistOne".into()],
                    artwork_url: app
                        .recent_tracks
                        .get(i % app.recent_tracks.len())
                        .and_then(|track| track.artwork_url.clone()),
                })
                .collect(),
            next_offset: Some(12),
        },
        notice: None,
    }
}

#[test]
fn catalogue_pages_append_deduplicate_and_ignore_responses_for_a_previous_view() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.open_artist_id("Preview Artist".into(), "ArtistOne".into(), &ctx);
    let generation = app.browse.generation;
    let overview = catalogue_fixture(&mut app);
    app.tx
        .send(AppEvent::ArtistLoaded {
            generation,
            result: Ok(overview.clone()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert_eq!(app.artist_tracks.len(), 23);
    app.load_more_artist(&ctx);
    assert!(app.browse.more_loading);
    app.tx
        .send(AppEvent::ArtistSongsLoaded {
            generation,
            result: Ok(catalog::SongPage {
                items: vec![overview.songs.items[0].clone(), real_track("nextpage")],
                next: None,
            }),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert_eq!(app.artist_tracks.len(), 24);
    assert!(!app.browse.more_loading);
    app.tx
        .send(AppEvent::ArtistAlbumsLoaded {
            generation,
            result: Ok(catalog::Page {
                items: vec![
                    overview.albums.items[0].clone(),
                    catalog::Album {
                        id: "NextAlbum".into(),
                        ..Default::default()
                    },
                ],
                next_offset: None,
            }),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert_eq!(app.browse.albums.len(), 13);
    app.open_album(overview.albums.items[0].as_track(), &ctx);
    app.tx
        .send(AppEvent::ArtistLoaded {
            generation,
            result: Ok(catalog::ArtistOverview::default()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert_eq!(app.artist_name, "Preview Artist");
    app.tx
        .send(AppEvent::AlbumLoaded {
            generation: app.browse.generation,
            result: Ok(catalog::AlbumDetails {
                album: overview.albums.items[0].clone(),
                tracks: overview.songs.items,
            }),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert_eq!(app.browse.album.as_ref().unwrap().tracks.len(), 23);
    assert!(!app.browse.album_loading);
}

#[test]
fn render_catalogue_previews() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/ui-previews");
    fs::create_dir_all(&directory).unwrap();
    for (name, releases, album, width) in [
        ("artist-songs", false, false, 1440),
        ("artist-releases", true, false, 1080),
        ("album", false, true, 1440),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.show_queue = true;
        let overview = catalogue_fixture(&mut app);
        app.open_artist_id("Preview Artist".into(), "ArtistOne".into(), &ctx);
        app.tx
            .send(AppEvent::ArtistLoaded {
                generation: app.browse.generation,
                result: Ok(overview.clone()),
                token: None,
            })
            .unwrap();
        app.handle_events(&ctx);
        app.browse.releases = releases;
        if album {
            app.open_album(overview.albums.items[0].as_track(), &ctx);
            app.browse.album_loading = false;
            app.browse.album = Some(catalog::AlbumDetails {
                album: overview.albums.items[0].clone(),
                tracks: overview.songs.items,
            });
        }
        render_app(&mut app, &ctx, width, 900)
            .save(directory.join(format!("{name}.png")))
            .unwrap();
    }
}

#[test]
fn collection_shortcuts_scroll_with_playlists_while_primary_navigation_stays_fixed() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    app.playlists = (0..50)
        .map(|i| Playlist {
            id: format!("scroll{i}"),
            name: format!("Scroll playlist {i}"),
            ..Default::default()
        })
        .collect();
    let frame = |app: &mut AurafyApp, events: Vec<egui::Event>| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                events,
                ..Default::default()
            },
            |ctx| draw(app, ctx),
        )
    };
    let label_y = |output: &egui::FullOutput, label: &str| {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.pos.x < 216.0 && text.galley.job.text == label => {
                Some(text.pos.y)
            }
            _ => None,
        })
    };
    for _ in 0..3 {
        frame(&mut app, vec![]);
    }
    let before = frame(&mut app, vec![]);
    let home = label_y(&before, "Home").unwrap();
    let liked = label_y(&before, "Liked Songs").unwrap();
    let playlist = label_y(&before, "Scroll playlist 0").unwrap();
    frame(
        &mut app,
        vec![
            egui::Event::PointerMoved(Pos2::new(110.0, liked + 4.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, -260.0),
                modifiers: Default::default(),
            },
        ],
    );
    for _ in 0..18 {
        frame(&mut app, vec![]);
    }
    let after = frame(&mut app, vec![]);
    assert_eq!(label_y(&after, "Home"), Some(home));
    assert!(label_y(&after, "Liked Songs").is_none_or(|y| y < liked - 100.0));
    assert!(label_y(&after, "Scroll playlist 0").is_none_or(|y| y < playlist - 100.0));
}

#[test]
fn integrated_window_buttons_issue_native_commands_and_do_not_cover_search() {
    for (x, expected) in [
        (1334.0, "minimize"),
        (1374.0, "maximize"),
        (1414.0, "close"),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        for _ in 0..3 {
            ui_frame(&mut app, &ctx, vec![]);
        }
        assert!(app.search_rect.unwrap().right() < 1251.0);
        let mut commands = Vec::new();
        for pressed in [true, false] {
            let pos = Pos2::new(x, 32.0);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ],
                    ..Default::default()
                },
                |ctx| draw(&mut app, ctx),
            );
            commands.extend(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .clone(),
            );
        }
        assert!(
            commands.iter().any(|command| matches!(
                (expected, command),
                ("minimize", egui::ViewportCommand::Minimized(true))
                    | ("maximize", egui::ViewportCommand::Maximized(true))
                    | ("close", egui::ViewportCommand::Close)
            )),
            "Missing {expected} command"
        );
    }
}

#[test]
fn single_collection_button_opens_liked_songs_and_playlists_without_changing_playback() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.current_track = real_track("MenuTrack123");
    let original = app.current_track.uri.clone();
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let more = app.current_track_menu_rect.unwrap().center();
    ui_click(&mut app, &ctx, more);
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    assert!(app.show_add_to_playlist);
    assert!(app.playlist_picker_rect.is_some());
    assert_eq!(app.track_to_add_to_playlist.as_ref().unwrap().uri, original);
    assert!(!app.is_playing);
}

#[test]
fn liked_songs_in_the_collection_menu_waits_for_confirmation_and_can_unsave() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.current_track = real_track("Collection123");
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let collection = app.player_layout.unwrap().collection.center();
    ui_click(&mut app, &ctx, collection);
    let liked = |app: &mut AurafyApp| {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                ..Default::default()
            },
            |ctx| draw(app, ctx),
        );
        let popup = app.playlist_picker_rect.unwrap();
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text)
                    if text.galley.job.text == "Liked Songs" && popup.contains(text.pos) =>
                {
                    Some(text.pos + text.galley.size() * 0.5)
                }
                _ => None,
            })
            .expect("Liked Songs belongs in the save menu")
    };
    let point = liked(&mut app);
    ui_click(&mut app, &ctx, point);
    assert!(app.library_changes["Collection123"].1);
    assert!(!app.saved_track_ids.contains("Collection123"));
    app.tx
        .send(AppEvent::ActionFinished {
            action: "save:Collection123".into(),
            result: Ok(()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.saved_track_ids.contains("Collection123"));
    let point = liked(&mut app);
    ui_click(&mut app, &ctx, point);
    assert!(!app.library_changes["Collection123"].1);
    app.tx
        .send(AppEvent::ActionFinished {
            action: "unsave:Collection123".into(),
            result: Ok(()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(!app.saved_track_ids.contains("Collection123"));
    assert_eq!(app.current_track.uri, "spotify:track:Collection123");
}

#[test]
fn integrated_header_drags_and_border_handles_resize_the_native_window() {
    for (start, end, resize) in [
        (Pos2::new(1000.0, 41.0), Pos2::new(1040.0, 45.0), false),
        (Pos2::new(2.0, 2.0), Pos2::new(2.0, 2.0), true),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        let mut commands = Vec::new();
        let mut frame = |events: Vec<egui::Event>| {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                    events,
                    ..Default::default()
                },
                |ctx| draw(&mut app, ctx),
            );
            commands.extend(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .clone(),
            );
        };
        for _ in 0..3 {
            frame(vec![egui::Event::PointerMoved(start)]);
        }
        frame(vec![egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }]);
        frame(vec![egui::Event::PointerMoved(end)]);
        assert!(
            commands.iter().any(|command| if resize {
                matches!(
                    command,
                    egui::ViewportCommand::BeginResize(egui::ResizeDirection::NorthWest)
                )
            } else {
                matches!(command, egui::ViewportCommand::StartDrag)
            }),
            "Missing native drag/resize command"
        );
    }
}

#[test]
fn transport_stays_centered_across_window_sizes_and_queue_layouts() {
    for width in [1080.0, 1440.0, 1920.0] {
        for queue in [false, true] {
            let ctx = egui::Context::default();
            let mut app = AurafyApp::build(&ctx, AppConfig::default());
            app.show_queue = queue;
            let mut centers = Vec::new();
            for _ in 0..3 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.0))),
                        ..Default::default()
                    },
                    |ctx| draw(&mut app, ctx),
                );
                centers = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Circle(circle)
                            if circle.radius == 16.0 && circle.center.y > 800.0 =>
                        {
                            Some(circle.center.x)
                        }
                        _ => None,
                    })
                    .collect();
            }
            assert_eq!(centers.len(), 1);
            assert!(
                (centers[0] - width * 0.5).abs() <= 1.0,
                "center {:?} for width {width}",
                centers
            );
        }
    }
}

#[test]
fn player_uses_80_pixels_and_keeps_controls_within_its_bounds() {
    for (width, height) in [(1080.0, 680.0), (1440.0, 900.0), (1920.0, 1080.0)] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        let mut remaining = Rect::NOTHING;
        for _ in 0..3 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height))),
                    ..Default::default()
                },
                |ctx| {
                    app.draw_player(ctx);
                    remaining = ctx.available_rect();
                },
            );
        }
        assert!((remaining.bottom() - (height - 80.0)).abs() < 0.1);
        let bounds = Rect::from_min_max(Pos2::new(0.0, height - 80.0), Pos2::new(width, height));
        assert!(bounds.contains_rect(app.current_track_menu_rect.unwrap()));
        assert!(bounds.contains_rect(app.device_button_rect.unwrap()));
    }
}

#[test]
fn player_metadata_stays_clear_and_transport_has_the_approved_14_pixel_top_gap() {
    for width in [1080.0, 1260.0, 1440.0, 1680.0, 1920.0] {
        for title in [
            "Runaway",
            "A very long title that should be truncated while leaving the save button clear",
        ] {
            let ctx = egui::Context::default();
            let mut app = fixture(&ctx);
            app.current_track.name = title.into();
            app.current_track.artist = "Kanye West, Pusha T".into();
            app.current_track.artist_names = vec!["Kanye West".into(), "Pusha T".into()];
            for _ in 0..3 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 900.0))),
                        ..Default::default()
                    },
                    |ctx| draw(&mut app, ctx),
                );
            }
            let layout = app.player_layout.unwrap();
            assert!((layout.transport[2].top() - layout.bounds.top() - 14.0).abs() < 0.001);
            assert!((layout.transport[2].width() - 32.0).abs() < 0.001);
            assert!((layout.seek.center().y - layout.bounds.top() - 64.0).abs() < 0.001);
            assert!((layout.volume.center().y - layout.bounds.top() - 40.0).abs() < 0.001);
            assert!(layout.artist.right() + 5.0 < layout.collection.left());
            assert!(layout.title.right() + 5.0 < layout.collection.left());
            assert!(layout.title.bottom() < layout.artist.top() - 1.0);
            assert!(layout.seek.top() > layout.transport[2].bottom());
        }
    }
}

#[test]
fn fullscreen_utility_requests_native_fullscreen() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let point = app.player_layout.unwrap().utilities[4].center();
    let mut commands = Vec::new();
    for pressed in [true, false] {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                events: vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
        commands.extend(
            output.viewport_output[&egui::ViewportId::ROOT]
                .commands
                .clone(),
        );
    }
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Fullscreen(true)))
    );
}

#[test]
fn lyrics_button_preserves_the_page_and_ignores_a_previous_songs_response() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    app.search_query = "my search".into();
    app.progress_ms = 20_000;
    app.current_track.uri = "spotify:track:lyrics-first".into();
    let original = app.current_track.clone();
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let lyrics_button = app.player_layout.unwrap().utilities[0].center();
    ui_click(&mut app, &ctx, lyrics_button);
    assert!(app.page == Page::Lyrics);
    let first = lyrics::TrackKey::for_track(&original).unwrap();
    assert_eq!(app.lyrics.pending.as_ref(), Some(&first));
    app.current_track = app.demo_tracks[1].clone();
    app.current_track.uri = "spotify:track:lyrics-second".into();
    let second = lyrics::TrackKey::for_track(&app.current_track).unwrap();
    app.tx
        .send(AppEvent::LyricsLoaded {
            key: first.clone(),
            result: Ok(Some(lyrics::Lyrics {
                plain: "An original test lyric for the previous song".into(),
                ..Default::default()
            })),
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.lyrics.get(&first).is_some());
    assert!(app.lyrics.get(&second).is_none());
    ui_frame(&mut app, &ctx, vec![]);
    let lyrics_button = app.player_layout.unwrap().utilities[0].center();
    ui_click(&mut app, &ctx, lyrics_button);
    assert!(app.page == Page::Search);
    assert_eq!(app.search_query, "my search");
    assert_eq!(app.progress_ms, 20_000);
    assert!(!app.is_playing);
}

fn fixture_lyrics(app: &mut AurafyApp) {
    if app.current_track.uri.is_empty() {
        app.current_track.uri = "spotify:track:lyrics-preview".into();
    }
    let key = lyrics::TrackKey::for_track(&app.current_track).unwrap();
    let lines = (0..12)
        .map(|i| lyrics::Line {
            time_ms: i * 10_000,
            text: match i {
                0 => "An original line for the layout preview",
                1 => "The next line follows the music",
                2 => "A little room to breathe",
                3 => "Keep your place in the song",
                _ => "Another original line for the preview",
            }
            .into(),
        })
        .collect();
    app.lyrics.finish(
        key,
        Ok(Some(lyrics::Lyrics {
            lines,
            ..Default::default()
        })),
    );
}

#[test]
fn timed_lyrics_seek_the_native_player_and_mute_restores_the_volume() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.page = Page::Lyrics;
    app.local_status.active = true;
    app.current_track = real_track("lyrics");
    app.duration_ms = app.current_track.duration_ms;
    fixture_lyrics(&mut app);
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let point = label_center(&mut app, &ctx, "The next line follows the music");
    ui_click(&mut app, &ctx, point);
    assert_eq!(app.progress_ms, 10_000);
    assert!(app.lyrics_follow);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![serde_json::json!({"type":"command","command":"seek","position_ms":10000})]
    );
    let volume = app.volume;
    let mute = app.player_layout.unwrap().utilities[3].center();
    ui_click(&mut app, &ctx, mute);
    assert_eq!(app.volume, 0.0);
    let _ = rx.try_recv().unwrap();
    app.handle_local_event(local_player::Event::CommandFinished {
        action: "volume".into(),
        result: Ok(()),
    });
    ui_click(&mut app, &ctx, mute);
    assert_eq!(app.volume, volume);
    let _ = rx.try_recv().unwrap();
}

#[test]
fn volume_drag_changes_audio_before_release_and_ignores_old_playback_snapshots() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.local_status.active = true;
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let volume_rect = app.player_layout.unwrap().volume;
    let at = |percent: f32| {
        Pos2::new(
            volume_rect.left() + 6.0 + percent * (volume_rect.width() - 12.0) / 100.0,
            volume_rect.center().y,
        )
    };
    let start = at(20.0);
    ui_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(start)]);
    ui_frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    assert!(app.volume_control.interacting);
    assert_eq!(rx.try_recv().unwrap().requests[0]["volume"], 20);
    for percent in [40.0, 70.0, 83.0] {
        ui_frame(&mut app, &ctx, vec![egui::Event::PointerMoved(at(percent))]);
    }
    assert!(
        rx.try_recv().is_err(),
        "intermediate requests must coalesce"
    );
    app.apply_playback_state(PlaybackState {
        volume_percent: Some(20),
        ..Default::default()
    });
    assert!(
        (app.volume - 0.83).abs() < 0.001,
        "old state must not fight the thumb"
    );
    ui_frame(
        &mut app,
        &ctx,
        vec![egui::Event::PointerButton {
            pos: at(83.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    );
    app.handle_local_event(local_player::Event::CommandFinished {
        action: "volume".into(),
        result: Ok(()),
    });
    ui_frame(&mut app, &ctx, vec![]);
    assert_eq!(rx.try_recv().unwrap().requests[0]["volume"], 83);
    app.handle_local_event(local_player::Event::CommandFinished {
        action: "volume".into(),
        result: Ok(()),
    });
    app.apply_playback_state(PlaybackState {
        volume_percent: Some(20),
        ..Default::default()
    });
    assert!((app.volume - 0.83).abs() < 0.001);
    assert!(!app.volume_control.interacting);
}

#[test]
fn paused_lyrics_highlight_the_saved_position_and_seek_backwards() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.token = None;
    app.page = Page::Lyrics;
    app.progress_ms = 24_000;
    app.progress = 24_000.0 / app.duration_ms as f32;
    fixture_lyrics(&mut app);
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let key = lyrics::TrackKey::for_track(&app.current_track).unwrap();
    assert_eq!(app.lyrics_last_line, Some((key.clone(), Some(2))));
    app.seek_playback(0, &ctx);
    ui_frame(&mut app, &ctx, vec![]);
    assert_eq!(app.lyrics_last_line, Some((key, Some(0))));
    assert!(!app.is_playing);
}

#[test]
fn device_picker_opens_above_player_and_transfers_without_leaving_page() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.page = Page::Home;
    app.follow_remote_device = true;
    app.device_name = Some("Phone".into());
    app.devices = vec![Device {
        id: "phone".into(),
        name: "Phone".into(),
        device_type: "Smartphone".into(),
        is_active: true,
        volume_percent: Some(50),
    }];
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let button = app.device_button_rect.unwrap();
    ui_click(&mut app, &ctx, button.center());
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    assert!(app.device_picker_open);
    assert!(app.page == Page::Home);
    let popup = app.device_popup_rect.unwrap();
    assert!(popup.bottom() < button.top());
    assert!(popup.width() <= 320.0);
    assert!(popup.height() < 400.0);
    let local = label_center(&mut app, &ctx, "Aurafy");
    ui_click(&mut app, &ctx, local);
    assert!(app.page == Page::Home);
    assert!(!app.device_picker_open);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![
            local_player::request("activate"),
            local_player::request("play")
        ]
    );
}

#[test]
fn sleep_timer_pauses_once_in_the_tray_and_does_not_change_startup_preferences() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.config.pause_on_start = true;
    app.local_status.active = true;
    app.is_playing = true;
    app.window_state.hidden = true;
    app.sleep_timer = Some(preferences::SleepTimer::Deadline(
        Instant::now() - Duration::from_secs(1),
    ));
    app.tick_sleep_timer(&ctx);
    assert!(app.sleep_timer.is_none());
    assert!(app.config.pause_on_start);
    assert!(app.window_state.hidden);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("pause")]
    );
    app.tick_sleep_timer(&ctx);
    assert!(rx.try_recv().is_err());
    app.is_playing = false;
    app.sleep_timer = Some(preferences::SleepTimer::Deadline(Instant::now()));
    app.tick_sleep_timer(&ctx);
    assert!(rx.try_recv().is_err());
    app.is_playing = true;
    app.local_status.active = false;
    app.select_local_on_ready = true;
    app.sleep_timer = Some(preferences::SleepTimer::Deadline(Instant::now()));
    app.tick_sleep_timer(&ctx);
    assert_eq!(
        rx.try_recv().unwrap().requests,
        vec![local_player::request("pause")]
    );
    assert!(
        !app.select_local_on_ready,
        "a timer must not reclaim the output after a handoff"
    );
}

#[test]
fn search_privacy_clears_existing_history_and_prevents_saving_or_migration() {
    let ctx = egui::Context::default();
    let mut app = AurafyApp::build(&ctx, AppConfig::default());
    app.config.recent_searches.push("old query".into());
    app.config.recent_search_tracks.push(real_track("old"));
    app.search_query = "old query".into();
    app.set_search_history_enabled(false);
    app.run_search("Kanye".into(), &ctx);
    app.search_query = "Kanye".into();
    app.remember_search_track(&real_track("new"));
    app.migrate_recent_search_tracks();
    app.save_session();
    assert!(app.config.recent_searches.is_empty());
    assert!(app.config.recent_search_tracks.is_empty());
    assert!(app.config.last_query.is_empty());
    assert!(
        !app.search_results.is_empty(),
        "private searches must still work"
    );
    app.set_search_history_enabled(true);
    app.run_search("Kanye".into(), &ctx);
    app.remember_search_track(&real_track("new"));
    assert_eq!(app.config.recent_searches.len(), 1);
    assert_eq!(app.config.recent_search_tracks.len(), 1);
    app.clear_search_history();
    assert!(app.search_query.is_empty());
    assert!(app.config.last_query.is_empty());
}

#[test]
fn startup_volume_and_start_page_persist_without_overwriting_phone_volume() {
    let ctx = egui::Context::default();
    let cfg = AppConfig {
        last_local_volume: Some(57),
        last_page: Page::Search,
        last_query: "old query".into(),
        start_page: preferences::StartPage::Library,
        ..Default::default()
    };
    let app = AurafyApp::build(&ctx, cfg);
    assert!((app.volume - 0.57).abs() < 0.001);
    assert!(app.page == Page::Library);
    assert!(app.search_query.is_empty());
    let cfg: AppConfig =
        serde_json::from_str(&serde_json::to_string(&app.config).unwrap()).unwrap();
    assert_eq!(cfg.last_local_volume, Some(57));
    assert_eq!(cfg.cache_mb(), 100);
    let app = AurafyApp::build(
        &ctx,
        AppConfig {
            startup_volume: Some(23),
            last_local_volume: Some(57),
            cache_limit_mb: 500,
            ..Default::default()
        },
    );
    assert!((app.volume - 0.23).abs() < 0.001);
    assert_eq!(app.config.cache_mb(), 500);
    let (mut app, rx) = local_fixture(&ctx);
    app.config.last_local_volume = Some(57);
    app.follow_remote_device = true;
    app.volume = 0.1;
    app.save_session();
    assert_eq!(app.config.last_local_volume, Some(57));
    app.follow_remote_device = false;
    app.volume = 0.44;
    app.start_spotify_action(
        "volume".into(),
        |_| panic!("local volume must stay local"),
        &ctx,
    );
    assert_eq!(app.config.last_local_volume, Some(44));
    assert_eq!(rx.try_recv().unwrap().requests[0]["volume"], 44);
}

#[test]
fn hiding_artwork_releases_covers_keeps_avatar_and_stops_image_requests() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let avatar = "https://example.test/avatar".to_string();
    app.user_profile.as_mut().unwrap().image_url = Some(avatar.clone());
    let texture = ctx.load_texture(
        "test-avatar",
        egui::ColorImage::filled([2, 2], GREEN),
        Default::default(),
    );
    app.artwork_cache.insert(avatar.clone(), texture);
    assert!(app.artwork_cache.len() > 1);
    app.set_artwork_visible(false);
    assert_eq!(app.artwork_cache.len(), 1);
    assert!(app.artwork_cache.contains_key(&avatar));
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    assert!(app.artwork_pending.is_empty());
    app.set_artwork_visible(true);
    assert!(!app.config.hide_artwork);
}

fn test_cover_bytes() -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        2,
        image::Rgba([90, 150, 120, 255]),
    ))
    .write_to(&mut bytes, image::ImageFormat::Png)
    .unwrap();
    bytes.into_inner()
}

#[test]
fn artwork_downloads_stop_after_visible_rows_in_a_large_library_are_loaded() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    app.show_queue = true;
    app.playlists = (0..200)
        .map(|i| Playlist {
            id: format!("library-{i}"),
            name: format!("Playlist {i}"),
            image_url: Some(format!("https://example.test/playlist/{i}")),
            ..Default::default()
        })
        .collect();
    app.queue = (0..200)
        .map(|i| Track {
            artwork_url: Some(format!("https://example.test/queue/{i}")),
            ..real_track(&format!("queue-{i}"))
        })
        .collect();
    let mut downloads = HashSet::new();
    for _ in 0..24 {
        ui_frame(&mut app, &ctx, vec![]);
        let pending: Vec<_> = app.artwork_pending.iter().cloned().collect();
        for url in pending {
            downloads.insert(url.clone());
            app.tx
                .send(AppEvent::ArtworkLoaded {
                    url,
                    bytes: Ok(test_cover_bytes()),
                })
                .unwrap();
        }
        app.handle_events(&ctx);
    }
    assert!(
        downloads
            .iter()
            .all(|url| url.rsplit('/').next().unwrap().parse::<usize>().unwrap() < 20),
        "Offscreen rows requested artwork: {downloads:?}"
    );
    ui_frame(&mut app, &ctx, vec![]);
    assert!(
        app.artwork_pending.is_empty(),
        "A settled view must stop downloading artwork"
    );
    assert!(downloads.contains("https://example.test/playlist/0"));
    assert!(downloads.contains("https://example.test/queue/0"));
}

#[test]
fn loading_more_covers_cannot_evict_an_onscreen_song_or_playlist() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    while app.artwork_cache.len() < MAX_ARTWORK_CACHE {
        let url = format!("https://example.test/warm/{}", app.artwork_cache.len());
        let texture = ctx.load_texture(
            &url,
            egui::ColorImage::filled([2, 2], GREEN),
            Default::default(),
        );
        app.artwork_cache.insert(url, texture);
    }
    // The old arbitrary eviction always selected this entry, even while drawn.
    let visible = app.artwork_cache.keys().next().unwrap().clone();
    let original_id = app.artwork_cache.get(&visible).unwrap().id();
    app.current_track.artwork_url = Some(visible.clone());
    app.playlists[0].image_url = Some(visible.clone());
    for i in 0..20 {
        ui_frame(&mut app, &ctx, vec![]);
        app.tx
            .send(AppEvent::ArtworkLoaded {
                url: format!("https://example.test/new/{i}"),
                bytes: Ok(test_cover_bytes()),
            })
            .unwrap();
        app.handle_events(&ctx);
        assert_eq!(
            app.artwork_cache.get(&visible).map(|texture| texture.id()),
            Some(original_id),
            "A visible cover was evicted and would flash back to its placeholder"
        );
    }
}

#[test]
fn listening_timer_and_privacy_controls_work_from_the_settings_ui() {
    let ctx = egui::Context::default();
    let mut app = AurafyApp::build(&ctx, AppConfig::default());
    app.page = Page::Settings;
    app.settings_tab = SettingsTab::Listening;
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    ui_frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::PointerMoved(Pos2::new(1000.0, 600.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, -260.0),
                modifiers: Default::default(),
            },
        ],
    );
    for _ in 0..18 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let point = label_center(&mut app, &ctx, "Start timer");
    ui_click(&mut app, &ctx, point);
    assert!(matches!(
        app.sleep_timer,
        Some(preferences::SleepTimer::Deadline(_))
    ));
    let point = label_center(&mut app, &ctx, "Cancel timer");
    ui_click(&mut app, &ctx, point);
    assert!(app.sleep_timer.is_none());
    let point = label_center(&mut app, &ctx, "End of this song");
    ui_click(&mut app, &ctx, point);
    assert!(matches!(
        app.sleep_timer,
        Some(preferences::SleepTimer::EndOfTrack { .. })
    ));
    app.is_playing = true;
    app.advance_demo_track(1);
    app.tick_sleep_timer(&ctx);
    assert!(app.sleep_timer.is_none());
    assert!(!app.is_playing);
    app.settings_tab = SettingsTab::Privacy;
    ui_frame(
        &mut app,
        &ctx,
        vec![
            egui::Event::PointerMoved(Pos2::new(1000.0, 500.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, 900.0),
                modifiers: Default::default(),
            },
        ],
    );
    for _ in 0..18 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    app.config.recent_searches.push("private query".into());
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let point = label_center(&mut app, &ctx, "Save recent searches on this computer");
    ui_click(&mut app, &ctx, point);
    assert!(app.config.private_search);
    assert!(app.config.recent_searches.is_empty());
}

fn ui_click(app: &mut AurafyApp, ctx: &egui::Context, pos: Pos2) {
    ui_frame(app, ctx, vec![egui::Event::PointerMoved(pos)]);
    for pressed in [true, false] {
        ui_frame(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

#[test]
fn recent_popup_plays_song_directly_and_remove_does_not_play() {
    for remove in [false, true] {
        let ctx = egui::Context::default();
        let (mut app, rx) = local_fixture(&ctx);
        app.local_status.active = true;
        app.config.recent_search_tracks = vec![real_track("recent")];
        for _ in 0..3 {
            ui_frame(&mut app, &ctx, Vec::new());
        }
        let search_center = app.search_rect.unwrap().center();
        ui_click(&mut app, &ctx, search_center);
        for _ in 0..2 {
            ui_frame(&mut app, &ctx, Vec::new());
        }
        let popup = app
            .search_popup_rect
            .expect("search focus must open recent songs");
        let pos = Pos2::new(
            if remove {
                popup.right() - 39.0
            } else {
                popup.left() + 42.0
            },
            popup.top() + 96.0,
        );
        ui_click(&mut app, &ctx, pos);
        if remove {
            assert!(app.config.recent_search_tracks.is_empty());
            assert!(rx.try_recv().is_err(), "remove must not play the song");
        } else {
            let job = rx
                .try_recv()
                .expect("clicking recent artwork must play immediately");
            assert_eq!(
                job.requests,
                vec![
                    serde_json::json!({"type":"command","command":"play","uri":"spotify:track:recent"})
                ]
            );
            assert!(app.last_sent_query.is_empty());
            assert!(app.page == Page::Home);
            assert!(!app.search_popup_open);
        }
    }
}

#[test]
fn home_shortcuts_open_their_own_views() {
    for (x, expected) in [
        (360.0, Page::Liked),
        (760.0, Page::Recent),
        (1160.0, Page::Library),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.token = None; // These are UI checks, so no API calls are authorized.
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0));
        for _ in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| draw(&mut app, ctx),
            );
        }
        let pos = Pos2::new(x, 210.0);
        for pressed in [true, false] {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events: vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..Default::default()
                },
                |ctx| draw(&mut app, ctx),
            );
        }
        assert!(
            app.page == expected,
            "home shortcut at {x} opened the wrong page"
        );
    }
}

#[test]
fn late_api_response_does_not_reconnect_signed_out_account() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let old_token = app.token.take();
    app.tx
        .send(AppEvent::DevicesLoaded {
            devices: Ok(Vec::new()),
            token: old_token,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.token.is_none());
}

#[test]
fn clearing_search_resets_the_query_and_result_view() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.token = None;
    app.page = Page::Search;
    app.search_query = "old query".into();
    app.last_sent_query = "old query".into();
    app.search_results = app.saved_tracks.clone();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0));
    for _ in 0..2 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
    }
    let search_rect = app.search_rect.unwrap();
    let pos = Pos2::new(search_rect.right() - 28.0, search_rect.center().y);
    for pressed in [true, false] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(screen),
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
    }
    assert!(app.search_query.is_empty());
    assert!(app.last_sent_query.is_empty());
    assert!(app.search_results.is_empty());
    assert!(!app.search_loading);
}

#[test]
fn clearing_search_from_the_keyboard_discards_a_pending_result() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    app.search_query = "old query".into();
    app.last_sent_query = "old query".into();
    app.search_loading = true;
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0));
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        },
        |ctx| {
            ctx.memory_mut(|m| m.request_focus(egui::Id::new("global-search")));
            draw(&mut app, ctx);
        },
    );
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(screen),
            events: vec![
                egui::Event::Key {
                    key: Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::CTRL | egui::Modifiers::COMMAND,
                },
                egui::Event::Key {
                    key: Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        },
        |ctx| draw(&mut app, ctx),
    );
    assert!(app.search_query.is_empty());
    app.tx
        .send(AppEvent::SearchFinished {
            query: "old query".into(),
            result: Ok(vec![real_track("old")]),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.last_sent_query.is_empty());
    assert!(app.search_results.is_empty());
    assert!(!app.search_loading);
}

#[test]
fn switching_playlists_keeps_the_selected_playlist_loading() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.selected_playlist = Some(Playlist {
        id: "second".into(),
        ..Default::default()
    });
    app.playlist_loading = true;
    app.tx
        .send(AppEvent::PlaylistLoaded {
            playlist_id: "first".into(),
            tracks: Err("old error".into()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.playlist_loading);
    assert!(app.playlist_error.is_none());
    app.tx
        .send(AppEvent::PlaylistLoaded {
            playlist_id: "second".into(),
            tracks: Err("current error".into()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(!app.playlist_loading);
    assert_eq!(app.playlist_error.as_deref(), Some("current error"));
}

#[test]
fn render_ui_previews() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/ui-previews");
    fs::create_dir_all(&directory).unwrap();
    for (name, width, height, failed) in [
        ("playlist-picker", 1440, 900, false),
        ("playlist-picker-error", 1080, 680, true),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.page = Page::Search;
        app.open_playlist_picker(real_track("Preview123"));
        for i in 0..40 {
            app.playlists.push(Playlist {
                id: format!("list{i}"),
                name: format!("Playlist {}", i + 1),
                owner_id: "preview-owner".into(),
                items_accessible: true,
                ..Default::default()
            });
        }
        if failed {
            app.playlist_add_error = Some(
                "Spotify denied this action. Check your access to this playlist and try again."
                    .into(),
            );
        }
        render_app(&mut app, &ctx, width, height)
            .save(directory.join(format!("{name}.png")))
            .unwrap();
    }
    for width in [1080, 1193, 1440, 1920] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.current_track.name = "Demons and Angels (feat. Juice WRLD)".into();
        app.current_track.artist = "A Boogie Wit da Hoodie, Juice WRLD".into();
        app.current_track.artist_names = vec!["A Boogie Wit da Hoodie".into(), "Juice WRLD".into()];
        app.device_name = Some("Aurafy".into());
        app.is_playing = true;
        app.progress_ms = 72_000;
        app.progress = 0.36;
        let picture = render_app(&mut app, &ctx, width, 900);
        let player_height = player_layout::Geometry::HEIGHT as u32;
        image::imageops::crop_imm(
            &picture,
            0,
            picture.height() - player_height,
            width,
            player_height,
        )
        .to_image()
        .save(directory.join(format!("player-{width}.png")))
        .unwrap();
    }
    for (name, width, height, queue) in [
        ("lyrics", 1440, 900, false),
        ("lyrics-narrow", 1080, 680, true),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.page = Page::Lyrics;
        app.show_queue = queue;
        app.progress_ms = 24_000;
        app.progress = 24_000.0 / app.duration_ms as f32;
        fixture_lyrics(&mut app);
        render_app(&mut app, &ctx, width, height)
            .save(directory.join(format!("{name}.png")))
            .unwrap();
    }
    for (name, page, tab, width, height, queue, connected) in [
        (
            "home",
            Page::Home,
            SettingsTab::General,
            1440,
            900,
            false,
            true,
        ),
        (
            "search",
            Page::Search,
            SettingsTab::General,
            1440,
            900,
            false,
            true,
        ),
        (
            "profile",
            Page::Profile,
            SettingsTab::General,
            1440,
            900,
            false,
            true,
        ),
        (
            "settings-general",
            Page::Settings,
            SettingsTab::General,
            1440,
            900,
            false,
            true,
        ),
        (
            "settings-listening",
            Page::Settings,
            SettingsTab::Listening,
            1440,
            900,
            false,
            true,
        ),
        (
            "settings-privacy",
            Page::Settings,
            SettingsTab::Privacy,
            1440,
            900,
            false,
            true,
        ),
        (
            "settings-narrow",
            Page::Settings,
            SettingsTab::General,
            1080,
            680,
            true,
            true,
        ),
        (
            "settings-account",
            Page::Settings,
            SettingsTab::Account,
            1440,
            900,
            false,
            true,
        ),
        (
            "settings-interface",
            Page::Settings,
            SettingsTab::Appearance,
            1440,
            900,
            false,
            true,
        ),
        (
            "library",
            Page::Library,
            SettingsTab::General,
            1440,
            900,
            false,
            true,
        ),
        (
            "liked",
            Page::Liked,
            SettingsTab::General,
            1440,
            900,
            false,
            true,
        ),
        (
            "playlist",
            Page::Library,
            SettingsTab::General,
            1440,
            900,
            true,
            true,
        ),
        (
            "narrow-queue",
            Page::Search,
            SettingsTab::General,
            1080,
            680,
            true,
            true,
        ),
        (
            "narrow-home",
            Page::Home,
            SettingsTab::General,
            1080,
            680,
            false,
            true,
        ),
        (
            "connect",
            Page::Settings,
            SettingsTab::Account,
            1080,
            680,
            false,
            false,
        ),
    ] {
        let image = render(page, tab, width, height, queue, connected);
        assert_eq!(image.dimensions(), (width, height));
        image.save(directory.join(format!("{name}.png"))).unwrap();
    }
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.config.recent_search_tracks = app
        .saved_tracks
        .iter()
        .take(5)
        .enumerate()
        .map(|(i, track)| {
            let mut track = track.clone();
            track.uri = format!("spotify:track:preview{i}");
            track
        })
        .collect();
    app.search_popup_open = true;
    ctx.memory_mut(|m| m.request_focus(Id::new("global-search")));
    render_app(&mut app, &ctx, 1440, 900)
        .save(directory.join("recent-searches.png"))
        .unwrap();
    assert!(app.search_popup_rect.is_some());
    for (name, width, height) in [
        ("device-picker", 1440, 900),
        ("device-picker-narrow", 1080, 680),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        let (player, _) = local_player::Player::test_channel();
        app.local_player = Some(player);
        app.local_status.paired = true;
        app.local_status.active = true;
        app.devices.push(Device {
            id: "phone".into(),
            name: "realme P3 Ultra 5G".into(),
            device_type: "Smartphone".into(),
            is_active: false,
            volume_percent: Some(40),
        });
        app.device_picker_open = true;
        render_app(&mut app, &ctx, width, height)
            .save(directory.join(format!("{name}.png")))
            .unwrap();
    }
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    render_app_inner(&mut app, &ctx, 1440, 900, true)
        .save(directory.join("account-menu.png"))
        .unwrap();
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Artist;
    app.artist_name = "Kanye West".into();
    app.artist_tracks = app
        .demo_tracks
        .iter()
        .filter(|track| track.artist == app.artist_name)
        .cloned()
        .collect();
    render_app(&mut app, &ctx, 1440, 900)
        .save(directory.join("artist.png"))
        .unwrap();
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let (player, _) = local_player::Player::test_channel();
    app.local_player = Some(player);
    app.local_status = local_player::Status {
        running: true,
        paired: true,
        message: "Ready to play on this computer".into(),
        ..Default::default()
    };
    app.page = Page::Settings;
    render_app(&mut app, &ctx, 1440, 900)
        .save(directory.join("settings-local-ready.png"))
        .unwrap();
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Search;
    app.search_query = "Night".into();
    app.last_sent_query = "Night".into();
    app.search_loading = false;
    app.search_results = app.saved_tracks.iter().take(6).cloned().collect();
    app.search_catalog = Some(spotify::catalog::SearchCatalog {
        tracks: app.search_results.clone(),
        artists: vec![
            spotify::catalog::Artist {
                id: "a1".into(),
                name: "The Weeknd".into(),
                image_url: None,
            },
            spotify::catalog::Artist {
                id: "a2".into(),
                name: "Kavinsky".into(),
                image_url: None,
            },
        ],
        albums: vec![spotify::catalog::Album {
            id: "alb1".into(),
            name: "After Hours".into(),
            artwork_url: None,
            artist_names: vec!["The Weeknd".into()],
            artist_ids: vec!["a1".into()],
            release_date: "2020".into(),
            kind: "album".into(),
            total_tracks: 14,
        }],
        playlists: app.playlists.iter().take(2).cloned().collect(),
    });
    render_app(&mut app, &ctx, 1440, 900)
        .save(directory.join("search.png"))
        .unwrap();
}

#[test]
fn exhausted_local_context_replays_on_main_play_but_paused_context_resumes() {
    for idle in [false, true] {
        let ctx = egui::Context::default();
        let (mut app, rx) = local_fixture(&ctx);
        app.select_local_on_ready = false;
        app.current_track = real_track("finished-song");
        let uri = app.current_track.uri.clone();
        app.local_status.active = true;
        app.handle_local_event(local_player::Event::Playback(PlaybackState {
            is_idle: idle,
            is_playing: false,
            progress_ms: if idle { 0 } else { 42000 },
            device_name: Some(local_player::DEVICE_NAME.into()),
            ..Default::default()
        }));
        for _ in 0..3 {
            ui_frame(&mut app, &ctx, vec![]);
        }
        let play = app.player_layout.unwrap().transport[2].center();
        ui_click(&mut app, &ctx, play);
        let job = rx.try_recv().unwrap();
        assert_eq!(job.action, "resume");
        assert_eq!(job.requests.len(), 1);
        assert_eq!(job.requests[0]["command"], "play");
        if idle {
            assert_eq!(job.requests[0]["uri"], uri);
            assert!(app.pending_handoff.is_none());
            assert!(!app.restoring_playback);
        } else {
            assert!(
                job.requests[0].get("uri").is_none(),
                "ordinary pause must retain its native context"
            );
        }
    }
}

#[test]
fn selected_local_output_ignores_web_queue_and_remote_handoff_uses_remote_queue() {
    let ctx = egui::Context::default();
    let (mut app, _rx) = local_fixture(&ctx);
    app.select_local_on_ready = false;
    let track = real_track("repeatable");
    app.queue = vec![track.clone(), track.clone()];
    app.local_status.active = true;
    app.handle_local_event(local_player::Event::Queue(vec![]));
    assert!(
        app.queue.is_empty(),
        "an authoritative empty native queue clears previous entries"
    );
    app.local_status.active = false;
    for name in [local_player::DEVICE_NAME, "Phone"] {
        app.tx
            .send(AppEvent::PlayerSynced {
                playback: Ok(Some(PlaybackState {
                    track: Some(track.clone()),
                    is_playing: true,
                    device_name: Some(name.into()),
                    ..Default::default()
                })),
                queue: Some(Ok(vec![track.clone(), track.clone()])),
                token: None,
            })
            .unwrap();
        app.handle_events(&ctx);
        assert_eq!(app.queue.len(), if name == "Phone" { 2 } else { 0 });
    }
    app.local_status.active = true;
    app.handle_local_event(local_player::Event::Queue(vec![track.clone(), track]));
    assert_eq!(
        app.queue.len(),
        2,
        "intentional duplicate native queue entries must not be deduplicated"
    );
}

#[test]
fn playlist_table_uses_its_context_and_original_occurrence_not_single_song_autoplay() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Library;
    app.selected_playlist = Some(Playlist {
        id: "MyPlaylist".into(),
        ..Default::default()
    });
    let mut repeated = real_track("duplicate");
    repeated.context_position = Some(52);
    let selection = app.table_play_context(&repeated, 1).unwrap();
    assert_eq!(selection.uri, "spotify:playlist:MyPlaylist");
    assert_eq!(selection.position, 52);
    repeated.context_position = Some(53);
    assert_eq!(app.table_play_context(&repeated, 2).unwrap().position, 53);
    repeated.context_position = None;
    assert_eq!(app.table_play_context(&repeated, 0).unwrap().position, 0);
    app.page = Page::Search;
    assert!(
        app.table_play_context(&repeated, 0).is_none(),
        "search must not inherit a previously opened playlist"
    );
    app.page = Page::Library;
    app.selected_playlist = None;
    assert!(app.table_play_context(&repeated, 0).is_none());
}

#[test]
fn playlist_header_and_card_buttons_pause_the_playing_context_without_navigation() {
    for page in [Page::Library, Page::Home] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        let playlist = app.playlists[0].clone();
        if page == Page::Home {
            app.recent_tracks.clear();
        }
        app.page = page;
        app.playlist_loading = false;
        if page == Page::Library {
            app.selected_playlist = Some(playlist.clone());
        }
        app.playback_context_uri = Some(format!("spotify:playlist:{}", playlist.id));
        app.is_playing = true;
        let (player, rx) = local_player::Player::test_channel();
        app.local_player = Some(player);
        app.local_status = local_player::Status {
            running: true,
            paired: true,
            active: true,
            ..Default::default()
        };
        for _ in 0..3 {
            ui_frame(&mut app, &ctx, vec![]);
        }
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
        let radius = if page == Page::Library { 26.0 } else { 17.0 };
        let point = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle)
                    if (circle.radius - radius).abs() < 0.1 && circle.fill == GREEN =>
                {
                    Some(circle.center)
                }
                _ => None,
            })
            .expect("playlist play/pause button must be visible");
        ui_click(&mut app, &ctx, point);
        let job = rx
            .try_recv()
            .unwrap_or_else(|error| panic!("playlist button must control playback: {error:?}; home={}; point={point:?}; still_on_page={}; playing={}; context={:?}; status={}", page == Page::Home, app.page == page, app.is_playing, app.playback_context_uri, app.status_message));
        assert_eq!(job.requests, vec![local_player::request("pause")]);
        assert!(app.page == page, "card playback must not navigate away");
        assert_eq!(
            app.playback_context_uri,
            Some(format!("spotify:playlist:{}", playlist.id))
        );
    }
}

#[test]
#[ignore = "Manual offscreen visual review"]
fn render_home_appearance_previews() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/ui-previews/0.8.0");
    fs::create_dir_all(&directory).unwrap();
    for (name, theme, page, width, height, compact, native, zoom) in [
        (
            "home-dark",
            appearance::Theme::Dark,
            Page::Home,
            1440,
            900,
            false,
            false,
            100,
        ),
        (
            "home-light",
            appearance::Theme::Light,
            Page::Home,
            1080,
            720,
            true,
            false,
            100,
        ),
        (
            "appearance-black",
            appearance::Theme::Black,
            Page::Settings,
            1440,
            1000,
            false,
            false,
            100,
        ),
        (
            "general-light",
            appearance::Theme::Light,
            Page::Settings,
            1080,
            720,
            true,
            true,
            90,
        ),
    ] {
        let ctx = egui::Context::default();
        let mut app = fixture(&ctx);
        app.page = page;
        app.settings_tab = if name == "general-light" {
            SettingsTab::General
        } else {
            SettingsTab::Appearance
        };
        app.config.display.theme = theme;
        app.config.display.compact_sidebar = compact;
        app.config.display.native_titlebar = native;
        app.config.display.zoom_percent = zoom;
        app.insights.cache.insert(
            spotify::insights::Range::FourWeeks,
            spotify::insights::Data {
                tracks: app.recent_tracks.clone(),
                artists: vec![spotify::catalog::Artist {
                    id: "test".into(),
                    name: "Preview artist".into(),
                    image_url: None,
                }],
            },
        );
        app.insights.attempted = true;
        render_app(&mut app, &ctx, width, height)
            .save(directory.join(format!("{name}.png")))
            .unwrap();
    }
}

#[test]
fn top_music_ignores_signed_out_response() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.token = None;
    app.insights.reset();
    let generation = app.insights.generation;
    app.tx
        .send(AppEvent::InsightsLoaded {
            generation,
            range: spotify::insights::Range::FourWeeks,
            result: Ok(spotify::insights::Data::default()),
            token: Some(TokenState {
                access_token: "stale".into(),
                refresh_token: None,
                expires_at: Instant::now() + Duration::from_secs(600),
            }),
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.insights.cache.is_empty() && app.token.is_none());
}

#[test]
fn collection_membership_survives_picker_track_changes_and_rejects_old_accounts() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let track = real_track("existing");
    app.playlist_membership
        .insert("owned".into(), HashSet::from([track.uri.clone()]));
    app.open_playlist_picker(track.clone());
    assert!(app.playlist_picker_added.contains("owned"));
    app.open_playlist_picker(real_track("other"));
    assert!(app.playlist_picker_added.is_empty());
    assert!(app.known_memberships(&track.uri).contains("owned"));
    app.membership_generation = 3;
    app.tx
        .send(AppEvent::MembershipLoaded {
            generation: 2,
            playlist_id: "stale".into(),
            uris: Ok(HashSet::from([track.uri.clone()])),
            token: None,
            done: true,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(!app.playlist_membership.contains_key("stale"));
}

#[test]
#[ignore = "Manual offscreen dialog review"]
fn render_create_playlist_preview() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.show_create_playlist = true;
    render_app(&mut app, &ctx, 1080, 720)
        .save("target/ui-previews/create-playlist-centered.png")
        .unwrap();
}

#[test]
fn playlist_removal_commits_only_after_confirmation_and_keeps_failed_removals_retryable() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let track = real_track("Remove123");
    let id = app.playlists[0].id.clone();
    app.open_playlist_picker(track.clone());
    app.playlist_picker_added.insert(id.clone());
    app.playlist_membership
        .insert(id.clone(), HashSet::from([track.uri.clone()]));
    app.playlist_remove_target = Some((id.clone(), track.uri.clone()));
    app.playlist_add_pending = Some(id.clone());
    app.tx
        .send(AppEvent::ActionFinished {
            action: format!("playlist_remove:{id}:{}", track.id),
            result: Err("Denied".into()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(app.playlist_picker_added.contains(&id));
    assert!(app.playlist_add_pending.is_none());
    assert!(app.playlist_add_error.is_some());
    app.playlist_remove_target = Some((id.clone(), track.uri.clone()));
    app.playlist_add_pending = Some(id.clone());
    app.tx
        .send(AppEvent::ActionFinished {
            action: format!("playlist_remove:{id}:{}", track.id),
            result: Ok(()),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(!app.playlist_picker_added.contains(&id));
    assert!(app.known_memberships(&track.uri).is_empty());
    assert!(app.show_add_to_playlist);
    let foreign = Playlist {
        id: "foreign".into(),
        owner_id: "someone-else".into(),
        ..Default::default()
    };
    app.remove_track_from_playlist(foreign, track, &ctx);
    assert!(app.playlist_add_pending.is_none());
    assert!(app.playlist_add_error.is_some());
}

#[test]
#[ignore = "Repaint diagnostic"]
fn diagnose_idle_repaint_requests() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.config.reduce_motion = true;
    app.config.smooth_progress = false;
    app.page = Page::Home;
    app.insights.attempted = true;
    app.insights.cache.insert(
        spotify::insights::Range::FourWeeks,
        spotify::insights::Data::default(),
    );
    for pass in 0..12 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440., 900.))),
                ..Default::default()
            },
            |ctx| draw(&mut app, ctx),
        );
        println!("pass {pass}: {:?}", ctx.repaint_causes());
    }
}

#[test]
fn navigation_settles_without_an_immediate_repaint_loop() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.token = None;
    app.config.reduce_motion = true;
    app.config.smooth_progress = false;
    ctx.style_mut(|style| style.animation_time = 0.0);
    for _ in 0..3 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    ui_click(&mut app, &ctx, Pos2::new(90.0, 228.0));
    for _ in 0..30 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    assert!(
        !ctx.requested_repaint_last_pass(),
        "navigation left a continuous repaint request: {:?}",
        ctx.repaint_causes()
    );
}

#[test]
fn custom_connect_device_name_sanitizes_and_drives_local_output() {
    assert_eq!(AppConfig::sanitize_device_name("  \n "), "Aurafy");
    assert_eq!(
        AppConfig::sanitize_device_name("  Living Room PC\x07 "),
        "Living Room PC"
    );
    let ctx = egui::Context::default();
    let (mut app, _) = local_fixture(&ctx);
    app.config.connect_device_name = "Studio PC".into();
    app.local_status.paired = true;
    app.local_status.active = true;
    assert_eq!(app.selected_output_name(), Some("Studio PC"));
    let devices = app.playback_devices();
    assert_eq!(devices[0].id, "auralis-local");
    assert_eq!(devices[0].name, "Studio PC");
}

#[test]
fn smtc_play_and_pause_actions_are_idempotent() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.local_status.active = true;
    app.is_playing = false;
    app.handle_desktop_action(desktop::Action::Pause, &ctx);
    assert!(
        rx.try_recv().is_err(),
        "Pause while paused should be a no-op"
    );
    app.handle_desktop_action(desktop::Action::Play, &ctx);
    let job = rx.try_recv().expect("Play while paused should send play");
    assert_eq!(job.action, "resume");
    app.is_playing = true;
    app.handle_desktop_action(desktop::Action::Play, &ctx);
    assert!(
        rx.try_recv().is_err(),
        "Play while playing should be a no-op"
    );
    app.handle_desktop_action(desktop::Action::Pause, &ctx);
    let job = rx
        .try_recv()
        .expect("Pause while playing should send pause");
    assert_eq!(job.action, "pause");
}

#[test]
fn header_back_and_forward_navigation_retraces_page_history() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.page = Page::Home;
    app.nav_history.clear();
    app.nav_forward.clear();
    assert!(!app.can_go_back());
    assert!(!app.can_go_forward());

    app.record_nav();
    app.page = Page::Search;
    assert!(app.can_go_back());
    assert!(!app.can_go_forward());

    app.record_nav();
    app.page = Page::Library;
    assert!(app.can_go_back());

    app.go_back(&ctx);
    assert!(app.page == Page::Search);
    assert!(app.can_go_back());
    assert!(app.can_go_forward());

    app.go_back(&ctx);
    assert!(app.page == Page::Home);
    assert!(!app.can_go_back());
    assert!(app.can_go_forward());

    app.go_forward(&ctx);
    assert!(app.page == Page::Search);
    assert!(app.can_go_forward());

    // Branching to Liked from Search drops stale Library forward history.
    app.record_nav();
    app.page = Page::Liked;
    assert!(app.can_go_back());
    assert!(!app.can_go_forward());
}

#[test]
fn search_catalog_filter_pills_and_clear_reset_state() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.token = None;
    app.config.client_id.clear();
    app.config.local_playback = false;
    app.page = Page::Search;
    app.search_query = "Kanye".into();
    app.search_filter = SearchFilter::Songs;
    app.run_search("Kanye".into(), &ctx);

    assert!(!app.search_results.is_empty());
    let catalog = app
        .search_catalog
        .as_ref()
        .expect("demo search should populate search_catalog");
    assert_eq!(catalog.tracks.len(), app.search_results.len());
    assert!(app.search_filter == SearchFilter::Songs);

    app.search_filter = SearchFilter::Albums;
    let _ = render_app(&mut app, &ctx, 1440, 900);
    assert!(app.search_filter == SearchFilter::Albums);

    app.search_query.clear();
    app.clear_search();
    assert!(app.search_query.is_empty());
    assert!(app.search_results.is_empty());
    assert!(app.search_catalog.is_none());
}

#[test]
fn circular_artist_previews_render_without_clipping() {
    // Circular artist avatar fallback on Search (no artwork_url) should render cleanly.
    let ctx2 = egui::Context::default();
    let mut app2 = fixture(&ctx2);
    app2.page = Page::Search;
    app2.search_query = "Weeknd".into();
    app2.search_results = app2.saved_tracks.iter().take(2).cloned().collect();
    app2.search_catalog = Some(spotify::catalog::SearchCatalog {
        tracks: app2.search_results.clone(),
        artists: vec![spotify::catalog::Artist {
            id: "a1".into(),
            name: "The Weeknd".into(),
            image_url: None,
        }],
        albums: vec![],
        playlists: vec![],
    });
    let search_img = render_app(&mut app2, &ctx2, 1440, 900);
    assert_eq!(search_img.dimensions(), (1440, 900));
}

fn green_header_play_center(app: &mut AurafyApp, ctx: &egui::Context) -> Pos2 {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            ..Default::default()
        },
        |ctx| draw(app, ctx),
    );
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Circle(circle)
                if circle.fill == GREEN && circle.radius >= 20.0 && circle.center.y < 600.0 =>
            {
                Some(circle.center)
            }
            _ => None,
        })
        .expect("Missing green header play/pause button")
}

#[test]
fn artist_and_album_header_play_pauses_when_current_track_is_playing() {
    for on_album in [false, true] {
        let ctx = egui::Context::default();
        let (mut app, rx) = local_fixture(&ctx);
        app.local_status.active = true;
        let t1 = real_track("h1");
        let t2 = real_track("h2");
        if on_album {
            app.recent_tracks.push(real_track("rec1"));
            let overview = catalogue_fixture(&mut app);
            app.page = Page::Album;
            app.browse.album = Some(spotify::catalog::AlbumDetails {
                album: overview.albums.items[0].clone(),
                tracks: vec![t1.clone(), t2],
            });
        } else {
            app.page = Page::Artist;
            app.artist_name = "Test Artist".into();
            app.browse.artist_id = "art1".into();
            app.artist_tracks = vec![t1.clone(), t2];
        }
        app.current_track = t1;
        app.is_playing = true;
        for _ in 0..2 {
            ui_frame(&mut app, &ctx, vec![]);
        }
        let pos = green_header_play_center(&mut app, &ctx);
        ui_click(&mut app, &ctx, pos);
        let jobs: Vec<_> = rx.try_iter().collect();
        assert!(
            jobs.iter().any(|job| job.action == "pause"),
            "expected pause job on {} header, got {:?}",
            if on_album { "album" } else { "artist" },
            jobs.iter().map(|j| j.action.as_str()).collect::<Vec<_>>()
        );
        assert!(
            !jobs.iter().any(|job| job.action == "play_track"),
            "must not restart track 0 when pausing"
        );
    }
}

#[test]
fn shuffle_queues_remaining_tracks_on_artist_album_and_playlist() {
    for mode in ["artist", "album", "playlist"] {
        let ctx = egui::Context::default();
        let (mut app, rx) = local_fixture(&ctx);
        app.local_status.active = true;
        let tracks = vec![
            real_track(&format!("{:0>22}", 101)),
            real_track(&format!("{:0>22}", 102)),
            real_track(&format!("{:0>22}", 103)),
        ];
        match mode {
            "artist" => {
                app.page = Page::Artist;
                app.artist_name = "Test Artist".into();
                app.browse.artist_id = "art1".into();
                app.artist_tracks = tracks;
            }
            "album" => {
                app.recent_tracks.push(real_track("rec1"));
                let overview = catalogue_fixture(&mut app);
                app.page = Page::Album;
                app.browse.album = Some(spotify::catalog::AlbumDetails {
                    album: overview.albums.items[0].clone(),
                    tracks,
                });
            }
            _ => {
                app.page = Page::Library;
                let pl = Playlist {
                    id: "pl1".into(),
                    name: "Mix".into(),
                    owner_id: "u1".into(),
                    owner: "User".into(),
                    total_items: 3,
                    ..Default::default()
                };
                app.playlists = vec![pl.clone()];
                app.selected_playlist = Some(pl);
                app.playlist_tracks = tracks;
            }
        }
        for _ in 0..2 {
            ui_frame(&mut app, &ctx, vec![]);
        }
        let pos = label_center(&mut app, &ctx, "Shuffle");
        ui_click(&mut app, &ctx, pos);
        let jobs: Vec<_> = rx.try_iter().collect();
        assert!(
            jobs.iter().any(|j| j.action == "play_track"),
            "{mode}: expected play_track job"
        );
        let queued = jobs
            .iter()
            .find(|j| j.action == "queue")
            .unwrap_or_else(|| panic!("{mode}: expected queue job for remaining shuffled tracks"));
        assert_eq!(queued.requests.len(), 2);
    }
}

#[test]
fn track_table_sorting_preserves_context_position_on_playback() {
    let ctx = egui::Context::default();
    let (mut app, _rx) = local_fixture(&ctx);
    app.token = Some(TokenState {
        access_token: "test-only".into(),
        refresh_token: None,
        expires_at: Instant::now() + Duration::from_secs(300),
    });
    app.local_status.active = true;
    app.page = Page::Library;
    let pl = Playlist {
        id: "pl_sort".into(),
        name: "Sortable".into(),
        owner_id: "u1".into(),
        owner: "User".into(),
        total_items: 3,
        items_accessible: true,
        ..Default::default()
    };
    app.playlists = vec![pl.clone()];
    app.selected_playlist = Some(pl);
    let mut t0 = real_track(&format!("{:0>22}", 200));
    t0.name = "Zebra".into();
    t0.duration_ms = 240_000;
    t0.context_position = Some(0);
    let mut t1 = real_track(&format!("{:0>22}", 201));
    t1.name = "Alpha".into();
    t1.duration_ms = 120_000;
    t1.context_position = Some(1);
    let expected_alpha_uri = t1.uri.clone();
    let mut t2 = real_track(&format!("{:0>22}", 202));
    t2.name = "Middle".into();
    t2.duration_ms = 180_000;
    t2.context_position = Some(2);
    app.playlist_tracks = vec![t0, t1, t2];

    for _ in 0..2 {
        ui_frame(&mut app, &ctx, vec![]);
    }
    let dur_pos = label_center(&mut app, &ctx, "DURATION");
    ui_click(&mut app, &ctx, dur_pos);
    assert_eq!(app.track_sort.column, crate::app::TrackSortColumn::Duration);
    assert!(app.track_sort.ascending);

    let alpha_pos = label_center(&mut app, &ctx, "Alpha");
    let middle_pos = label_center(&mut app, &ctx, "Middle");
    let zebra_pos = label_center(&mut app, &ctx, "Zebra");
    assert!(alpha_pos.y < middle_pos.y && middle_pos.y < zebra_pos.y);

    // Single-click the play column on the "Alpha" row (left of the title text).
    ui_click(&mut app, &ctx, Pos2::new(alpha_pos.x - 90.0, alpha_pos.y));
    assert_eq!(
        app.requested_track_uri.as_deref(),
        Some(expected_alpha_uri.as_str())
    );
    let ctx_pos = app.table_play_context(&app.playlist_tracks[1], 1).unwrap();
    assert_eq!(ctx_pos.uri, "spotify:playlist:pl_sort");
    assert_eq!(ctx_pos.position, 1);

    // Clicking "DURATION ↑" flips to descending.
    let dur_asc_pos = label_center(&mut app, &ctx, "DURATION ↑");
    ui_click(&mut app, &ctx, dur_asc_pos);
    assert!(!app.track_sort.ascending);
    let alpha_desc = label_center(&mut app, &ctx, "Alpha");
    let zebra_desc = label_center(&mut app, &ctx, "Zebra");
    assert!(zebra_desc.y < alpha_desc.y);

    // Clicking "#" resets sorting back to Default.
    let hash_pos = label_center(&mut app, &ctx, "#");
    ui_click(&mut app, &ctx, hash_pos);
    assert_eq!(app.track_sort.column, crate::app::TrackSortColumn::Default);
    let zebra_reset = label_center(&mut app, &ctx, "Zebra");
    let alpha_reset = label_center(&mut app, &ctx, "Alpha");
    let middle_reset = label_center(&mut app, &ctx, "Middle");
    assert!(zebra_reset.y < alpha_reset.y && alpha_reset.y < middle_reset.y);
}

#[test]
fn queue_header_shows_up_next_without_fixed_counts_and_sleep_timer() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    app.show_queue = true;
    app.queue = vec![real_track("q1"), real_track("q2")];
    app.sleep_timer = Some(crate::preferences::SleepTimer::Deadline(
        std::time::Instant::now() + std::time::Duration::from_secs(15 * 60),
    ));
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 900.0))),
            ..Default::default()
        },
        |ctx| draw(&mut app, ctx),
    );
    let texts: Vec<String> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        texts.iter().any(|t| t == "Up next"),
        "expected plain 'Up next' header without fixed counts in texts: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t.contains("6 min")),
        "queue header must not show fixed runtime: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("Sleep timer")),
        "expected active sleep timer badge in queue: {texts:?}"
    );
}

#[test]
fn artist_and_album_header_play_resumes_when_paused_on_current_track() {
    for on_album in [false, true] {
        let ctx = egui::Context::default();
        let (mut app, rx) = local_fixture(&ctx);
        app.local_status.active = true;
        let t1 = real_track(&format!("{:0>22}", 201));
        let t2 = real_track(&format!("{:0>22}", 202));
        if on_album {
            app.recent_tracks.push(real_track(&format!("{:0>22}", 203)));
            let overview = catalogue_fixture(&mut app);
            app.page = Page::Album;
            app.browse.album = Some(spotify::catalog::AlbumDetails {
                album: overview.albums.items[0].clone(),
                tracks: vec![t1.clone(), t2],
            });
        } else {
            app.page = Page::Artist;
            app.artist_name = "Test Artist".into();
            app.browse.artist_id = "art1".into();
            app.artist_tracks = vec![t1.clone(), t2];
        }
        app.current_track = t1;
        app.is_playing = false;
        for _ in 0..2 {
            ui_frame(&mut app, &ctx, vec![]);
        }
        let pos = green_header_play_center(&mut app, &ctx);
        ui_click(&mut app, &ctx, pos);
        let jobs: Vec<_> = rx.try_iter().collect();
        assert!(
            jobs.iter().any(|job| job.action == "resume"),
            "expected resume job on {} header, got {:?}",
            if on_album { "album" } else { "artist" },
            jobs.iter().map(|j| j.action.as_str()).collect::<Vec<_>>()
        );
        assert!(
            !jobs.iter().any(|job| job.action == "play_track"),
            "must not restart track 0 when resuming"
        );
    }
}

#[test]
fn play_tracks_shuffled_skips_unplayable_tracks() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.local_status.active = true;
    let mut unplayable = real_track("skip1");
    unplayable.uri.clear();
    let playable = real_track(&format!("{:0>22}", 301));
    let expected_uri = playable.uri.clone();
    app.play_tracks_shuffled(vec![unplayable.clone(), unplayable, playable], &ctx);
    let jobs: Vec<_> = rx.try_iter().collect();
    assert!(
        jobs.iter().any(|job| job.action == "play_track"
            && job.requests.iter().any(|r| r["uri"] == expected_uri)),
        "expected valid track to play even when unplayable rows exist: {:?}",
        jobs.iter()
            .map(|j| (&j.action, &j.requests))
            .collect::<Vec<_>>()
    );
}

#[test]
fn queued_or_suppressed_track_does_not_block_playlist_play_collection() {
    let ctx = egui::Context::default();
    let mut app = fixture(&ctx);
    let playlist = app.playlists[0].clone();
    let mut playlist_track = real_track(&format!("{:0>22}", 501));
    playlist_track.name = "Intro".into();
    let mut second_track = real_track(&format!("{:0>22}", 502));
    second_track.name = "Lucid Dreams".into();
    let mut third_track = real_track(&format!("{:0>22}", 503));
    third_track.name = "All Girls Are The Same".into();
    let mut queued_track = real_track(&format!("{:0>22}", 901));
    queued_track.name = "Heartless".into();

    app.selected_playlist = Some(playlist.clone());
    app.playlist_tracks = vec![
        playlist_track.clone(),
        second_track.clone(),
        third_track.clone(),
    ];
    app.playback_context_uri = Some(format!("spotify:playlist:{}", playlist.id));
    app.is_playing = true;
    app.current_track = queued_track.clone();
    app.queue = vec![queued_track.clone()];

    let (player, rx) = local_player::Player::test_channel();
    app.local_player = Some(player);
    app.local_status = local_player::Status {
        running: true,
        paired: true,
        active: true,
        ..Default::default()
    };

    assert!(
        !app.is_playlist_actively_playing(&playlist.id),
        "manually queued track outside playlist must not show playlist as actively playing"
    );
    app.current_track = playlist_track.clone();
    assert!(
        !app.is_playlist_actively_playing(&playlist.id),
        "polluted Up next[0] outside playlist must allow Play to restart playlist straight"
    );
    app.play_playlist(&playlist, &ctx);
    let job = rx
        .try_recv()
        .expect("must dispatch local play for playlist track 1 when queue has stuck tracks");
    assert_eq!(job.action, "play_track");
    assert_eq!(
        app.requested_track_uri.as_deref(),
        Some(playlist_track.uri.as_str())
    );
    assert!(app.queue.is_empty(), "stuck upstream queue must be cleared");
    assert!(app.suppress_upstream_queue);
    assert_eq!(
        app.priority_queue,
        vec![second_track.clone(), third_track.clone()]
    );
    assert!(app.is_playlist_actively_playing(&playlist.id));

    app.active_collection_uri = None;
    app.suppress_upstream_queue = false;
    app.playback_context_uri = None;
    app.priority_queue.clear();
    app.play_playlist(&playlist, &ctx);
    let fresh_job = rx.try_recv().expect(
        "local playlist Play must start track 1 and priority_queue even when queue starts empty",
    );
    assert_eq!(fresh_job.action, "play_track");
    assert!(app.suppress_upstream_queue);
    assert_eq!(
        app.priority_queue,
        vec![second_track.clone(), third_track.clone()]
    );

    app.send_playback_action("next", &ctx);
    let next_job = rx.try_recv().expect("Next must advance to Lucid Dreams");
    assert_eq!(next_job.action, "play_track");
    assert_eq!(
        app.requested_track_uri.as_deref(),
        Some(second_track.uri.as_str())
    );
    assert_eq!(app.priority_queue, vec![third_track.clone()]);
    assert!(app.is_playlist_actively_playing(&playlist.id));

    let tracks = app.playlist_tracks.clone();
    app.play_tracks_shuffled(tracks, &ctx);
    let shuffle_job = rx
        .try_recv()
        .expect("Shuffle must start shuffled track locally");
    assert_eq!(shuffle_job.action, "play_track");
    assert_eq!(app.priority_queue.len(), 2);
    assert!(app.queue.is_empty());
    assert!(
        rx.try_recv().is_err(),
        "Shuffle must not append to stuck Spotify queue"
    );
}

#[test]
fn unexpected_soloist_track_change_advances_priority_queue_or_stops_at_end() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.local_status.active = true;
    app.suppress_upstream_queue = true;
    let playlist_uri = "spotify:playlist:pl_guard".to_owned();
    app.active_collection_uri = Some(playlist_uri.clone());

    let mut current = real_track(&format!("{:0>22}", 601));
    current.name = "All Girls Are The Same".into();
    let mut next_pl = real_track(&format!("{:0>22}", 602));
    next_pl.name = "Rich And Blind".into();
    let mut third_pl = real_track(&format!("{:0>22}", 603));
    third_pl.name = "Legends".into();
    let mut stuck = real_track(&format!("{:0>22}", 991));
    stuck.name = "Empire State Of Mind".into();
    let mut stuck_two = real_track(&format!("{:0>22}", 992));
    stuck_two.name = "High Energy".into();

    app.current_track = current.clone();
    app.duration_ms = 200_000;
    app.progress_ms = 199_000;
    app.progress_anchor = Instant::now();
    app.is_playing = true;
    app.priority_auto_armed = true;
    app.priority_queue = vec![next_pl.clone(), third_pl.clone()];
    app.requested_track_uri = None;
    app.restoring_playback = false;

    // Even during a buffering snapshot with a different context URI from Spotify's manual queue,
    // end-of-track transitions must immediately jump to Rich And Blind.
    app.handle_local_event_with_ctx(
        local_player::Event::Playback(PlaybackState {
            track: Some(stuck.clone()),
            context_uri: Some("spotify:playlist:unrelated_queue_context".into()),
            is_playing: false,
            is_idle: false,
            progress_ms: 0,
            device_name: Some(local_player::DEVICE_NAME.into()),
            ..Default::default()
        }),
        Some(&ctx),
    );
    let advance = rx
        .try_recv()
        .expect("unexpected Soloist advance into stuck queue must jump straight to Rich And Blind");
    assert_eq!(advance.action, "play_track");
    assert_eq!(
        advance.requests[0]["uri"],
        serde_json::json!(next_pl.uri.clone())
    );
    assert_eq!(app.priority_queue, vec![third_pl.clone()]);
    assert_eq!(
        app.active_collection_uri.as_deref(),
        Some(playlist_uri.as_str())
    );
    assert_ne!(app.current_track.uri, stuck.uri);

    // Follow-up stale state snapshot for the stuck track while Rich And Blind loads must be ignored.
    app.handle_local_event_with_ctx(
        local_player::Event::Playback(PlaybackState {
            track: Some(stuck.clone()),
            is_playing: true,
            progress_ms: 120,
            device_name: Some(local_player::DEVICE_NAME.into()),
            ..Default::default()
        }),
        Some(&ctx),
    );
    assert_ne!(app.current_track.uri, stuck.uri);

    // Confirm Rich And Blind starts cleanly, then test end-of-queue interception when priority_queue is empty.
    app.handle_local_event_with_ctx(
        local_player::Event::Playback(PlaybackState {
            track: Some(next_pl.clone()),
            is_playing: true,
            progress_ms: 0,
            device_name: Some(local_player::DEVICE_NAME.into()),
            ..Default::default()
        }),
        Some(&ctx),
    );
    assert!(app.requested_track_uri.is_none());
    assert_eq!(app.current_track.uri, next_pl.uri);
    app.priority_queue.clear();

    app.handle_local_event_with_ctx(
        local_player::Event::Playback(PlaybackState {
            track: Some(stuck_two.clone()),
            is_playing: true,
            progress_ms: 0,
            device_name: Some(local_player::DEVICE_NAME.into()),
            ..Default::default()
        }),
        Some(&ctx),
    );
    let stop_job = rx
        .try_recv()
        .expect("unexpected advance after priority_queue empties must pause Soloist");
    assert_eq!(stop_job.action, "pause");
    assert!(!app.is_playing);
    assert_eq!(app.status_message, "Reached end of queue");
    assert_eq!(app.current_track.uri, next_pl.uri);
}

#[test]
fn search_play_keeps_manual_entries_and_restores_spotify_queue() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.local_status.active = true;
    app.token = Some(TokenState {
        access_token: "test-only".into(),
        refresh_token: None,
        expires_at: Instant::now() + Duration::from_secs(300),
    });
    let playlist = Playlist {
        id: "pl_q".into(),
        name: "Queue Mix".into(),
        ..Default::default()
    };
    app.playlists = vec![playlist.clone()];
    let mut pl1 = real_track(&format!("{:0>22}", 701));
    pl1.name = "Intro".into();
    let mut pl2 = real_track(&format!("{:0>22}", 702));
    pl2.name = "Lucid Dreams".into();
    let mut pl3 = real_track(&format!("{:0>22}", 703));
    pl3.name = "All Girls Are The Same".into();

    let mut q1 = real_track(&format!("{:0>22}", 801));
    q1.name = "What I've Done".into();
    let mut q2 = real_track(&format!("{:0>22}", 802));
    q2.name = "Numb".into();
    let auto1 = real_track(&format!("{:0>22}", 850));
    let auto2 = real_track(&format!("{:0>22}", 851));

    app.selected_playlist = Some(playlist.clone());
    app.playlist_tracks = vec![pl1.clone(), pl2.clone(), pl3.clone()];
    app.play_playlist(&playlist, &ctx);
    let _ = rx.try_recv().unwrap();
    assert_eq!(app.priority_queue, vec![pl2.clone(), pl3.clone()]);
    assert_eq!(app.manual_queue_len, 0);

    // Adding from search while playlist is playing puts tracks at the TOP (before pl2, pl3) in FIFO order.
    app.add_to_queue(q1.clone(), &ctx);
    app.add_to_queue(q2.clone(), &ctx);
    assert_eq!(app.manual_queue_len, 2);
    assert_eq!(
        app.priority_queue,
        vec![q1.clone(), q2.clone(), pl2.clone(), pl3.clone()]
    );

    // Starting a single song keeps manual additions and returns control
    // of the rest of Up next to Spotify.
    let mut search_song = real_track(&format!("{:0>22}", 899));
    search_song.name = "Faint".into();
    app.play_track(search_song.clone(), &ctx);
    let _ = rx.try_recv().unwrap();
    assert_eq!(app.priority_queue, vec![q1.clone(), q2.clone()]);
    assert_eq!(app.manual_queue_len, 2);
    assert!(!app.suppress_upstream_queue);
    assert!(app.active_collection_uri.is_none());
    assert!(app.queue.is_empty());

    let spotify_queue = vec![auto2, auto1.clone(), auto1];
    app.handle_local_event_with_ctx(
        local_player::Event::Queue(spotify_queue.clone()),
        Some(&ctx),
    );
    assert_eq!(app.queue, spotify_queue);
    assert_eq!(app.priority_queue, vec![q1, q2]);
    assert!(
        rx.try_recv().is_err(),
        "reading the queue must not send a playback command"
    );

    let _ = app.clear_up_next_queue();
    assert!(app.suppress_upstream_queue);
    assert!(app.queue.is_empty());
    assert_eq!(app.manual_queue_len, 0);

    app.play_track(search_song.clone(), &ctx);
    assert_eq!(rx.try_recv().unwrap().action, "play_track");
    assert!(!app.suppress_upstream_queue);
    let next_from_spotify = spotify_queue[0].clone();
    for (track, progress_ms) in [(search_song, 199_000), (next_from_spotify.clone(), 0)] {
        app.handle_local_event_with_ctx(
            local_player::Event::Playback(PlaybackState {
                track: Some(track),
                progress_ms,
                is_playing: true,
                ..Default::default()
            }),
            Some(&ctx),
        );
    }
    assert_eq!(app.current_track.uri, next_from_spotify.uri);
    assert!(app.is_playing);
    assert!(
        rx.try_recv().is_err(),
        "Spotify advances without an Aurafy override"
    );
}

#[test]
fn autoplay_continues_after_queue_empties_unless_cleared() {
    let ctx = egui::Context::default();
    let (mut app, rx) = local_fixture(&ctx);
    app.local_status.active = true;
    app.token = Some(TokenState {
        access_token: "test-token".into(),
        refresh_token: None,
        expires_at: Instant::now() + Duration::from_secs(3600),
    });

    let mut first = real_track(&format!("{:0>22}", 910));
    first.name = "Lovers Rock".into();
    let mut auto1 = real_track(&format!("{:0>22}", 911));
    auto1.name = "Afraid".into();
    let mut auto2 = real_track(&format!("{:0>22}", 912));
    auto2.name = "CHIHIRO".into();

    app.current_track = first;
    app.duration_ms = 200_000;
    app.progress_ms = 199_800;
    app.is_playing = true;
    app.suppress_upstream_queue = true;

    app.handle_suppressed_queue_end(None, &ctx);
    assert!(app.autoplay_pending);
    assert!(rx.try_recv().is_err());

    app.tx
        .send(AppEvent::AutoplayLoaded {
            generation: app.autoplay_generation,
            result: Ok(vec![auto1.clone(), auto2.clone()]),
            token: None,
        })
        .unwrap();
    app.handle_events(&ctx);
    assert!(!app.autoplay_pending);
    let play = rx.try_recv().expect("autoplay should dispatch first track");
    assert_eq!(play.action, "play_track");
    assert_eq!(play.requests[0]["uri"], serde_json::json!(auto1.uri));
    assert_eq!(app.priority_queue, vec![auto2]);

    let _ = app.clear_up_next_queue();
    assert!(app.queue_cleared_by_user);
    app.handle_suppressed_queue_end(None, &ctx);
    let stop = rx.try_recv().expect("cleared queue should pause at end");
    assert_eq!(stop.action, "pause");
    assert!(!app.is_playing);
}

fn shapes_text_rects(shapes: &[egui::epaint::ClippedShape], target: &str) -> Vec<Rect> {
    let mut out = Vec::new();
    for clipped in shapes {
        collect_shape_text(&clipped.shape, target, &mut out);
    }
    out
}

fn collect_shape_text(shape: &egui::Shape, target: &str, out: &mut Vec<Rect>) {
    match shape {
        egui::Shape::Text(text) => {
            if text.galley.text() == target {
                out.push(text.visual_bounding_rect());
            }
        }
        egui::Shape::Vec(vec) => {
            for inner in vec {
                collect_shape_text(inner, target, out);
            }
        }
        _ => {}
    }
}
