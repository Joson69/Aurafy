# Settings feature checklist

Reference: the supplied Spotifast Playback and Appearance screenshots. This records implementation status, not feature parity.

| Feature | Aurafy status |
| --- | --- |
| Audio quality | Normal, High, Very high, Lossless requested through Soloist startup prefs. Quit and reopen to apply; the engine does not expose delivered-format verification. |
| Volume normalization | On/Off through Soloist startup prefs. |
| Background playback | Close to tray or quit; explicit tray Quit. |
| Windows startup | Off, window, or tray; per-user startup registration. |
| Resume at launch | Independent of the default local device, preserves saved position. |
| Playback cache | Existing bounded native audio cache setting. No disk artwork cache added. |
| Theme | Dark, Black, Light, Follow system. |
| Album-art colours | Optional tint using already decoded cover pixels. |
| Compact layouts | Separate sidebar and song-list options. |
| Interface zoom | 80, 90, 100, 110, 125 percent, plus Reset. |
| Title bar | Custom Aurafy controls or Windows decorations. |
| Tray style | Colour, White, Black; same persistent tray identity. |
| Refresh rate | 15, 30, 60 fps for smooth progress; idle/background stays quiet. |
| Sleep timer | Duration or end of track. |
| Lyrics | Size and automatic-follow preference. |
| Search privacy | History saving and clear history. |
| Seek / volume shortcuts | Configurable steps. |
| Device name | Configurable Spotify Connect device name in Settings → Listening & sound (defaults to `Aurafy`, up to 48 non-control characters). |
| Autoplay / gapless / crossfade | No new controls: a reliable configuration path for all real Soloist playlist/album contexts has not been established. |
| Visualizers / mini-player | Not implemented; need an audio-data path and separate window lifecycle. |
| Local files | Not implemented; needs decoding and queue integration. |
| Language | English, Spanish, French, German, Japanese, and Korean interface localization with startup OS CJK fallback fonts. |
| Proxy | No app-level custom proxy controls. |
| Automatic updates | Manual GitLab Releases check in Settings → Account targeting `aurafy-project1/aurafy-downloads` (gracefully handles private repos without tokens). |

# Home top music

Spotify's official `/me/top/tracks` and `/me/top/artists`, scoped with `user-top-read`. Four-week, six-month and one-year affinity rankings; twenty items per type. Results stay in memory, per signed-in session, with stale-response rejection. This is not exact listening-time, play-count, calendar-month or Wrapped data.

Sources: [Spotify top items](https://developer.spotify.com/documentation/web-api/reference/get-users-top-artists-and-tracks), [Soloist WebSocket reference](https://developer.spotify.com/documentation/soloist/reference/websocket-api).
