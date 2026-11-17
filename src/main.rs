#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appearance;
mod artwork;
mod audio_prefs;
mod audio_session;
mod browse;
mod browser;
mod desktop;
mod free_player;
mod i18n;
mod local_player;
mod lyrics;
#[cfg(target_os = "windows")]
mod player_capture;
mod player_layout;
mod player_process;
mod preferences;
#[cfg(target_os = "windows")]
mod smtc;
mod spotify;
mod startup;
mod ui;
#[cfg(test)]
mod ui_preview;
mod updates;
mod volume;

mod app;
pub(crate) use app::*;

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "windows")]
    let Some(mut instance) =
        desktop::Instance::claim().map_err(|error| eframe::Error::AppCreation(Box::new(error)))?
    else {
        return Ok(());
    };
    let display = AppConfig::load().display;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Aurafy")
            .with_decorations(display.native_titlebar)
            .with_resizable(true)
            .with_icon(desktop::window_icon())
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1080.0 * display.zoom(), 680.0 * display.zoom()]),
        renderer: eframe::Renderer::Glow,
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        "Aurafy",
        options,
        Box::new(move |cc| {
            let mut app = AurafyApp::new(cc);
            #[cfg(target_os = "windows")]
            {
                instance.listen(cc, app.tx.clone());
                app.desktop_instance = Some(instance);
            }
            Ok(Box::new(app))
        }),
    )
}
