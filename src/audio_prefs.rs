//! Startup-only preferences understood by the Soloist engine.
//! The local API cannot report the format actually delivered by Spotify.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Normal,
    High,
    VeryHigh,
    Lossless,
}
impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal · 96 kbps",
            Self::High => "High · 160 kbps",
            Self::VeryHigh => "Very high · 320 kbps",
            Self::Lossless => "Lossless",
        }
    }
    fn value(self) -> u8 {
        match self {
            Self::Normal => 2,
            Self::High => 3,
            Self::VeryHigh => 4,
            Self::Lossless => 5,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub quality: Option<Quality>,
    pub normalize: Option<bool>,
}

impl Settings {
    fn values(self) -> Vec<(&'static str, String)> {
        let mut values = Vec::new();
        if let Some(quality) = self.quality {
            values.push((
                "audio.play_bitrate_enumeration",
                quality.value().to_string(),
            ));
            values.push((
                "audio.play_bitrate_non_metered_enumeration",
                quality.value().to_string(),
            ));
            values.push(("audio.play_bitrate_non_metered_migrated", "true".into()));
        }
        if let Some(normalize) = self.normalize {
            values.push(("audio.normalize_v2", normalize.to_string()));
        }
        values
    }

    pub fn write(self, data: &Path) -> Result<(), String> {
        let values = self.values();
        if values.is_empty() {
            return Ok(());
        }
        let settings = data.join("settings");
        let mut files = vec![settings.join("prefs")];
        let users = settings.join("Users");
        if users.exists() {
            for entry in
                fs::read_dir(&users).map_err(|_| "Could not read player audio preferences")?
            {
                let entry = entry.map_err(|_| "Could not read player audio preferences")?;
                let kind = entry
                    .file_type()
                    .map_err(|_| "Could not read player audio preferences")?;
                if kind.is_dir() {
                    files.push(entry.path().join("prefs"));
                }
            }
        }
        // Prepare every store before writing any: invalid files must not be truncated.
        let mut updates: Vec<(PathBuf, String)> = Vec::new();
        for file in files {
            let original = if file.exists() {
                let metadata = fs::symlink_metadata(&file)
                    .map_err(|_| "Could not read player audio preferences")?;
                if metadata.file_type().is_symlink() || metadata.len() > 1_048_576 {
                    return Err("Player audio preference file is invalid".into());
                }
                fs::read_to_string(&file).map_err(|_| "Could not read player audio preferences")?
            } else {
                String::new()
            };
            let mut lines: Vec<String> = original
                .lines()
                .filter(|line| {
                    let key = line.split_once('=').map(|(key, _)| key).unwrap_or(line);
                    !values.iter().any(|(managed, _)| *managed == key)
                })
                .map(str::to_owned)
                .collect();
            lines.extend(values.iter().map(|(key, value)| format!("{key}={value}")));
            updates.push((file, lines.join("\n") + "\n"));
        }
        for (file, content) in updates {
            fs::create_dir_all(
                file.parent()
                    .ok_or("Invalid player preferences directory")?,
            )
            .map_err(|_| "Could not save player audio preferences")?;
            let temporary =
                file.with_extension(format!("aurafy-{:016x}.tmp", rand::random::<u64>()));
            fs::write(&temporary, content)
                .map_err(|_| "Could not save player audio preferences")?;
            if fs::rename(&temporary, &file).is_err() {
                let _ = fs::remove_file(&temporary);
                return Err("Could not replace player audio preferences".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quality_and_normalization_update_both_stores_without_touching_other_preferences() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/audio-prefs-tests")
            .join(rand::random::<u64>().to_string());
        let user = root.join("settings/Users/synthetic-user/prefs");
        fs::create_dir_all(user.parent().unwrap()).unwrap();
        fs::write(&user, "unrelated=value\naudio.crossfade_migrated_to_ps=true\naudio.play_bitrate_enumeration=2\naudio.normalize_v2=false\n").unwrap();
        let config = Settings {
            quality: Some(Quality::Lossless),
            normalize: Some(true),
        };
        config.write(&root).unwrap();
        for file in [root.join("settings/prefs"), user.clone()] {
            let text = fs::read_to_string(&file).unwrap();
            assert!(text.contains("audio.play_bitrate_enumeration=5\n"));
            assert!(text.contains("audio.play_bitrate_non_metered_enumeration=5\n"));
            assert!(text.contains("audio.play_bitrate_non_metered_migrated=true\n"));
            assert!(text.contains("audio.normalize_v2=true\n"));
            assert_eq!(text.matches("audio.play_bitrate_enumeration=").count(), 1);
        }
        assert!(
            fs::read_to_string(&user)
                .unwrap()
                .contains("unrelated=value\naudio.crossfade_migrated_to_ps=true")
        );
        Settings {
            quality: Some(Quality::VeryHigh),
            normalize: Some(false),
        }
        .write(&root)
        .unwrap();
        let text = fs::read_to_string(user).unwrap();
        assert!(
            text.contains("audio.play_bitrate_enumeration=4\n")
                && text.contains("audio.normalize_v2=false\n")
        );
    }
    #[test]
    fn existing_installs_are_untouched_until_audio_settings_are_selected() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/audio-prefs-tests")
            .join(rand::random::<u64>().to_string());
        Settings::default().write(&root).unwrap();
        assert!(!root.exists());
        assert!(serde_json::from_str::<Settings>(r#"{"quality":"unknown"}"#).is_err());
    }
}
