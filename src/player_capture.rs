//! Opt-in validation of the live native viewport, never the offscreen UI renderer.
//! The normal app has no capture job and performs no image writes here.
use super::*;

enum Stage {
    Start,
    Restored,
    AwaitRestored,
    Maximized,
    AwaitMaximized,
}

pub struct Session {
    directory: PathBuf,
    stage: Stage,
    started: Instant,
    deadline: Instant,
    original: Option<desktop::WindowPlacement>,
}

impl Session {
    pub fn from_env() -> Option<Self> {
        #[cfg(test)]
        {
            None
        }
        #[cfg(not(test))]
        {
            let directory = PathBuf::from(
                std::env::var_os("AURAFY_CAPTURE_PLAYER_PREVIEWS")
                    .or_else(|| std::env::var_os("AURALIS_CAPTURE_PLAYER_PREVIEWS"))?,
            );
            Some(Self {
                directory,
                stage: Stage::Start,
                started: Instant::now(),
                deadline: Instant::now() + Duration::from_secs(12),
                original: None,
            })
        }
    }

    pub fn update(
        &mut self,
        ctx: &egui::Context,
        geometry: &desktop::Geometry,
        layout: Option<player_layout::Geometry>,
        ready: bool,
    ) -> bool {
        let now = Instant::now();
        for event in ctx.input(|input| input.events.clone()) {
            if let egui::Event::Screenshot {
                user_data, image, ..
            } = event
                && let Some(tag) = user_data
                    .data
                    .as_ref()
                    .and_then(|data| data.downcast_ref::<String>())
                && matches!(tag.as_str(), "restored" | "maximized")
            {
                let Some(layout) = layout else { continue };
                if let Err(error) = self.save(ctx, tag, &image, layout) {
                    let _ = fs::write(self.directory.join("capture-error.txt"), error);
                    if let Some(original) = self.original {
                        geometry.restore(original);
                    }
                    return true;
                }
                if tag == "restored" {
                    if let Some(original) = self.original {
                        geometry.restore(desktop::WindowPlacement {
                            maximized: true,
                            ..original
                        });
                    }
                    self.stage = Stage::Maximized;
                    self.deadline = now + Duration::from_millis(1200);
                } else {
                    if let Some(original) = self.original {
                        geometry.restore(original);
                    }
                    return true;
                }
            }
        }
        if self.started.elapsed() > Duration::from_secs(60) {
            let _ = fs::create_dir_all(&self.directory);
            let _ = fs::write(
                self.directory.join("capture-error.txt"),
                "Timed out waiting for the live viewport capture.",
            );
            if let Some(original) = self.original {
                geometry.restore(original);
            }
            return true;
        }
        if now >= self.deadline {
            match self.stage {
                Stage::Start if ready => {
                    self.original = geometry.capture();
                    if let Some(original) = self.original {
                        geometry.restore(desktop::WindowPlacement {
                            maximized: false,
                            ..original
                        });
                    }
                    self.stage = Stage::Restored;
                    self.deadline = now + Duration::from_millis(1200);
                }
                Stage::Restored => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        "restored".to_owned(),
                    )));
                    self.stage = Stage::AwaitRestored;
                }
                Stage::Maximized
                    if ctx.input(|input| input.viewport().maximized.unwrap_or(false)) =>
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        "maximized".to_owned(),
                    )));
                    self.stage = Stage::AwaitMaximized;
                }
                _ => {}
            }
        }
        ctx.request_repaint_after(Duration::from_millis(50));
        false
    }

    fn save(
        &self,
        ctx: &egui::Context,
        tag: &str,
        source: &egui::ColorImage,
        layout: player_layout::Geometry,
    ) -> Result<(), String> {
        fs::create_dir_all(&self.directory).map_err(|error| error.to_string())?;
        let bytes: Vec<_> = source
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_array())
            .collect();
        let image =
            image::RgbaImage::from_raw(source.width() as u32, source.height() as u32, bytes)
                .ok_or("Invalid native screenshot dimensions")?;
        image
            .save(self.directory.join(format!("aurafy-{tag}.png")))
            .map_err(|error| error.to_string())?;
        let scale = source.width() as f32 / ctx.content_rect().width();
        let top = ((layout.bounds.top() * scale).round().max(0.0) as u32)
            .min(image.height().saturating_sub(1));
        let height = (layout.bounds.height() * scale).round() as u32;
        image::imageops::crop_imm(
            &image,
            0,
            top.min(image.height().saturating_sub(1)),
            image.width(),
            height.min(image.height() - top),
        )
        .to_image()
        .save(self.directory.join(format!("aurafy-{tag}-player.png")))
        .map_err(|error| error.to_string())?;
        let rect = |rect: Rect| [rect.left(), rect.top(), rect.right(), rect.bottom()];
        let report = serde_json::json!({
            "source":"live eframe native framebuffer", "version":env!("CARGO_PKG_VERSION"),
            "viewport_pixels":[image.width(),image.height()], "pixels_per_point":scale,
            "maximized":ctx.input(|input|input.viewport().maximized.unwrap_or(false)),
            "player":rect(layout.bounds), "midpoint_y":layout.bounds.center().y,
            "play_top_clearance":layout.transport[2].top()-layout.bounds.top(),
            "play_center_y_from_top":layout.transport[2].center().y-layout.bounds.top(),
            "timeline_center_y_from_top":layout.seek.center().y-layout.bounds.top(),
            "play_button":rect(layout.transport[2]), "play_clearance_above_midpoint":layout.bounds.center().y-layout.transport[2].bottom(),
            "zones":[rect(layout.left),rect(layout.center),rect(layout.right)],
            "title":rect(layout.title),"artist":rect(layout.artist),"collection_button":rect(layout.collection),"seek":rect(layout.seek),
            "utilities":layout.utilities.map(rect),"volume":rect(layout.volume)
        });
        fs::write(
            self.directory.join(format!("layout-{tag}.json")),
            serde_json::to_vec_pretty(&report).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }
}
