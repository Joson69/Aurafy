use egui::{Pos2, Rect, Vec2};

/// All coordinates are relative to the allocated playback bar in logical points.
/// Transport has its own upper baseline; metadata and utilities have
/// their own vertical center, rather than sharing the transport's baseline.
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub bounds: Rect,
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
    pub artwork: Rect,
    pub title: Rect,
    pub artist: Rect,
    pub collection: Rect,
    pub queue_current: Rect,
    pub transport: [Rect; 5],
    pub seek: Rect,
    pub elapsed: Pos2,
    pub duration: Pos2,
    pub utilities: [Rect; 5],
    pub volume: Rect,
}

impl Geometry {
    pub const HEIGHT: f32 = 80.0;

    pub fn new(bounds: Rect, text_width: f32) -> Self {
        let height = bounds.height();
        let inset = 16.0;
        let button = 30.0;
        let play_button = 32.0;
        let gap = 4.0;
        let center_width = (bounds.width() * 0.40).min(722.0);
        let center = Rect::from_center_size(bounds.center(), Vec2::new(center_width, height));
        let left = Rect::from_min_max(
            bounds.min + Vec2::new(inset, 0.0),
            Pos2::new(center.left() - inset, bounds.bottom()),
        );
        let right = Rect::from_min_max(
            Pos2::new(center.right() + inset, bounds.top()),
            bounds.max - Vec2::new(inset, 0.0),
        );
        let artwork = Rect::from_center_size(
            Pos2::new(left.left() + 26.0, bounds.center().y),
            Vec2::splat(52.0),
        );
        let text_left = artwork.right() + 14.0;
        let collection_gap = 10.0;
        let collection_button_gap = 4.0;
        let available_text =
            (left.right() - text_left - collection_gap - collection_button_gap - 2.0 * button)
                .max(1.0);
        let text_width = text_width.ceil().max(57.6).min(available_text).min(288.0);
        let title = Rect::from_min_size(
            Pos2::new(text_left, bounds.center().y - 16.0),
            Vec2::new(text_width, 16.0),
        );
        let artist = Rect::from_min_size(
            Pos2::new(text_left, bounds.center().y + 3.0),
            Vec2::new(text_width, 16.0),
        );
        let collection = Rect::from_center_size(
            Pos2::new(
                title.right() + collection_gap + button * 0.5,
                bounds.center().y,
            ),
            Vec2::splat(button),
        );
        let queue_current = Rect::from_center_size(
            Pos2::new(
                collection.center().x + button + collection_button_gap,
                bounds.center().y,
            ),
            Vec2::splat(button),
        );
        let step = 40.0;
        let transport_y = bounds.top() + height * 0.375;
        let transport = std::array::from_fn(|i| {
            let size = if i == 2 { play_button } else { button };
            Rect::from_center_size(
                Pos2::new(center.center().x + (i as f32 - 2.0) * step, transport_y),
                Vec2::splat(size),
            )
        });
        let timeline_y = bounds.top() + height * 0.80;
        let time_space = 40.0;
        let seek = Rect::from_center_size(
            Pos2::new(center.center().x, timeline_y),
            Vec2::new(center.width() - 2.0 * time_space, 24.0),
        );
        let elapsed = Pos2::new(seek.left() - 8.0, timeline_y - 1.0);
        let duration = Pos2::new(seek.right() + 8.0, timeline_y - 1.0);
        let volume_width = 96.0;
        let utility_width = 5.0 * button + volume_width + 5.0 * gap;
        let scale = (right.width() / utility_width).min(1.0);
        let button = button * scale;
        let gap = gap * scale;
        let volume_width = volume_width * scale;
        let utility_width = utility_width * scale;
        let utility_left = right.right() - utility_width;
        let utilities = std::array::from_fn(|i| {
            let x = if i < 4 {
                utility_left + i as f32 * (button + gap)
            } else {
                right.right() - button
            };
            Rect::from_center_size(
                Pos2::new(x + button * 0.5, bounds.center().y),
                Vec2::splat(button),
            )
        });
        let volume = Rect::from_center_size(
            Pos2::new(
                utilities[3].right() + gap + volume_width * 0.5,
                bounds.center().y,
            ),
            Vec2::new(volume_width, 24.0),
        );
        Self {
            bounds,
            left,
            center,
            right,
            artwork,
            title,
            artist,
            collection,
            queue_current,
            transport,
            seek,
            elapsed,
            duration,
            utilities,
            volume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taller_player_preserves_horizontal_positions_and_control_sizes() {
        for width in [1080.0, 1260.0, 1680.0, 1920.0] {
            let before = Geometry::new(
                Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 72.0)),
                160.0,
            );
            let after = Geometry::new(
                Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 80.0)),
                160.0,
            );
            let elements = |layout: Geometry| {
                [
                    layout.artwork,
                    layout.title,
                    layout.artist,
                    layout.collection,
                    layout.seek,
                    layout.volume,
                    layout.transport[0],
                    layout.transport[2],
                    layout.transport[4],
                    layout.utilities[0],
                    layout.utilities[2],
                    layout.utilities[4],
                ]
            };
            for (old, new) in elements(before).into_iter().zip(elements(after)) {
                assert!((old.left() - new.left()).abs() < 0.001);
                assert!((old.width() - new.width()).abs() < 0.001);
                assert!((old.height() - new.height()).abs() < 0.001);
            }
        }
    }

    #[test]
    fn all_zones_stay_separate_and_rows_follow_the_container_height() {
        for width in [
            1080.0, 1193.0, 1260.0, 1440.0, 1680.0, 1920.0, 2560.0, 3840.0,
        ] {
            for height in [72.0, 80.0, 90.0] {
                for text in [45.0, 140.0, 500.0] {
                    let bounds =
                        Rect::from_min_size(Pos2::new(17.0, 731.0), Vec2::new(width, height));
                    let layout = Geometry::new(bounds, text);
                    let play = layout.transport[2];
                    assert!((play.width() - 32.0).abs() < 0.001);
                    assert!((play.height() - 32.0).abs() < 0.001);
                    assert!((play.center().y - bounds.top() - height * 0.375).abs() < 0.001);
                    if height == 80.0 {
                        assert!((play.top() - bounds.top() - 14.0).abs() < 0.001);
                    }
                    assert!((layout.seek.center().y - bounds.top() - height * 0.80).abs() < 0.001);
                    assert!((layout.artwork.center().y - bounds.center().y).abs() < 0.001);
                    assert!((layout.volume.center().y - bounds.center().y).abs() < 0.001);
                    assert!(bounds.contains_rect(layout.transport[2]));
                    assert!(layout.seek.top() > bounds.center().y);
                    assert!((layout.transport[2].center().x - bounds.center().x).abs() < 0.01);
                    assert!(layout.left.contains_rect(layout.collection));
                    assert!(layout.title.right() < layout.collection.left());
                    assert!(layout.artist.right() < layout.collection.left());
                    assert!(layout.right.contains_rect(layout.volume));
                    for rect in layout.utilities {
                        // Scaled f32 rectangles can differ by a fraction of a pixel.
                        assert!(
                            layout.right.expand(0.001).contains_rect(rect),
                            "{width} x {height}: {:?} outside {:?}",
                            rect,
                            layout.right
                        );
                    }
                    assert!(layout.utilities[3].right() < layout.volume.left());
                    assert!(layout.volume.right() < layout.utilities[4].left());
                    assert!(layout.left.right() < layout.center.left());
                    assert!(layout.center.right() < layout.right.left());
                }
            }
        }
    }
}
