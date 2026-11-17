//! Keep the visible working set stable while bounding the warm, offscreen cache.
//! Draw order, library size and asynchronous completions must not evict a cover
//! that is still on screen. In particular, HashMap iteration is not an LRU.
use egui::TextureHandle;
use std::collections::{HashMap, HashSet};

struct Entry {
    texture: TextureHandle,
    last_used: u64,
    colour: Option<egui::Color32>,
}

#[derive(Default)]
pub struct Cache {
    entries: HashMap<String, Entry>,
    visible: HashSet<String>,
    previous: HashSet<String>,
    clock: u64,
}

impl Cache {
    pub fn begin_frame(&mut self) {
        // Preserve the preceding view until all panels in this frame have had
        // a chance to mark their covers. Uploads can arrive before rendering.
        self.previous = std::mem::take(&mut self.visible);
    }

    pub fn finish_frame(&mut self) {
        self.previous.clear();
        self.trim();
    }

    pub fn get(&mut self, url: &str) -> Option<&TextureHandle> {
        // Mark missing images too: their uploads must be protected once ready.
        self.visible.insert(url.to_owned());
        self.clock = self.clock.saturating_add(1);
        let entry = self.entries.get_mut(url)?;
        entry.last_used = self.clock;
        Some(&entry.texture)
    }

    pub fn contains_key(&self, url: &str) -> bool {
        self.entries.contains_key(url)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    pub fn insert(&mut self, url: String, texture: TextureHandle) {
        self.clock = self.clock.saturating_add(1);
        self.entries.insert(
            url,
            Entry {
                texture,
                last_used: self.clock,
                colour: None,
            },
        );
        self.trim();
    }

    pub fn colour(&self, url: &str) -> Option<egui::Color32> {
        self.entries.get(url).and_then(|entry| entry.colour)
    }
    pub fn insert_with_colour(
        &mut self,
        url: String,
        texture: TextureHandle,
        colour: egui::Color32,
    ) {
        self.insert(url.clone(), texture);
        if let Some(entry) = self.entries.get_mut(&url) {
            entry.colour = Some(colour);
        }
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&String, &TextureHandle) -> bool) {
        self.entries.retain(|url, entry| keep(url, &entry.texture));
        self.visible.retain(|url| self.entries.contains_key(url));
        self.previous.retain(|url| self.entries.contains_key(url));
    }

    fn trim(&mut self) {
        while self.entries.len() > crate::MAX_ARTWORK_CACHE {
            let victim = self
                .entries
                .iter()
                .filter(|(url, _)| !self.visible.contains(*url) && !self.previous.contains(*url))
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(url, _)| url.clone());
            let Some(victim) = victim else {
                // A large/high-DPI view may itself exceed the warm-cache limit.
                // Retain that working set, not an arbitrarily flashing subset.
                // Clipped rows never enter `visible`; scrolling releases pins.
                break;
            };
            self.entries.remove(&victim);
        }
    }

    #[cfg(test)]
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MAX_ARTWORK_CACHE;

    fn texture(ctx: &egui::Context, name: &str) -> TextureHandle {
        ctx.load_texture(
            name,
            egui::ColorImage::filled([2, 2], egui::Color32::WHITE),
            Default::default(),
        )
    }

    #[test]
    fn eviction_is_lru_and_duplicate_uploads_do_not_evict_other_covers() {
        let ctx = egui::Context::default();
        let mut cache = Cache::default();
        for i in 0..MAX_ARTWORK_CACHE {
            cache.insert(i.to_string(), texture(&ctx, &i.to_string()));
        }
        cache.get("0");
        cache.begin_frame();
        cache.finish_frame();
        cache.insert("new".into(), texture(&ctx, "new"));
        assert!(cache.contains_key("0"));
        assert!(!cache.contains_key("1"));
        let count = cache.len();
        cache.insert("new".into(), texture(&ctx, "replacement"));
        assert_eq!(cache.len(), count);
        assert!(cache.contains_key("0"));
    }

    #[test]
    fn a_large_visible_working_set_is_stable_then_returns_to_the_warm_limit() {
        let ctx = egui::Context::default();
        let mut cache = Cache::default();
        let count = MAX_ARTWORK_CACHE + 24;
        cache.begin_frame();
        for i in 0..count {
            cache.get(&i.to_string());
        }
        cache.finish_frame();
        for i in 0..count {
            cache.insert(i.to_string(), texture(&ctx, &i.to_string()));
        }
        assert_eq!(cache.len(), count);
        // Queue/sidebar redraw order must not unpin covers from the main panel.
        cache.begin_frame();
        for i in (0..count).rev() {
            assert!(cache.get(&i.to_string()).is_some());
        }
        cache.finish_frame();
        assert_eq!(cache.len(), count);
        assert_eq!(cache.visible_len(), count);
        cache.begin_frame();
        cache.get("0");
        cache.finish_frame();
        assert_eq!(cache.len(), MAX_ARTWORK_CACHE);
        assert!(cache.contains_key("0"));
    }
}
