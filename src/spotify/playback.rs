//! Context starts use the Web API: Soloist's play-URI command has no row offset.
use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextStart {
    pub uri: String,
    pub position: usize,
}

fn request_context(
    client: &Client,
    base: &str,
    access: &str,
    start: &ContextStart,
    device: Option<&str>,
) -> Result<RequestBuilder, String> {
    let valid = ["spotify:playlist:", "spotify:album:"]
        .iter()
        .any(|prefix| {
            start
                .uri
                .strip_prefix(prefix)
                .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()))
        });
    if !valid {
        return Err("This collection has no valid Spotify playlist or album URI.".into());
    }
    let mut request = client.put(format!("{base}/me/player/play")).bearer_auth(access)
        .json(&serde_json::json!({"context_uri":start.uri,"offset":{"position":start.position},"position_ms":0}));
    if let Some(device) = device {
        request = request.query(&[("device_id", device)]);
    }
    Ok(request)
}

pub fn start_context(
    access: &str,
    start: &ContextStart,
    device: Option<&str>,
) -> Result<(), String> {
    api_response(
        request_context(&http_client()?, API_BASE, access, start, device)?,
        "Could not start Spotify collection playback",
    )?;
    Ok(())
}

pub fn local_device_id(access: &str, device_name: &str) -> Result<String, String> {
    // Native activation and Web API discovery are asynchronous. Wait briefly for
    // this receiver; never fall through to a phone or another active client.
    let target_name = if device_name.trim().is_empty() {
        crate::local_player::DEVICE_NAME
    } else {
        device_name.trim()
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let devices = get_devices(access)?;
        let local: Vec<_> = devices
            .into_iter()
            .filter(|d| d.name == target_name)
            .collect();
        if let Some(active) = local.iter().find(|d| d.is_active) {
            return Ok(active.id.clone());
        }
        if local.len() == 1 {
            return Ok(local[0].id.clone());
        }
        if local.len() > 1 {
            return Err(format!(
                "More than one {target_name} device is available. Select this computer in the device picker, then try again."
            ));
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "Spotify hasn't made this {target_name} device available yet. Wait a moment, then play the playlist again."
            ));
        }
        thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn context_request_targets_the_collection_row_and_selected_device() {
        let client = Client::new();
        for (uri, position) in [
            ("spotify:playlist:MyList", 0),
            ("spotify:playlist:MyList", 113),
            ("spotify:album:MyAlbum", 4),
        ] {
            let request = request_context(
                &client,
                "http://127.0.0.1",
                "test-only",
                &ContextStart {
                    uri: uri.into(),
                    position,
                },
                Some("local-id"),
            )
            .unwrap()
            .build()
            .unwrap();
            assert_eq!(request.method(), reqwest::Method::PUT);
            assert_eq!(request.url().path(), "/me/player/play");
            assert_eq!(request.url().query(), Some("device_id=local-id"));
            let body: serde_json::Value =
                serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
            assert_eq!(
                body,
                serde_json::json!({"context_uri":uri,"offset":{"position":position},"position_ms":0})
            );
            assert!(
                body.get("uris").is_none(),
                "a playlist must not become a single-track request"
            );
        }
        assert!(
            request_context(
                &client,
                "http://127.0.0.1",
                "test-only",
                &ContextStart {
                    uri: "spotify:track:Song".into(),
                    position: 0
                },
                None
            )
            .is_err()
        );
    }
}

#[cfg(all(test, windows))]
mod live_tests {
    use super::*;
    use std::{os::windows::process::CommandExt, process::Command};

    fn native(args: &[&str]) -> Result<serde_json::Value, String> {
        let executable = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/tools/soloist/soloist.exe");
        let mut command = Command::new(executable);
        command
            .creation_flags(0x08000000)
            .arg("ctl")
            .args(args)
            .arg("--data-dir")
            .arg(crate::local_player::directory().join("data"));
        let output = command.output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("Native playback check failed".into());
        }
        if args.contains(&"--json") {
            serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    fn wait_for(uri: &str) -> Result<serde_json::Value, String> {
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            let state = native(&["now", "--json"])?;
            if state["item"]["uri"] == uri && state["status"] == "playing" {
                return Ok(state);
            }
            if Instant::now() >= deadline {
                return Err("Expected playlist row did not start in the native receiver".into());
            }
            thread::sleep(Duration::from_millis(300));
        }
    }

    #[test]
    #[ignore = "Plays the requested owned playlist on the live Aurafy receiver and seeks to verify the first-to-second transition"]
    fn live_playlist_context_smoke() -> Result<(), String> {
        let name = std::env::var("AURAFY_QA_PLAYLIST_NAME")
            .map_err(|_| "Set AURAFY_QA_PLAYLIST_NAME for this explicit live test")?;
        let config = crate::AppConfig::load();
        let refresh = load_refresh_token().ok_or("No saved account")?;
        let token = restore_session(&refresh, &config.client_id)?;
        if let Some(refresh) = token.refresh_token.as_deref() {
            save_refresh_token(refresh)?;
        }
        let profile = get_profile(&token.access_token)?;
        let lists: Vec<_> = get_playlists(&token.access_token)?
            .into_iter()
            .filter(|p| p.name == name && p.is_owned_by(&profile.id))
            .collect();
        if lists.len() != 1 {
            return Err("Expected exactly one matching owned playlist".into());
        }
        let playlist = &lists[0];
        let tracks = get_playlist_items(&token.access_token, &playlist.id)?;
        if tracks.len() < 3 {
            return Err("Need at least three playable playlist rows for this test".into());
        }
        let device = local_device_id(&token.access_token, crate::local_player::DEVICE_NAME)?;
        let first = &tracks[0];
        let second = &tracks[1];
        let third = &tracks[2];
        let context = format!("spotify:playlist:{}", playlist.id);
        start_context(
            &token.access_token,
            &ContextStart {
                uri: context.clone(),
                position: first.context_position.unwrap(),
            },
            Some(&device),
        )?;
        let start = wait_for(&first.uri)?;
        if start["context"]["uri"] != context {
            return Err("Native player lost the playlist context".into());
        }
        let queue = native(&["queue", "--json"])?;
        if queue["upcoming"][0]["item"]["uri"] != second.uri
            || queue["upcoming"][0]["source"] != "context"
        {
            return Err("Native queue does not continue with the second playlist row".into());
        }
        let end = first.duration_ms.saturating_sub(1500).to_string();
        native(&["seek", &end])?;
        let next = wait_for(&second.uri)?;
        if next["context"]["uri"] != context {
            return Err("Playlist context changed at the song boundary".into());
        }
        start_context(
            &token.access_token,
            &ContextStart {
                uri: context.clone(),
                position: second.context_position.unwrap(),
            },
            Some(&device),
        )?;
        let selected = wait_for(&second.uri)?;
        let queue = native(&["queue", "--json"])?;
        if queue["upcoming"][0]["item"]["uri"] != third.uri {
            return Err("Starting the second row did not retain the following row".into());
        }
        let report = serde_json::json!({"playlist":name,"first_song":first.name,"second_song":second.name,"next_after_second":third.name,"first_to_second_passed":true,"nonzero_offset_passed":true,"native_playlist_context":selected["context"]["uri"],"queue_source":queue["upcoming"][0]["source"]});
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/live-playlist-context.json");
        fs_write_report(&path, &report)?;
        println!("Playlist row 1 -> row 2, explicit row 2 start, and row 3 queue checks passed.");
        Ok(())
    }

    fn fs_write_report(path: &std::path::Path, report: &serde_json::Value) -> Result<(), String> {
        std::fs::write(path, serde_json::to_vec_pretty(report).unwrap()).map_err(|e| e.to_string())
    }
}
