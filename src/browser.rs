/// Use the registered browser directly, preserving the entire OAuth URL as one
/// argument. Starting Explorer only confirms that Explorer started, not that a
/// browser was found. This library is already included by egui's link support.
pub fn open_url(url: &str) -> Result<(), String> {
    webbrowser::open(url).map_err(|error| {
        format!("Could not open your default browser: {error}. Copy the sign-in link into your browser, or check your default browser in Windows Settings.")
    })
}

pub fn open_sound_settings() -> Result<(), String> {
    webbrowser::open("ms-settings:sound")
        .map_err(|error| format!("Could not open Windows sound settings: {error}"))
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "Opens Spotify Developer Dashboard in the system default browser"]
    fn opens_spotify_dashboard() {
        super::open_url("https://developer.spotify.com/dashboard").unwrap();
    }
}
