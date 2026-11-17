//! Native lifecycle check without Spotify, credentials, or the user's settings.
//! Run separately: cargo run --release --example desktop_smoke --locked
#[path = "../src/appearance.rs"]
#[allow(dead_code)]
mod appearance;
#[path = "../src/desktop.rs"]
#[allow(dead_code, unused_imports)]
mod desktop;

use eframe::egui;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

enum AppEvent {
    Desktop(desktop::Action),
}

struct Check {
    tray: desktop::Tray,
    window: desktop::WindowState,
    rx: mpsc::Receiver<AppEvent>,
    tx: mpsc::Sender<AppEvent>,
    started: Instant,
    stage: u8,
}

impl eframe::App for Check {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.tray.update(false, "Desktop lifecycle check", "Aurafy");
        while let Ok(AppEvent::Desktop(action)) = self.rx.try_recv() {
            match action {
                desktop::Action::Open => {
                    assert!(self.window.hidden);
                    self.window.open(ctx);
                    self.stage = 3;
                    println!("hidden window received wakeup and reopened");
                }
                desktop::Action::Quit => {
                    assert!(
                        self.window.hidden,
                        "Quit must also work from the hidden window"
                    );
                    self.stage = 6;
                    self.window.quit(ctx);
                    println!("hidden window processed Quit");
                }
                _ => {}
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.window.close(self.tray.registered(), false, ctx);
            if !self.window.quitting {
                assert!(self.window.hidden);
                let action = if self.stage == 4 {
                    desktop::Action::Quit
                } else {
                    desktop::Action::Open
                };
                self.stage = if self.stage == 4 { 5 } else { 2 };
                let tx = self.tx.clone();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(600));
                    tx.send(AppEvent::Desktop(action)).unwrap();
                    ctx.request_repaint();
                });
                println!("native close request canceled; window hidden");
            }
        }
        if self.stage == 0 && self.started.elapsed() >= Duration::from_millis(600) {
            assert!(
                self.tray.registered(),
                "Windows did not register the tray icon"
            );
            println!("tray icon registered with Windows");
            self.stage = 1;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.stage == 3 {
            assert!(self.tray.registered());
            self.stage = 4;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !self.window.hidden {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Checking Aurafy tray lifecycle…");
            });
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn main() -> eframe::Result<()> {
    // Fail instead of leaving an invisible test process if wakeup is broken.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(12));
        eprintln!("Desktop lifecycle check timed out");
        std::process::exit(2);
    });
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Aurafy desktop check")
            .with_icon(desktop::window_icon())
            .with_inner_size([340.0, 100.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Aurafy desktop check",
        options,
        Box::new(|cc| {
            let (tx, rx) = mpsc::channel();
            Ok(Box::new(Check {
                tray: desktop::Tray::new(
                    cc,
                    tx.clone(),
                    0xd726918d_1b54_4a3f_8703_b2a981fa803f,
                    appearance::TrayStyle::Colour,
                )
                .expect("Create tray"),
                window: Default::default(),
                rx,
                tx,
                started: Instant::now(),
                stage: 0,
            }))
        }),
    )?;
    println!("Quit closed the event loop and removed the tray icon");
    Ok(())
}
