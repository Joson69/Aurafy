mod chrome;
mod player;
mod settings;
mod views;

use super::*;
use egui::{StrokeKind, epaint::PathShape, text::LayoutJob};

#[derive(Clone, Copy)]
pub(crate) enum Icon {
    Home,
    Search,
    Library,
    Heart,
    Clock,
    User,
    Settings,
    Play,
    Pause,
    Next,
    Previous,
    Shuffle,
    Repeat,
    Queue,
    Lyrics,
    Volume,
    Mute,
    Device,
    Phone,
    Close,
    Refresh,
    Plus,
    More,
    Check,
    CirclePlus,
    CircleCheck,
    Fullscreen,
    Minimize,
    Maximize,
    Restore,
}

pub(crate) fn configure(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // egui does not automatically use Windows font fallback. Load CJK support
    // once at startup, after the existing UI fonts, without redistributing OS fonts.
    #[cfg(target_os = "windows")]
    if let Some(root) = std::env::var_os("WINDIR").map(std::path::PathBuf::from) {
        let candidates = [
            ("aurafy-korean-fallback", ["Fonts/malgun.ttf"].as_slice()),
            (
                "aurafy-japanese-fallback",
                ["Fonts/YuGothR.ttc", "Fonts/msgothic.ttc"].as_slice(),
            ),
        ];
        for (name, paths) in candidates {
            if let Some(data) = paths
                .iter()
                .find_map(|rel| std::fs::read(root.join(rel)).ok())
            {
                let font_name = name.to_string();
                fonts
                    .font_data
                    .insert(font_name.clone(), egui::FontData::from_owned(data).into());
                for family in fonts.families.values_mut() {
                    family.push(font_name.clone());
                }
            }
        }
    }
    let fallback = fonts.families[&egui::FontFamily::Proportional].clone();
    for (name, data) in [
        (
            "aurafy-regular",
            include_bytes!("../../assets/fonts/Lato-Regular.ttf").as_slice(),
        ),
        (
            "aurafy-bold",
            include_bytes!("../../assets/fonts/Lato-Bold.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), egui::FontData::from_static(data).into());
        let mut family = vec![name.to_string()];
        family.extend(fallback.clone());
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), family);
    }
    ctx.set_fonts(fonts);
    apply_style(ctx);
}

pub(crate) fn apply_style(ctx: &egui::Context) {
    let colours = appearance::palette(ctx);
    let mut style = (*ctx.style()).clone();
    style.visuals = if colours.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    style.visuals.panel_fill = colours.bg;
    style.visuals.window_fill = colours.panel2;
    style.visuals.window_stroke = Stroke::new(1.0_f32, colours.border);
    style.visuals.extreme_bg_color = colours.panel;
    style.visuals.faint_bg_color = colours.panel2;
    style.visuals.widgets.noninteractive.bg_fill = colours.panel2;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.noninteractive.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.inactive.bg_fill = colours.hover;
    style.visuals.widgets.inactive.weak_bg_fill = colours.panel2;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.hovered.bg_fill = colours.selected;
    style.visuals.widgets.hovered.weak_bg_fill = colours.hover;
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.corner_radius = CornerRadius::same(10);
    style.visuals.widgets.active.bg_fill = colours.selected;
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, colours.border);
    style.visuals.widgets.active.corner_radius = CornerRadius::same(10);
    style.visuals.selection.bg_fill = colours.selected;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, colours.border);
    style.visuals.override_text_color = Some(colours.text);
    style.visuals.window_corner_radius = CornerRadius::same(12);
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 8.0);
    style.spacing.interact_size.y = 32.0;
    style.spacing.slider_width = 180.0;
    style.animation_time = 0.10;
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, FontId::proportional(11.0));
    ctx.set_style(style);
}

fn player_font(size: f32, bold: bool) -> FontId {
    FontId::new(
        size,
        egui::FontFamily::Name(
            if bold {
                "aurafy-bold"
            } else {
                "aurafy-regular"
            }
            .into(),
        ),
    )
}

// Icons are geometry, so they never depend on a font containing a music glyph.
fn icon(p: &egui::Painter, rect: Rect, kind: Icon, color: Color32) {
    let r = Rect::from_center_size(rect.center(), Vec2::splat(18.0_f32.min(rect.width() - 4.0)));
    let at = |x: f32, y: f32| Pos2::new(r.left() + x * r.width(), r.top() + y * r.height());
    let s = Stroke::new(1.6_f32, color);
    let line = |a, b| {
        p.line_segment([a, b], s);
    };
    let path = |v: Vec<Pos2>, closed| {
        p.add(PathShape {
            points: v,
            closed,
            fill: Color32::TRANSPARENT,
            stroke: s.into(),
        });
    };
    match kind {
        Icon::CirclePlus => {
            p.circle_stroke(r.center(), r.width() * 0.38, s);
            line(at(0.32, 0.5), at(0.68, 0.5));
            line(at(0.5, 0.32), at(0.5, 0.68));
        }
        Icon::CircleCheck => {
            let foreground = appearance::palette(p.ctx()).button_foreground;
            p.circle_filled(r.center(), r.width() * 0.40, color);
            p.line_segment(
                [at(0.28, 0.49), at(0.44, 0.64)],
                Stroke::new(1.5_f32, foreground),
            );
            p.line_segment(
                [at(0.44, 0.64), at(0.73, 0.34)],
                Stroke::new(1.5_f32, foreground),
            );
        }
        Icon::Fullscreen => {
            for (x, y, dx, dy) in [
                (0.12, 0.12, 1.0, 1.0),
                (0.88, 0.12, -1.0, 1.0),
                (0.12, 0.88, 1.0, -1.0),
                (0.88, 0.88, -1.0, -1.0),
            ] {
                path(
                    vec![at(x + dx * 0.24, y), at(x, y), at(x, y + dy * 0.24)],
                    false,
                );
            }
        }
        Icon::Minimize => {
            line(at(0.2, 0.5), at(0.8, 0.5));
        }
        Icon::Maximize => {
            p.rect_stroke(
                Rect::from_min_max(at(0.2, 0.2), at(0.8, 0.8)),
                1.0,
                s,
                StrokeKind::Inside,
            );
        }
        Icon::Restore => {
            path(vec![at(0.35, 0.2), at(0.8, 0.2), at(0.8, 0.65)], false);
            p.rect_stroke(
                Rect::from_min_max(at(0.18, 0.35), at(0.65, 0.82)),
                1.0,
                s,
                StrokeKind::Inside,
            );
        }
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![at(0.28, 0.12), at(0.87, 0.5), at(0.28, 0.88)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            for x in [0.23, 0.62] {
                p.rect_filled(
                    Rect::from_min_max(at(x, 0.14), at(x + 0.15, 0.86)),
                    1.0,
                    color,
                );
            }
        }
        Icon::Next | Icon::Previous => {
            let x: f32 = if matches!(kind, Icon::Next) {
                1.0
            } else {
                -1.0
            };
            let pos = |a: f32, b: f32| at(0.5 + (a - 0.5) * x, b);
            p.add(egui::Shape::convex_polygon(
                vec![pos(0.14, 0.16), pos(0.72, 0.5), pos(0.14, 0.84)],
                color,
                Stroke::NONE,
            ));
            line(pos(0.84, 0.16), pos(0.84, 0.84));
        }
        Icon::Search => {
            p.circle_stroke(at(0.43, 0.42), r.width() * 0.28, s);
            line(at(0.65, 0.65), at(0.94, 0.94));
        }
        Icon::Home => {
            path(vec![at(0.08, 0.45), at(0.5, 0.08), at(0.92, 0.45)], false);
            path(
                vec![
                    at(0.2, 0.4),
                    at(0.2, 0.9),
                    at(0.42, 0.9),
                    at(0.42, 0.62),
                    at(0.62, 0.62),
                    at(0.62, 0.9),
                    at(0.82, 0.9),
                    at(0.82, 0.4),
                ],
                false,
            );
        }
        Icon::Library => {
            for x in [0.12, 0.36] {
                p.rect_stroke(
                    Rect::from_min_max(at(x, 0.15), at(x + 0.14, 0.9)),
                    1.0,
                    s,
                    StrokeKind::Inside,
                );
            }
            line(at(0.65, 0.12), at(0.9, 0.85));
            line(at(0.77, 0.1), at(1.0, 0.81));
        }
        Icon::Heart => {
            path(
                vec![
                    at(0.5, 0.88),
                    at(0.1, 0.5),
                    at(0.08, 0.3),
                    at(0.23, 0.16),
                    at(0.39, 0.17),
                    at(0.5, 0.3),
                    at(0.61, 0.17),
                    at(0.77, 0.16),
                    at(0.92, 0.3),
                    at(0.9, 0.5),
                ],
                true,
            );
        }
        Icon::Clock => {
            p.circle_stroke(r.center(), r.width() * 0.42, s);
            path(vec![at(0.5, 0.24), at(0.5, 0.51), at(0.72, 0.63)], false);
        }
        Icon::User => {
            p.circle_stroke(at(0.5, 0.3), r.width() * 0.19, s);
            path(
                vec![
                    at(0.13, 0.92),
                    at(0.17, 0.72),
                    at(0.32, 0.61),
                    at(0.68, 0.61),
                    at(0.83, 0.72),
                    at(0.87, 0.92),
                ],
                false,
            );
        }
        Icon::Settings => {
            p.circle_stroke(r.center(), r.width() * 0.29, s);
            p.circle_stroke(r.center(), r.width() * 0.11, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                let v = Vec2::angled(a);
                line(
                    r.center() + v * r.width() * 0.3,
                    r.center() + v * r.width() * 0.44,
                );
            }
        }
        Icon::Queue => {
            for y in [0.22, 0.48, 0.74] {
                line(at(0.12, y), at(0.76, y));
            }
            p.add(egui::Shape::convex_polygon(
                vec![at(0.8, 0.59), at(1.0, 0.75), at(0.8, 0.91)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Lyrics => {
            p.rect_stroke(
                Rect::from_min_max(at(0.32, 0.05), at(0.68, 0.6)),
                4.0,
                s,
                StrokeKind::Inside,
            );
            path(
                vec![
                    at(0.17, 0.43),
                    at(0.18, 0.62),
                    at(0.29, 0.76),
                    at(0.5, 0.81),
                    at(0.71, 0.76),
                    at(0.82, 0.62),
                    at(0.83, 0.43),
                ],
                false,
            );
            line(at(0.5, 0.81), at(0.5, 0.98));
            line(at(0.32, 0.98), at(0.68, 0.98));
        }
        Icon::Mute => {
            path(
                vec![
                    at(0.08, 0.35),
                    at(0.28, 0.35),
                    at(0.52, 0.13),
                    at(0.52, 0.87),
                    at(0.28, 0.65),
                    at(0.08, 0.65),
                ],
                true,
            );
            line(at(0.7, 0.35), at(0.98, 0.65));
            line(at(0.7, 0.65), at(0.98, 0.35));
        }
        Icon::Volume => {
            path(
                vec![
                    at(0.08, 0.36),
                    at(0.29, 0.36),
                    at(0.5, 0.15),
                    at(0.5, 0.85),
                    at(0.29, 0.65),
                    at(0.08, 0.65),
                ],
                true,
            );
            path(vec![at(0.66, 0.3), at(0.73, 0.5), at(0.66, 0.7)], false);
            path(vec![at(0.84, 0.15), at(0.94, 0.5), at(0.84, 0.85)], false);
        }
        Icon::Device => {
            p.rect_stroke(
                Rect::from_min_max(at(0.06, 0.12), at(0.94, 0.7)),
                2.0,
                s,
                StrokeKind::Inside,
            );
            line(at(0.5, 0.72), at(0.5, 0.9));
            line(at(0.28, 0.92), at(0.72, 0.92));
        }
        Icon::Phone => {
            p.rect_stroke(
                Rect::from_min_max(at(0.28, 0.08), at(0.72, 0.92)),
                2.0,
                s,
                StrokeKind::Inside,
            );
            p.circle_filled(at(0.5, 0.79), 1.0, color);
        }
        Icon::Close => {
            line(at(0.25, 0.25), at(0.75, 0.75));
            line(at(0.75, 0.25), at(0.25, 0.75));
        }
        Icon::Plus => {
            line(at(0.5, 0.15), at(0.5, 0.85));
            line(at(0.15, 0.5), at(0.85, 0.5));
        }
        Icon::Check => {
            path(vec![at(0.16, 0.5), at(0.4, 0.76), at(0.86, 0.22)], false);
        }
        Icon::More => {
            for x in [0.2, 0.5, 0.8] {
                p.circle_filled(at(x, 0.5), 1.6, color);
            }
        }
        Icon::Shuffle => {
            path(
                vec![at(0.08, 0.2), at(0.27, 0.2), at(0.72, 0.8), at(0.94, 0.8)],
                false,
            );
            path(
                vec![at(0.08, 0.8), at(0.27, 0.8), at(0.72, 0.2), at(0.94, 0.2)],
                false,
            );
            path(vec![at(0.78, 0.06), at(0.94, 0.2), at(0.78, 0.34)], false);
            path(vec![at(0.78, 0.66), at(0.94, 0.8), at(0.78, 0.94)], false);
        }
        Icon::Repeat | Icon::Refresh => {
            path(
                vec![
                    at(0.12, 0.47),
                    at(0.12, 0.22),
                    at(0.82, 0.22),
                    at(0.7, 0.08),
                ],
                false,
            );
            path(
                vec![
                    at(0.88, 0.53),
                    at(0.88, 0.78),
                    at(0.18, 0.78),
                    at(0.3, 0.92),
                ],
                false,
            );
        }
    }
}

fn icon_at(ui: &Ui, rect: Rect, kind: Icon, color: Color32) {
    icon(ui.painter(), rect, kind, color);
}
pub(crate) fn icon_button(ui: &mut Ui, kind: Icon, label: &str, active: bool) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 8.0, colours.hover);
    }
    icon_at(
        ui,
        rect,
        kind,
        if active {
            colours.green
        } else if response.hovered() {
            colours.text
        } else {
            colours.muted
        },
    );
    response
        .clone()
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_text(label)
}
pub(crate) fn more_menu_button(
    ui: &mut Ui,
    label: &str,
    close_on_click: bool,
    add_contents: impl FnOnce(&mut Ui),
) {
    let ctx = ui.ctx().clone();
    let colours = appearance::palette(&ctx);
    let response = icon_button(ui, Icon::More, label, false);
    egui::Popup::menu(&response)
        .frame(
            Frame::popup(&ctx.style())
                .fill(colours.panel2)
                .inner_margin(Margin::same(10)),
        )
        .close_behavior(if close_on_click {
            egui::PopupCloseBehavior::CloseOnClick
        } else {
            egui::PopupCloseBehavior::CloseOnClickOutside
        })
        .show(add_contents);
}
pub(crate) fn action_at(
    ui: &mut Ui,
    rect: Rect,
    id: impl std::hash::Hash,
    kind: Icon,
    label: &str,
    active: bool,
) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let enabled = ui.is_enabled();
    let response = ui.interact(
        rect,
        ui.id().with(id),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 6.0, colours.hover);
    }
    icon_at(
        ui,
        rect,
        kind,
        if !enabled {
            colours.muted.gamma_multiply(0.45)
        } else if active {
            colours.green
        } else if response.hovered() {
            colours.text
        } else {
            colours.muted
        },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let response = response.on_hover_text(label);
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

fn playlist_play_at(
    ui: &mut Ui,
    rect: Rect,
    id: impl std::hash::Hash,
    playing: bool,
) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let response = ui.interact(rect, ui.id().with(id), Sense::click());
    ui.painter().circle_filled(
        rect.center(),
        rect.width() * 0.5,
        if response.hovered() {
            if colours.dark {
                Color32::from_rgb(147, 240, 199)
            } else {
                Color32::from_rgb(0, 92, 60)
            }
        } else {
            colours.green
        },
    );
    icon_at(
        ui,
        rect.shrink(rect.width() * 0.27),
        if playing { Icon::Pause } else { Icon::Play },
        colours.button_foreground,
    );
    let label = if playing {
        "Pause playlist"
    } else {
        "Play playlist"
    };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}
fn player_action(
    ui: &mut Ui,
    rect: Rect,
    id: impl std::hash::Hash,
    kind: Icon,
    label: &str,
    active: bool,
) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let response = ui.interact(rect, ui.id().with(id), Sense::click());
    let color = if active {
        colours.green
    } else if response.hovered() || response.has_focus() {
        colours.player_text
    } else {
        colours.player_muted
    };
    icon_at(ui, rect, kind, color);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response
        .on_hover_text(label)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}
fn caption_button(ui: &Ui, rect: Rect, kind: Icon, label: &str) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let response = ui.interact(rect, ui.id().with(label), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            6.0,
            if matches!(kind, Icon::Close) {
                Color32::from_rgb(190, 45, 58)
            } else {
                colours.hover
            },
        );
    }
    icon_at(
        ui,
        rect,
        kind,
        if response.hovered() {
            colours.text
        } else {
            colours.muted
        },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_text(label)
}
fn text(ui: &Ui, position: Pos2, value: &str, size: f32, color: Color32, width: f32) {
    text_with_font(
        ui,
        position,
        value,
        FontId::proportional(size),
        color,
        width,
    );
}
fn text_with_font(ui: &Ui, position: Pos2, value: &str, font: FontId, color: Color32, width: f32) {
    let mut job = LayoutJob::simple(value.to_owned(), font, color, width.max(1.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter().galley(position, galley, color);
}

fn account_action(ui: &mut Ui, label: &str) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 6.0, colours.hover);
    }
    text(
        ui,
        Pos2::new(rect.left() + 8.0, rect.center().y - 8.0),
        label,
        13.0,
        colours.text,
        rect.width() - 16.0,
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, label));
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
pub(crate) fn card(ctx: &egui::Context) -> Frame {
    let colours = appearance::palette(ctx);
    Frame::new()
        .fill(colours.panel)
        .corner_radius(10)
        .inner_margin(Margin::same(16))
}

fn preference(ui: &mut Ui, value: &mut bool, title: &str, description: &str) -> bool {
    let colours = appearance::palette(ui.ctx());
    let changed = ui
        .scope(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let changed = ui.checkbox(value, title).changed();
            ui.label(RichText::new(description).size(12.0).color(colours.muted));
            changed
        })
        .inner;
    ui.add_space(4.0);
    changed
}

fn start_page_label(page: preferences::StartPage) -> &'static str {
    match page {
        preferences::StartPage::LastView => "Last view",
        preferences::StartPage::Home => "Home",
        preferences::StartPage::Search => "Search",
        preferences::StartPage::Library => "Your library",
    }
}
pub(crate) fn primary<'a>(label: &'a str, ctx: &egui::Context) -> egui::Button<'a> {
    let colours = appearance::palette(ctx);
    egui::Button::new(
        RichText::new(label)
            .strong()
            .color(colours.button_foreground),
    )
    .fill(colours.green)
    .corner_radius(20)
    .min_size(Vec2::new(0.0, 32.0))
}

pub(crate) fn pill<'a>(label: &'a str, active: bool, ctx: &egui::Context) -> egui::Button<'a> {
    let colours = appearance::palette(ctx);
    if active {
        egui::Button::new(RichText::new(label).size(12.5).strong().color(colours.bg))
            .fill(colours.text)
            .stroke(Stroke::new(1.0_f32, colours.text))
            .corner_radius(16)
            .min_size(Vec2::new(0.0, 28.0))
    } else {
        egui::Button::new(RichText::new(label).size(12.5).color(colours.text))
            .fill(colours.panel2)
            .stroke(Stroke::new(1.0_f32, colours.border))
            .corner_radius(16)
            .min_size(Vec2::new(0.0, 28.0))
    }
}

pub(crate) fn action_button(
    ui: &mut Ui,
    label: &str,
    kind: Option<Icon>,
    active: bool,
) -> egui::Response {
    let enabled = ui.is_enabled();
    pill_button_sized(ui, label, kind, active, enabled, false)
}

pub(crate) fn compact_pill_button(
    ui: &mut Ui,
    label: &str,
    kind: Option<Icon>,
    active: bool,
) -> egui::Response {
    let enabled = ui.is_enabled();
    pill_button_sized(ui, label, kind, active, enabled, true)
}

pub(crate) fn compact_pill_button_enabled(
    ui: &mut Ui,
    label: &str,
    kind: Option<Icon>,
    active: bool,
    enabled: bool,
) -> egui::Response {
    pill_button_sized(ui, label, kind, active, enabled && ui.is_enabled(), true)
}

pub(crate) fn compact_pill_width(ui: &Ui, label: &str, kind: Option<Icon>) -> f32 {
    let fg = appearance::palette(ui.ctx()).text;
    pill_button_layout(ui, label, kind, fg, true).0
}

fn pill_button_layout(
    ui: &Ui,
    label: &str,
    kind: Option<Icon>,
    fg: Color32,
    compact: bool,
) -> (f32, f32, std::sync::Arc<egui::Galley>) {
    let height = if compact { 28.0 } else { 32.0 };
    let font_size = if compact { 11.5 } else { 12.5 };
    let pad_x = if compact { 11.0 } else { 13.0 };
    let icon_size = if compact { 12.0 } else { 13.5 };
    let icon_gap = 6.0;
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), egui::FontId::proportional(font_size), fg);
    let icon_w = if kind.is_some() {
        icon_size + icon_gap
    } else {
        0.0
    };
    let width = (pad_x * 2.0 + icon_w + galley.size().x).max(height);
    (width, height, galley)
}

fn pill_button_sized(
    ui: &mut Ui,
    label: &str,
    kind: Option<Icon>,
    active: bool,
    enabled: bool,
    compact: bool,
) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let pad_x = if compact { 11.0 } else { 13.0 };
    let icon_size = if compact { 12.0 } else { 13.5 };
    let icon_gap = 6.0;
    let fg = if !enabled {
        colours.muted.gamma_multiply(0.55)
    } else if active {
        colours.bg
    } else {
        colours.text
    };
    let (width, height, galley) = pill_button_layout(ui, label, kind, fg, compact);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), sense);
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let fill = if !enabled {
        colours.panel2.gamma_multiply(0.6)
    } else if active {
        if response.hovered() {
            colours.text.gamma_multiply(0.92)
        } else {
            colours.text
        }
    } else if response.is_pointer_button_down_on() {
        colours.hover.gamma_multiply(1.15)
    } else if response.hovered() {
        colours.hover
    } else {
        colours.panel2
    };
    let stroke_colour = if !enabled {
        colours.border.gamma_multiply(0.6)
    } else if active {
        colours.text
    } else if response.hovered() {
        colours.muted.gamma_multiply(0.75)
    } else {
        colours.border
    };
    let radius = (height * 0.5).round() as u8;
    ui.painter().rect(
        rect,
        CornerRadius::same(radius),
        fill,
        Stroke::new(1.0_f32, stroke_colour),
        egui::StrokeKind::Inside,
    );
    let mut text_left = rect.left() + pad_x;
    if let Some(icon_kind) = kind {
        let icon_rect = Rect::from_center_size(
            Pos2::new(text_left + icon_size * 0.5, rect.center().y),
            Vec2::new(icon_size, icon_size),
        );
        icon(ui.painter(), icon_rect, icon_kind, fg);
        text_left += icon_size + icon_gap;
    }
    ui.painter().galley(
        Pos2::new(text_left, rect.center().y - galley.size().y * 0.5),
        galley,
        fg,
    );
    response
}

pub(crate) fn search_pill_input(
    ui: &mut Ui,
    value: &mut String,
    id: impl std::hash::Hash,
    hint: &str,
    width: f32,
    clear_label: &str,
) -> bool {
    let colours = appearance::palette(ui.ctx());
    let mut changed = false;
    let target_w = width.max(120.0);
    egui::Frame::NONE
        .fill(colours.panel2)
        .stroke(Stroke::new(1.0_f32, colours.border))
        .corner_radius(16)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.set_width((target_w - 20.0).max(80.0));
            ui.set_height(24.0);
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let (search_rect, _) =
                    ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                icon(ui.painter(), search_rect, Icon::Search, colours.muted);
                ui.add_space(2.0);
                let clear_w = if value.is_empty() { 0.0 } else { 22.0 };
                let edit_w = (ui.available_width() - clear_w).max(50.0);
                let edit = ui.add_sized(
                    [edit_w, 22.0],
                    egui::TextEdit::singleline(value)
                        .id_salt(id)
                        .frame(false)
                        .margin(Margin::ZERO)
                        .vertical_align(Align::Center)
                        .hint_text(hint),
                );
                changed |= edit.changed();
                if !value.is_empty() {
                    let (clear_rect, clear_resp) =
                        ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::click());
                    if clear_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        ui.painter()
                            .circle_filled(clear_rect.center(), 9.0, colours.hover);
                    }
                    icon(
                        ui.painter(),
                        clear_rect.shrink(4.0),
                        Icon::Close,
                        colours.muted,
                    );
                    if clear_resp.on_hover_text(clear_label).clicked() {
                        value.clear();
                        changed = true;
                    }
                }
            });
        });
    changed
}

fn player_slider(ui: &mut Ui, value: &mut f32, width: f32, label: &str) -> egui::Response {
    let colours = appearance::palette(ui.ctx());
    let (rect, mut response) =
        ui.allocate_exact_size(Vec2::new(width.max(32.0), 24.0), Sense::click_and_drag());
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width() - 12.0, 4.0));
    let old = *value;
    if (response.is_pointer_button_down_on() || response.clicked() || response.dragged())
        && let Some(pos) = response.interact_pointer_pos()
    {
        *value = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
        response.request_focus();
    }
    if response.has_focus() {
        ui.ctx()
            .data_mut(|data| data.insert_temp(Id::new("player-slider-focus"), response.id));
        let delta = ui.input(|i| {
            if i.modifiers.any() {
                0.0
            } else if i.key_pressed(Key::ArrowRight) {
                0.01
            } else if i.key_pressed(Key::ArrowLeft) {
                -0.01
            } else {
                0.0
            }
        });
        *value = (*value + delta).clamp(0.0, 1.0);
    }
    if *value != old {
        response.mark_changed();
    }
    let highlighted = response.hovered() || response.dragged() || response.has_focus();
    ui.painter().rect_filled(track, 2.0, colours.slider);
    let x = track.left() + track.width() * *value;
    ui.painter().rect_filled(
        Rect::from_min_max(track.min, Pos2::new(x, track.bottom())),
        2.0,
        if highlighted {
            colours.green
        } else {
            colours.player_text
        },
    );
    if highlighted {
        ui.painter()
            .circle_filled(Pos2::new(x, track.center().y), 5.0, colours.player_text);
    }
    response
        .clone()
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Slider, true, label));
    response.on_hover_text(label)
}
fn heading(ui: &mut Ui, title: &str, subtitle: &str) {
    let colours = appearance::palette(ui.ctx());
    ui.label(RichText::new(title).size(30.0).strong());
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(13.0).color(colours.muted));
    }
    ui.add_space(18.0);
}

impl AurafyApp {
    pub(super) fn artist_links(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        track: &Track,
        position: Pos2,
        size: f32,
        width: f32,
    ) -> bool {
        self.artist_links_with_font(ui, ctx, track, position, FontId::proportional(size), width)
    }

    pub(super) fn artist_links_with_font(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        track: &Track,
        position: Pos2,
        font: FontId,
        width: f32,
    ) -> bool {
        let colours = appearance::palette(ctx);
        if track.uri.is_empty() && !self.is_demo_mode() {
            text_with_font(ui, position, &track.artist, font, colours.muted, width);
            return false;
        }
        let names = if track.artist_names.is_empty() {
            vec![track.artist.clone()]
        } else {
            track.artist_names.clone()
        };
        let mut x = position.x;
        let mut clicked = false;
        for (index, name) in names.iter().enumerate() {
            if name.trim().is_empty() || x >= position.x + width {
                break;
            }
            if index > 0 {
                text_with_font(
                    ui,
                    Pos2::new(x, position.y),
                    ", ",
                    font.clone(),
                    colours.muted,
                    9.0,
                );
                x += 9.0;
            }
            let remaining = position.x + width - x;
            if remaining <= 0.0 {
                break;
            }
            let measured = ui
                .painter()
                .layout_no_wrap(name.clone(), font.clone(), colours.muted)
                .size();
            let rect = Rect::from_min_size(
                Pos2::new(x, position.y - 2.0),
                Vec2::new(measured.x.min(remaining), measured.y + 4.0),
            );
            let response = ui
                .interact(
                    rect,
                    ui.id().with((
                        "artist",
                        &track.id,
                        index,
                        position.x.to_bits(),
                        position.y.to_bits(),
                    )),
                    Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            text_with_font(
                ui,
                Pos2::new(x, position.y),
                name,
                font.clone(),
                if response.hovered() {
                    colours.text
                } else {
                    colours.muted
                },
                remaining,
            );
            if response.hovered() {
                ui.painter().line_segment(
                    [
                        Pos2::new(x, position.y + measured.y),
                        Pos2::new(rect.right(), position.y + measured.y),
                    ],
                    Stroke::new(0.7_f32, colours.text),
                );
            }
            if response.on_hover_text(format!("View {name}")).clicked() {
                self.open_artist_id(
                    name.clone(),
                    track.artist_ids.get(index).cloned().unwrap_or_default(),
                    ctx,
                );
                clicked = true;
            }
            x += measured.x;
        }
        clicked
    }

    pub(super) fn navigate(&mut self, page: Page, ctx: &egui::Context) {
        if self.page != page || (page == Page::Library && self.selected_playlist.is_some()) {
            self.record_nav();
        }
        self.page = page;
        self.search_popup_open = false;
        self.search_edited_at = None;
        if page == Page::Library {
            self.selected_playlist = None;
        }
        if matches!(page, Page::Library | Page::Liked) {
            self.load_library(ctx, false);
        }
        if page == Page::Recent {
            self.load_home_data(ctx);
        }
        if page == Page::Search {
            ctx.memory_mut(|m| m.request_focus(Id::new("global-search")));
        }
    }

    pub(crate) fn art(
        &mut self,
        ui: &Ui,
        rect: Rect,
        url: Option<&str>,
        name: &str,
        ctx: &egui::Context,
    ) {
        self.art_rounded(ui, rect, url, name, ctx, 8);
    }

    pub(super) fn art_rounded(
        &mut self,
        ui: &Ui,
        rect: Rect,
        url: Option<&str>,
        name: &str,
        ctx: &egui::Context,
        radius: u8,
    ) {
        let colours = appearance::palette(ctx);
        // ScrollArea still lays out clipped rows. They must neither download
        // artwork nor keep offscreen covers pinned in the texture cache.
        if !ui.is_rect_visible(rect) {
            return;
        }
        if self.config.hide_artwork {
            ui.painter().rect_filled(rect, radius, colours.panel2);
            icon_at(
                ui,
                Rect::from_center_size(rect.center(), Vec2::splat(rect.width().min(32.0) * 0.5)),
                Icon::Library,
                colours.muted,
            );
            return;
        }
        if let Some(url) = url {
            if let Some(texture) = self.artwork_cache.get(url) {
                egui::Image::new((texture.id(), rect.size()))
                    .corner_radius(radius)
                    .paint_at(ui, rect);
                return;
            }
            self.request_artwork(url, ctx);
        }
        paint_cover_rounded(ui, rect, name, radius as f32);
    }

    pub(super) fn avatar(&mut self, ui: &Ui, rect: Rect, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        if !ui.is_rect_visible(rect) {
            return;
        }
        let url = self.user_profile.as_ref().and_then(|p| p.image_url.clone());
        if let Some(url) = url {
            if let Some(texture) = self.artwork_cache.get(&url) {
                egui::Image::new((texture.id(), rect.size()))
                    .corner_radius(255)
                    .paint_at(ui, rect);
                return;
            }
            self.request_artwork(&url, ctx);
        }
        ui.painter()
            .circle_filled(rect.center(), rect.width() * 0.5, colours.selected);
        let initial = self
            .user_name
            .as_deref()
            .unwrap_or("You")
            .chars()
            .next()
            .unwrap_or('Y')
            .to_uppercase()
            .to_string();
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            initial,
            FontId::proportional(rect.width() * 0.4),
            colours.text,
        );
    }

    pub(crate) fn error_notice(&self, ui: &mut Ui, error: &str) {
        let colours = appearance::palette(ui.ctx());
        Frame::new()
            .fill(if colours.dark {
                Color32::from_rgb(58, 36, 38)
            } else {
                Color32::from_rgb(255, 228, 228)
            })
            .corner_radius(9)
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(error).size(12.0).color(if colours.dark {
                    Color32::from_rgb(244, 177, 171)
                } else {
                    Color32::from_rgb(143, 30, 30)
                }));
            });
    }
    pub(crate) fn persist_preferences(&mut self) {
        self.config.settings_tab = self.settings_tab;
        if let Err(e) = self.config.save() {
            self.set_status(e);
        }
    }
    pub(crate) fn draw_toast(&mut self, ctx: &egui::Context) {
        let colours = appearance::palette(ctx);
        if self.status_message.is_empty() || Instant::now() >= self.status_until {
            return;
        }
        ctx.request_repaint_after(self.status_until.saturating_duration_since(Instant::now()));
        let screen = ctx.content_rect();
        egui::Area::new(Id::new("status-toast"))
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-20.0, -115.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(colours.panel2)
                    .stroke(Stroke::new(1.0_f32, colours.border))
                    .corner_radius(10)
                    .inner_margin(Margin::same(13))
                    .show(ui, |ui| {
                        ui.set_max_width((screen.width() - 260.0).min(430.0));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&self.status_message).size(12.0));
                            if icon_button(ui, Icon::Close, "Dismiss notification", false).clicked()
                            {
                                self.status_until = Instant::now();
                            }
                        });
                    });
            });
    }
}
