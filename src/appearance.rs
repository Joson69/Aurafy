use egui::{Color32, Context, Id};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const BG: Color32 = Color32::from_rgb(18, 21, 24);
pub const PANEL: Color32 = Color32::from_rgb(13, 16, 19);
pub const PANEL_2: Color32 = Color32::from_rgb(27, 31, 35);
pub const HOVER: Color32 = Color32::from_rgb(36, 42, 46);
pub const TEXT: Color32 = Color32::from_rgb(246, 247, 248);
pub const MUTED: Color32 = Color32::from_rgb(151, 161, 169);
pub const GREEN: Color32 = Color32::from_rgb(119, 228, 183);
pub const BORDER: Color32 = Color32::from_rgb(42, 49, 54);

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    Dark,
    Black,
    CatppuccinMocha,
    Dracula,
    Nord,
    TokyoNight,
    RosePine,
    GruvboxDark,
    Light,
    System,
}
impl Theme {
    pub const ALL: [Self; 10] = [
        Self::Dark,
        Self::Black,
        Self::CatppuccinMocha,
        Self::Dracula,
        Self::Nord,
        Self::TokyoNight,
        Self::RosePine,
        Self::GruvboxDark,
        Self::Light,
        Self::System,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Black => "AMOLED Black",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::Dracula => "Dracula",
            Self::Nord => "Nord",
            Self::TokyoNight => "Tokyo Night",
            Self::RosePine => "Rosé Pine",
            Self::GruvboxDark => "Gruvbox Dark",
            Self::Light => "Light",
            Self::System => "Follow system",
        }
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayStyle {
    #[default]
    Colour,
    White,
    Black,
}
impl TrayStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Colour => "Colour",
            Self::White => "White",
            Self::Black => "Black",
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub zoom_percent: u16,
    pub playback_fps: u8,
    pub compact_sidebar: bool,
    pub native_titlebar: bool,
    pub album_colours: bool,
    pub tray_style: TrayStyle,
    #[serde(default)]
    pub custom_accent: Option<[u8; 3]>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            zoom_percent: 100,
            playback_fps: 30,
            compact_sidebar: false,
            native_titlebar: false,
            album_colours: false,
            tray_style: TrayStyle::Colour,
            custom_accent: None,
        }
    }
}
impl Settings {
    pub fn zoom(self) -> f32 {
        f32::from(self.zoom_percent.clamp(80, 125)) / 100.0
    }
    pub fn fps(self) -> u8 {
        match self.playback_fps {
            15 | 30 | 60 => self.playback_fps,
            _ => 30,
        }
    }
    pub fn interval(self, playing: bool, hidden: bool, smooth: bool, reduced: bool) -> Duration {
        if !playing || hidden {
            Duration::from_secs(1)
        } else if smooth && !reduced {
            Duration::from_secs_f64(1.0 / f64::from(self.fps()))
        } else {
            Duration::from_secs(1)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub panel2: Color32,
    pub hover: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub green: Color32,
    pub border: Color32,
    pub selected: Color32,
    pub player: Color32,
    pub player_text: Color32,
    pub player_muted: Color32,
    pub button_foreground: Color32,
    pub slider: Color32,
    pub lyrics: Color32,
    pub lyrics_past: Color32,
    pub lyrics_future: Color32,
    pub dark: bool,
}
pub fn resolve(settings: Settings, system: Option<egui::Theme>, album: Option<Color32>) -> Palette {
    let dark = settings.theme != Theme::Light
        && !(settings.theme == Theme::System && system == Some(egui::Theme::Light));
    let mut p = if dark {
        Palette {
            bg: BG,
            panel: PANEL,
            panel2: PANEL_2,
            hover: HOVER,
            text: TEXT,
            muted: MUTED,
            green: GREEN,
            border: BORDER,
            selected: Color32::from_gray(40),
            player: Color32::BLACK,
            player_text: Color32::WHITE,
            player_muted: Color32::from_gray(179),
            button_foreground: PANEL,
            slider: Color32::from_gray(77),
            lyrics: Color32::from_rgb(22, 42, 35),
            lyrics_past: Color32::from_rgb(206, 219, 212),
            lyrics_future: Color32::from_rgb(157, 180, 167),
            dark: true,
        }
    } else {
        Palette {
            bg: Color32::from_rgb(246, 248, 250),
            panel: Color32::from_rgb(236, 240, 243),
            panel2: Color32::WHITE,
            hover: Color32::from_rgb(220, 229, 234),
            text: Color32::from_rgb(20, 30, 35),
            muted: Color32::from_rgb(84, 99, 108),
            green: Color32::from_rgb(0, 112, 72),
            border: Color32::from_rgb(199, 210, 218),
            selected: Color32::from_rgb(214, 222, 228),
            player: Color32::from_rgb(250, 252, 253),
            player_text: Color32::from_rgb(20, 30, 35),
            player_muted: Color32::from_rgb(84, 99, 108),
            button_foreground: Color32::WHITE,
            slider: Color32::from_rgb(180, 194, 200),
            lyrics: Color32::from_rgb(218, 237, 226),
            lyrics_past: Color32::from_rgb(51, 81, 66),
            lyrics_future: Color32::from_rgb(76, 105, 90),
            dark: false,
        }
    };
    if settings.theme == Theme::Black {
        p.bg = Color32::BLACK;
        p.panel = Color32::BLACK;
        p.panel2 = Color32::from_gray(15);
        p.hover = Color32::from_gray(28);
        p.selected = Color32::from_gray(32);
        p.border = Color32::from_gray(38);
    } else if settings.theme == Theme::CatppuccinMocha {
        p.bg = Color32::from_rgb(30, 30, 46);
        p.panel = Color32::from_rgb(24, 24, 37);
        p.panel2 = Color32::from_rgb(49, 50, 68);
        p.hover = Color32::from_rgb(69, 71, 90);
        p.selected = Color32::from_rgb(88, 91, 112);
        p.text = Color32::from_rgb(205, 214, 244);
        p.muted = Color32::from_rgb(166, 173, 200);
        p.green = Color32::from_rgb(203, 166, 247); // Mauve
        p.border = Color32::from_rgb(69, 71, 90);
        p.player = Color32::from_rgb(17, 17, 27);
        p.player_text = p.text;
        p.player_muted = p.muted;
        p.button_foreground = Color32::from_rgb(17, 17, 27);
        p.lyrics = Color32::from_rgb(38, 36, 56);
    } else if settings.theme == Theme::Dracula {
        p.bg = Color32::from_rgb(40, 42, 54);
        p.panel = Color32::from_rgb(33, 34, 44);
        p.panel2 = Color32::from_rgb(52, 55, 70);
        p.hover = Color32::from_rgb(68, 71, 90);
        p.selected = Color32::from_rgb(80, 84, 108);
        p.text = Color32::from_rgb(248, 248, 242);
        p.muted = Color32::from_rgb(164, 172, 200);
        p.green = Color32::from_rgb(80, 250, 123);
        p.border = Color32::from_rgb(68, 71, 90);
        p.player = Color32::from_rgb(25, 26, 33);
        p.player_text = p.text;
        p.player_muted = p.muted;
        p.button_foreground = Color32::from_rgb(25, 26, 33);
        p.lyrics = Color32::from_rgb(44, 48, 63);
    } else if settings.theme == Theme::Nord {
        p.bg = Color32::from_rgb(46, 52, 64);
        p.panel = Color32::from_rgb(36, 41, 51);
        p.panel2 = Color32::from_rgb(59, 66, 82);
        p.hover = Color32::from_rgb(67, 76, 94);
        p.selected = Color32::from_rgb(76, 86, 106);
        p.text = Color32::from_rgb(236, 239, 244);
        p.muted = Color32::from_rgb(180, 190, 206);
        p.green = Color32::from_rgb(136, 192, 208); // Nord Frost
        p.border = Color32::from_rgb(67, 76, 94);
        p.player = Color32::from_rgb(31, 35, 44);
        p.player_text = p.text;
        p.player_muted = p.muted;
        p.button_foreground = Color32::from_rgb(31, 35, 44);
        p.lyrics = Color32::from_rgb(49, 58, 73);
    } else if settings.theme == Theme::TokyoNight {
        p.bg = Color32::from_rgb(26, 27, 38);
        p.panel = Color32::from_rgb(22, 22, 30);
        p.panel2 = Color32::from_rgb(36, 40, 59);
        p.hover = Color32::from_rgb(47, 53, 73);
        p.selected = Color32::from_rgb(61, 69, 97);
        p.text = Color32::from_rgb(192, 202, 245);
        p.muted = Color32::from_rgb(142, 152, 196);
        p.green = Color32::from_rgb(122, 162, 247); // Tokyo Blue
        p.border = Color32::from_rgb(52, 59, 88);
        p.player = Color32::from_rgb(16, 16, 24);
        p.player_text = p.text;
        p.player_muted = p.muted;
        p.button_foreground = Color32::from_rgb(16, 16, 24);
        p.lyrics = Color32::from_rgb(31, 35, 53);
    } else if settings.theme == Theme::RosePine {
        p.bg = Color32::from_rgb(25, 23, 36);
        p.panel = Color32::from_rgb(20, 18, 30);
        p.panel2 = Color32::from_rgb(38, 35, 58);
        p.hover = Color32::from_rgb(57, 53, 82);
        p.selected = Color32::from_rgb(68, 65, 90);
        p.text = Color32::from_rgb(224, 222, 244);
        p.muted = Color32::from_rgb(144, 140, 170);
        p.green = Color32::from_rgb(235, 188, 186); // Rose
        p.border = Color32::from_rgb(57, 53, 82);
        p.player = Color32::from_rgb(16, 14, 24);
        p.player_text = p.text;
        p.player_muted = p.muted;
        p.button_foreground = Color32::from_rgb(25, 23, 36);
        p.lyrics = Color32::from_rgb(38, 34, 54);
    } else if settings.theme == Theme::GruvboxDark {
        p.bg = Color32::from_rgb(40, 40, 40);
        p.panel = Color32::from_rgb(29, 32, 33);
        p.panel2 = Color32::from_rgb(60, 56, 54);
        p.hover = Color32::from_rgb(80, 73, 69);
        p.selected = Color32::from_rgb(102, 92, 84);
        p.text = Color32::from_rgb(235, 219, 178);
        p.muted = Color32::from_rgb(189, 174, 147);
        p.green = Color32::from_rgb(184, 187, 38); // Gruvbox Green
        p.border = Color32::from_rgb(80, 73, 69);
        p.player = Color32::from_rgb(24, 26, 27);
        p.player_text = p.text;
        p.player_muted = p.muted;
        p.button_foreground = Color32::from_rgb(29, 32, 33);
        p.lyrics = Color32::from_rgb(50, 48, 47);
    }
    if let Some([r, g, b]) = settings.custom_accent {
        p.green = Color32::from_rgb(r, g, b);
    }
    if settings.album_colours
        && let Some(colour) = album
    {
        p.bg = blend(p.bg, colour, if dark { 14 } else { 7 });
        p.panel = blend(p.panel, colour, if dark { 10 } else { 5 });
        p.player = blend(p.player, colour, if dark { 18 } else { 8 });
    }
    p
}
fn blend(base: Color32, tint: Color32, percent: u16) -> Color32 {
    let channel =
        |i| ((u16::from(base[i]) * (100 - percent) + u16::from(tint[i]) * percent) / 100) as u8;
    Color32::from_rgb(channel(0), channel(1), channel(2))
}
pub fn palette(ctx: &Context) -> Palette {
    ctx.data(|data| data.get_temp::<Palette>(Id::new("aurafy-palette")))
        .unwrap_or_else(|| resolve(Settings::default(), None, None))
}
pub fn store(ctx: &Context, palette: Palette) {
    ctx.data_mut(|data| data.insert_temp(Id::new("aurafy-palette"), palette));
}
pub fn album_colour(pixels: &[u8]) -> Color32 {
    let mut sum = [0u64; 3];
    let mut count = 0u64;
    for pixel in pixels.chunks_exact(4).step_by(16) {
        if pixel[3] < 128 {
            continue;
        }
        let max = *pixel[..3].iter().max().unwrap();
        let min = *pixel[..3].iter().min().unwrap();
        if max < 45 || max.saturating_sub(min) < 20 {
            continue;
        }
        for i in 0..3 {
            sum[i] += u64::from(pixel[i]);
        }
        count += 1;
    }
    if count == 0 {
        Color32::from_gray(80)
    } else {
        Color32::from_rgb(
            (sum[0] / count) as u8,
            (sum[1] / count) as u8,
            (sum[2] / count) as u8,
        )
    }
}
pub fn tray_rgba(style: TrayStyle) -> Result<Vec<u8>, String> {
    let bytes: &[u8] = match style {
        TrayStyle::Colour => include_bytes!("../assets/tray.png"),
        TrayStyle::White => include_bytes!("../assets/tray-white.png"),
        TrayStyle::Black => include_bytes!("../assets/tray-black.png"),
    };
    Ok(image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .into_rgba8()
        .into_raw())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn theme_and_album_colours_are_local_to_each_context() {
        let light = resolve(
            Settings {
                theme: Theme::Light,
                ..Default::default()
            },
            None,
            None,
        );
        let dark = resolve(Settings::default(), None, None);
        assert!(light.bg.r() > 230 && light.text.r() < 30);
        assert_eq!(
            resolve(
                Settings {
                    theme: Theme::Black,
                    ..Default::default()
                },
                None,
                None
            )
            .bg,
            Color32::BLACK
        );
        assert_eq!(
            resolve(
                Settings {
                    theme: Theme::System,
                    ..Default::default()
                },
                Some(egui::Theme::Light),
                None
            ),
            light
        );
        let a = Context::default();
        let b = Context::default();
        store(&a, light);
        store(&b, dark);
        assert_eq!(palette(&a).bg, light.bg);
        assert_eq!(palette(&b).bg, dark.bg);
        let tint = resolve(
            Settings {
                album_colours: true,
                ..Default::default()
            },
            None,
            Some(Color32::RED),
        );
        assert!(tint.bg.r() > dark.bg.r() && tint.text == dark.text);
    }
    #[test]
    fn refresh_preferences_remain_quiet_when_idle_or_hidden() {
        let settings = Settings {
            zoom_percent: 900,
            playback_fps: 60,
            ..Default::default()
        };
        assert_eq!(settings.zoom(), 1.25);
        assert_eq!(
            settings.interval(false, false, true, false),
            Duration::from_secs(1)
        );
        assert_eq!(
            settings.interval(true, true, true, false),
            Duration::from_secs(1)
        );
        assert!(settings.interval(true, false, true, false) < Duration::from_millis(17));
        assert_eq!(
            settings.interval(true, false, true, true),
            Duration::from_secs(1)
        );
    }
    #[test]
    fn tray_variants_have_the_same_alpha_mask_and_do_not_change_colour_asset() {
        let colour = tray_rgba(TrayStyle::Colour).unwrap();
        let white = tray_rgba(TrayStyle::White).unwrap();
        let black = tray_rgba(TrayStyle::Black).unwrap();
        assert_eq!(colour.len(), 32 * 32 * 4);
        for (a, b) in white.chunks_exact(4).zip(black.chunks_exact(4)) {
            assert_eq!(a[3], b[3]);
            assert_eq!(&a[..3], &[255, 255, 255]);
            assert_eq!(&b[..3], &[0, 0, 0]);
        }
    }
}
