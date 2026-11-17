# Aurafy 0.8.0

Aurafy is a Windows-first native desktop music client built with **Rust + egui**. Version 0.4 redesigns the interface around a persistent search field, clear song lists, your listening history, and your actual playlists. Profile and settings have their own pages.

Aurafy is an independent project and is not affiliated with Spotify. Spotify is a trademark of Spotify AB.

### Framework compatibility

This release pins `eframe` and `egui` to `0.33.3`. egui 0.34 introduced a breaking application/UI API change (`App::update` was replaced by `App::ui`, and top-level panels now use a `Ui` entrypoint). The app code in this release targets 0.33.3 intentionally; keep both versions pinned together unless the UI is explicitly ported.

Aurafy was previously named Auralis. Version 0.7.0 introduced the Aurafy name. This public source uses Aurafy's equalizer icon. Existing settings, Windows credentials and native pairing remain in their original Auralis storage locations so upgrading keeps your connection and preferences. The old `RUN_AURALIS.bat` forwards to `RUN_AURAFY.bat`.

## New in 0.8.0

- Home leads with playlist shortcuts, followed by **Your top music** and **Recently played**. Top tracks and artists use Spotify's official affinity rankings for the last four weeks, six months or year. Twenty results per type/range are kept in memory for the session; there is no new disk history or artwork cache. Existing accounts can grant `user-top-read` through Home's Allow access button without resetting native playback pairing. These are not exact play counts, calendar-month statistics or Wrapped predictions.
- Appearance: Dark, Black, Light and Follow system themes; optional album-art colours; 80–125% interface zoom; compact library sidebar; and 15/30/60 fps for smooth playback progress. Paused/background playback stays on the quiet refresh path.
- General: Colour, White and Black tray icons, and a switch between Aurafy's custom title bar and the Windows title bar. Existing settings and native identity are preserved.
- See [SETTINGS_FEATURES.md](SETTINGS_FEATURES.md) for the feature checklist, including remaining work from the reference screenshots.

## New in 0.7.6

- Listening & sound now includes streaming quality (Normal, High, Very high and Lossless) and volume normalization. The selected preferences apply when Aurafy next starts its local player; existing installs keep their current audio configuration until a choice is made.
- These controls use Soloist's startup preference stores, following the independently measured Music Assistant integration. The public WebSocket API still does not report delivered quality: selecting Lossless requests that tier when Spotify makes it available, rather than asserting that every track is streaming losslessly. Premium and account/catalogue availability apply.
- Remove the repeat explanation below the queue heading. The player button retains repeat mode feedback.

## New in 0.7.5

- The native queue requests all entries available from Spotify instead of only 20. Repeat cycles are shown once using stable occurrence IDs, while intentional duplicate songs and manual additions are preserved. Short broadcast notifications trigger a full snapshot; only visible queue rows are rendered. Spotify may expose only part of very large contexts or a limited remote-device queue; Aurafy does not invent entries or change playback order.
- Settings → General: optional **Start Aurafy with Windows** and **Start in the tray** for Windows sign-in. These controls manage only Aurafy's per-user startup entry, require no administrator privileges, and leave automatic music resume independent. Manual launches still open the window.
- Settings → General: configurable seek steps (Alt Left/Right) and volume steps (Ctrl Up/Down), active when not typing in a text field.
- Settings → Appearance: lyrics text size and automatic following on open/song change. Scrolling freely and clicking timed lines still work.
- Soloist supports lossless audio. Aurafy configures the requested quality and normalization through the engine preference stores; the local API cannot verify the delivered format. Crossfade is not offered because the reviewed implementation does not reliably crossfade real playlist/album contexts.

## What works in this build

- Playlist and album row playback retains its Spotify collection context and original row offset, so the next track follows the collection rather than single-song autoplay. Playlist pages have a Play/Pause button, and Home/Library playlist cards expose one on hover or while playing.

- Native desktop UI and optional native Spotify playback through the official Soloist engine.
- Aurafy equalizer icon for the executable, window, taskbar, and Windows tray.
- Close to the tray to keep music playing. Click its icon or launch Aurafy again to reopen the same window. Right-click for playback controls and **Quit Aurafy**; `Ctrl+Q` also quits. Change close behavior in **Settings → General**.
- Home, Search, Library, Liked Songs, Recently Played, Profile, and Settings pages.
- Search while typing after a 450 ms pause, Enter for immediate search, `Ctrl+K` / `Ctrl+L` to focus, and recent-search suggestions when the field is empty.
- Recent searches retain the songs you selected, with artwork, artist names, direct playback, and individual removal.
- Reopening restores the last page, search, song, and position. Configured accounts show their saved music or a loading state while reconnecting; demo songs never substitute for their playback state.
- Borderless black player with white transport controls, compact artwork and text, a centered seek bar, and an optional queue panel. The 80px bar uses a 30px Play/Pause button with 14px top clearance, timeline at 76% height, and track information and utilities centered at 50%. Row coordinates follow the actual container; horizontal spacing remains unchanged.
- One save button opens your owned playlists and Liked Songs. Text reserves room for both title and artist, so neither runs underneath the button. Song options remain available in that menu or by right-clicking the save button.
- Fullscreen control beside volume; F11 toggles fullscreen and Escape exits it.
- Volume follows dragging immediately, coalesces updates for slower connections, and keeps incoming state updates from moving the thumb during a gesture.
- Embedded Lato typography for player metadata and bolder, higher-contrast synced lyrics. Font credits and the full license are available under Settings → Account.
- Top-right account menu for Profile, Settings, Account, background close, automatic resume, Log out, and Quit.
- Artist names in search results, recent searches, and the player open an artist view with songs, albums, singles, and more releases on demand. Song lists expand from actual releases when artist search returns only a few results.
- Clicking the player's song title opens its album with the complete track list. Artist and album IDs are preserved from Spotify metadata; older saved tracks resolve their album from their track ID. Browsing keeps playback and search text intact.
- Profile name and avatar from Spotify. Settings has General, Listening & sound, Appearance, Privacy, and Account tabs. Connection setup stays under Account.
- Persistent compact-list, queue, and smoother-progress preferences. Home shortcuts open their corresponding views; playlist filtering and per-song menus work.
- Built-in demo catalogue and simulated transport state before Spotify is configured. Sample titles and playlist names are hard-coded preview data, not imported from your Spotify account; Demo Mode does not output audio.
- Spotify OAuth Authorization Code with PKCE; no Client Secret is embedded.
- Spotify catalogue search with two API pages (up to 20 tracks per search, using Spotify's current Development Mode page size).
- Recently played tracks and profile greeting, with duplicate recent tracks removed from the home grid.
- Saved tracks: load, save, and remove from the library.
- Playlists: list playlists, open accessible playlists, create a playlist, and add a track to a playlist.
- Compact Spotify Connect device picker above the player, with refresh, current-output status, and phone/computer icons. Selecting an output keeps the current page open.
- Remember window size, position, and maximized state across restarts; preserve normal restore bounds while maximized or minimized.
- Spotify Connect device discovery and device transfer.
- Local audio output under the **Aurafy** Connect device name. Pair once, then transfer playback between Aurafy, your phone, and other Spotify devices.
- Native session verification on startup, direct local selection, and a local device row even before Spotify's Web API lists it. Routine device discovery stays quiet and pauses while the local player is active.
- Playback controls: play, pause, next, previous, seek, volume, shuffle, repeat, and add to queue.
- Playback state and queue synchronization from Spotify.
- Album and playlist artwork loaded off the UI thread, resized to 256 × 256, and held in a bounded in-memory texture cache.
- Blocking network work runs on worker threads so the UI thread remains responsive.
- Windows refresh-token storage through Windows Credential Manager. Sign-in opens the registered default browser directly. Settings also offers a copyable sign-in link, cancellation, and persistent errors.

## Music and app preferences

- **General:** reopen the last view, Home, Search, or Library; choose automatic music resume and background-close behavior.
- **Listening & sound:** 15–120 minute sleep timers or end-of-song pause; remember local volume or choose a launch level; set the native disk cache to 100, 250, 500, or 1000 MB; open Windows sound settings.
- **Appearance:** compact lists, artwork visibility, album columns, queue visibility, reduced animations, and smoother progress.
- **Privacy:** stop saving local searches or clear them. Turning history off clears Aurafy's saved queries and selected search songs, including the last restored query. It does not erase Spotify's listening history or library.
- **Account:** sign-in and native-player connection repair, with technical setup collapsed once connected.

Sleep timers operate in the tray, pause without activating another device, and reset when Aurafy quits. Launch volume/cache changes apply on the next app launch. Spotify's native engine does not expose quality, normalization, crossfade, or EQ preferences through its current public interface.

## Play on this computer

1. Generate your own [Spotify Soloist API key](https://developer.spotify.com/dashboard/soloist). Spotify requires Premium to generate it. This is separate from the Client ID used to sign in to Aurafy.
2. In **Settings → Account → Local player connection**, paste the key into the masked field and click **Save key & enable playback**. Aurafy stores the key in Windows Credential Manager. It does not put the key in config.json, source files, or logs.
3. Aurafy downloads Spotify's official Windows playback engine if needed. Keep your phone on the same local network and select **Aurafy** in Spotify's device picker once to pair it.
4. Aurafy is selected on startup. Use the device button in the player to switch to your phone or back to Aurafy. An explicit phone handoff is respected.

Local playback runs through one native Spotify process and a loopback WebSocket. Closing the window keeps Aurafy and its player running in the Windows tray by default. **Quit Aurafy** stops the engine; its paired session is retained for the next launch. When local playback is active, controls and state updates use the native engine. Other devices are controlled through the Web API.

On Windows the native player state is stored in `%USERPROFILE%\.auralis\player`, outside AppData redirection by packaged host applications. Existing state is copied once, preserving the Spotify device identity and retaining the old files. Aurafy selects this computer once the saved session becomes ready on startup; a later phone handoff is respected. Settings shows the running app version for troubleshooting.

Music resumes automatically when Aurafy opens, using the current native context and saved position. Turn off **Settings → General → Resume music when Aurafy opens** to start paused. Automatic selection/resume runs once at startup; it does not take playback back after you switch to your phone while Aurafy remains open.

Aurafy stays the selected output even with automatic resume disabled. Pressing the main Play button activates the native player directly. A paused receiver releasing its active Spotify connection does not select another output; an explicit remote choice or confirmed playback on another device does.

Normal startup preserves the engine's saved connection and verifies it before accepting local playback commands. **Reconnect player** retries without resetting that connection. **Settings → Account → Local player connection → Connection troubleshooting → Reset saved pairing** intentionally requires connecting from Spotify again; it is not a normal startup step. Session-management patterns follow [Spotifast's implementation](https://github.com/crmne/spotifast/blob/58ec45608a5a2bb7f18c08afd63e4b931d2f6ba0/src/soloist/mod.rs); attribution is in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Soloist is downloaded from [Spotify's official download service](https://developer.spotify.com/documentation/soloist/reference/downloads-and-updates); its binaries must not be redistributed with Aurafy. Each user supplies their own key. Spotify builds expire after 90 days; replace an expired engine with a fresh official build. The in-app startup error explains when an update is required.

Spotify's official Player API requires Premium for playback control and an active Connect device. Aurafy does not bypass that requirement.

## Window, collection and playlist controls

Aurafy draws its own unified header with branding, search, profile, minimize, maximize/restore and close controls. Drag empty header space to move the window; double-click it to maximize or restore. Border/corner handles resize the window. Close still follows the existing close-to-tray preference; Ctrl+Q quits. Window bounds, maximized state and native playback session remain preserved.

Home, Search and Your library stay fixed in the sidebar. The Collection heading, Liked Songs, Recently played and playlists share one scroll area.

The player's single save button opens Liked Songs and the playlist picker; song-list menus also provide **Add to playlist**. A compact, anchored menu shows only playlists whose Spotify owner ID matches the signed-in user's ID, with covers, song counts, search and **New playlist**. Followed/shared playlists owned by another user stay in the library and are excluded from this menu. Metadata pagination includes owned playlists beyond the first response page. Pending writes prevent duplicate submits; errors retain the selected song for retry. Confirmed adds receive checkmarks and remain visible so you can choose another playlist; **Done**, Escape or clicking outside closes the menu. Checkmarks reflect additions confirmed in Aurafy or positive membership already observed in loaded playlist items; opening the menu does not scan the contents of every playlist. The player is 80 pixels tall, with 50-pixel covers, a 30-pixel play button and one utility row. Output information is available on the device button, with a selected-output dot. Liked Songs updates only after confirmation; failed saves/removals retain the prior state, and stale library snapshots cannot undo newer writes.

Library saves/removals send the required `uris` query parameter to `PUT`/`DELETE /me/library`. Playlist adds continue to send a JSON `uris` array to `POST /playlists/{id}/items`. Empty/local/invalid song URIs are rejected before sending a write.

## Artwork loading

Album, song, playlist and profile covers load only when their artwork intersects the visible UI. Scrolling a large library or queue does not download every clipped row. A shared least-recently-used texture cache protects visible covers across all panels and asynchronous uploads, so new artwork cannot replace an onscreen image with its placeholder. The warm-cache target remains 72 textures at 256 × 256; unusually large views retain their visible working set, then return to the target when those covers leave view. Downloads remain limited to four in flight. The artwork visibility preference still releases cover textures while retaining the avatar.

## Lyrics and player

The microphone button beside the queue button opens lyrics inside Aurafy. Click it again, or use **Close lyrics**, to return to the previous page without losing your search. Synced lyrics highlight and follow the current line; click a timed line to seek. Scroll to read freely, then use **Follow song** to return to the current line. Plain lyrics and instrumental tracks have their own states. Missing lyrics and connection failures offer an explicit retry.

Lyrics are provided by [LRCLIB](https://lrclib.net/docs), not Spotify's private lyrics endpoints. Opening the lyrics view sends the current song's title, primary artist, album and duration to LRCLIB without Spotify credentials. Lookups happen on demand, run off the UI thread, and use a bounded in-memory cache with one request in flight. Server backoff is respected. Availability and timing depend on the provider; no additional API key is required.

The player uses balanced side sections, a centered transport and seek bar, aligned utility buttons and output caption, and clickable artist credits. The volume icon mutes and restores the previous volume; the repeat icon identifies repeat-one mode. Native seek, shuffle, repeat and volume also work while OAuth is reconnecting.

## Spotify Development Mode requirements

Spotify's February 2026 migration introduced tighter Development Mode requirements and API changes. At the time of writing:

- The app owner needs an active Spotify Premium subscription.
- New Development Mode apps are limited to five authorized users.
- Search requests accept a maximum `limit` of 10; Aurafy paginates in two pages.
- Saving/removing tracks uses `PUT` / `DELETE /me/library` and Spotify URIs.
- Playlist item retrieval and modification use `/playlists/{id}/items`.
- Spotify only returns playlist items for playlists the user owns or collaborates on; other playlists may appear as metadata-only entries.

Always check Spotify's current developer documentation before distributing or relying on the integration. Spotify can change access, endpoint availability, scopes, and account requirements independently of Aurafy.

Official references:

- https://developer.spotify.com/documentation/web-api/tutorials/february-2026-migration-guide
- https://developer.spotify.com/documentation/web-api/references/changes/february-2026
- https://developer.spotify.com/documentation/web-api/concepts/authorization
- https://developer.spotify.com/documentation/web-api/concepts/quota-modes

## Windows 11 setup

### 1. Install Rust

Install the Rust stable toolchain from https://rustup.rs/. Restart PowerShell after installation. This build requires Rust 1.90 or later, including its Windows tray library.

Verify:

```powershell
rustc --version
cargo --version
```

### 2. Run the demo

Double-click `RUN_AURAFY.bat`, or open PowerShell in this folder and run:

```powershell
cargo run --release
```

The first run downloads and compiles the dependencies. The app can be explored in Demo Mode without Spotify credentials.

### 3. Connect your Spotify account

1. Open https://developer.spotify.com/dashboard and create a Spotify app.
2. In the app's settings, add this exact Redirect URI:

   `http://127.0.0.1:8888/callback`

   Also add `http://127.0.0.1:43821/callback`. Aurafy uses port 43821 if 8888 is occupied. Keep both entries. Both are loopback addresses; no public callback server is needed.

3. Copy the **Client ID** (not a Client Secret).
4. Open Aurafy → Settings → Account → Connect your Spotify account, paste the Client ID, and click **Save and connect Spotify**.
5. Approve access in your browser. If no browser opens, use **Open sign-in page** or **Copy sign-in link** in Settings and paste the link into your browser. Keep Aurafy open. The link expires when the attempt ends (after three minutes or when cancelled).
6. Open Spotify on a device, then select it using the device button in Aurafy’s player. Sign-in errors remain visible in Settings → Account. Use **Cancel sign-in** to end a stuck attempt, then connect again. If port 8888 is occupied, Aurafy tries 43821 automatically. Register both URIs above if Spotify rejects the redirect. If both ports are occupied, close another Aurafy instance or the app using those ports.

The app uses PKCE; do not add a Client Secret to the project. On Windows, the refresh token is stored in Windows Credential Manager. The Client ID and recent searches are stored under your Windows user config directory in `Auralis/config.json`.

### 4. Build the executable

Double-click `BUILD_RELEASE.bat`, or run:

```powershell
cargo build --release
```

The executable will be:

```text
target\release\aurafy.exe
```

## Build the Windows executable with GitHub Actions

To enable the Windows build on GitHub, copy [the workflow template](.github/build-windows.yml.example) to `.github/workflows/build-windows.yml`. Then open **Actions → Build Aurafy for Windows → Run workflow** and download the `aurafy-windows-x64` artifact after the workflow finishes.

The workflow is a build helper, not an installer or code-signing service. Windows SmartScreen may show an unsigned-app warning for a personal build.

## Performance design

- Rust + egui native rendering instead of Electron/Chromium.
- Release profile uses size optimization, LTO, one codegen unit, and symbol stripping.
- UI idles at a low repaint rate; playback progress updates at 10 FPS by default. Settings → Appearance offers an optional 30 FPS mode.
- Network work is kept off the UI thread.
- Spotify API requests share an HTTP connection pool. Credential Manager is updated when the refresh token changes, rather than on every player poll.
- Artwork shares the HTTP connection pool and downloads are bounded before decoding. Queue API polling stops while the queue is hidden.
- Search results and long track lists use virtualized scrolling.
- Artwork requests are size-capped, limited to four concurrent downloads, decoded off-thread, resized to 256 × 256, and limited to a 72-texture in-memory cache.
- No JavaScript runtime or web view is used. Optional local playback uses Spotify's native engine, a loopback-only WebSocket, and a 100 MB audio cache cap.

RAM and frame time depend on the system, driver, display scaling, and number of visible artwork textures. No benchmark figures are claimed until measured on a real Windows machine.

## Known limitations

- Local audio requires a separately generated Soloist API key and one-time Spotify Connect pairing. Without local playback enabled, Aurafy controls another Spotify device.
- Spotify's API only exposes playlist contents for playlists the current user owns or collaborates on under the current Development Mode rules.
- The saved-tracks list and playlist list are limited to the first API page in this build; an opened playlist loads up to 150 tracks.
- Artwork is cached in memory for the current session, not persisted to disk.
- There is no installer, automatic updater, code signing, or cross-platform QA yet.
- Automated callback tests cover local HTTP requests; signing in to your actual account still requires approval in your browser.

## Verification

Run `cargo test --release --locked` for the callback regression tests. They use temporary loopback ports and make no requests to Spotify. To check browser launching manually, run `cargo test --release --locked opens_spotify_dashboard -- --ignored`; this opens your default browser.

The test suite also checks UI navigation, old-settings compatibility, nullable Spotify profile/local-track fields, and sign-out handling of in-flight requests. It renders the actual egui views offscreen at 1440×900 and 1080×680 into `target/ui-previews/` for visual review. These previews use deterministic test data and artwork; they do not read your account or credentials.

`RUN_AURAFY.bat` checks for changed source and builds it before launching, so it does not silently run an older executable. Both batch scripts also locate Rust in its standard user installation directory if the current terminal has an outdated PATH.

## Project layout

```text
Aurafy/
├── .github/build-windows.yml.example
├── Cargo.toml
├── BUILD_RELEASE.bat
├── FIRST_RUN.md
├── PATCH_NOTES.md
├── LICENSE
├── README.md
├── RUN_AURAFY.bat
└── src/
    ├── main.rs       # Native UI, screens, artwork cache, player and app state
    ├── ui.rs         # Native layout, vector icons, profile, settings and lists
    ├── ui_preview.rs # Test-only offscreen layout rendering
    ├── local_player.rs # Native Spotify playback process and Connect events
    ├── browser.rs    # Shared default-browser launcher
    └── spotify.rs    # OAuth PKCE, Web API, library, playlists and Connect controls
```
