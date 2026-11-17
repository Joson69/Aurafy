# Aurafy 0.7.0 first-run checklist (Windows 11)

## Install the build tools

This release pins both `eframe` and `egui` to **0.33.3**. The pin is intentional: egui 0.34 changed the application entrypoint from `App::update` to `App::ui` and reworked the top-level UI API. Do not change these versions without porting the application code.

1. Install Rust stable from https://rustup.rs/.
2. Restart PowerShell / Windows Terminal.
3. Confirm `rustc --version` reports 1.90 or later and `cargo --version` works.
4. Double-click `RUN_AURAFY.bat` to build and launch Demo Mode.

Closing the window keeps music playing in the Windows tray. Click the Aurafy tray icon (it may be under the tray's up arrow) or launch Aurafy again to reopen it. Right-click the icon for playback controls and **Quit Aurafy**. To exit when closing instead, disable **Settings → General → Keep running when I close Aurafy**.

## Set up Spotify

1. Visit https://developer.spotify.com/dashboard.
2. Create a Spotify app.
3. Add this Redirect URI exactly:

   `http://127.0.0.1:8888/callback`

   Also add `http://127.0.0.1:43821/callback`. This lets Aurafy use port 43821 when 8888 is busy. Keep both entries.

4. Copy the app's Client ID.
5. In Aurafy, open Settings → Account → Connect your Spotify account, paste the Client ID, and click **Save and connect Spotify**.
6. Approve the permissions in your browser. If it does not open, click **Open sign-in page** or **Copy sign-in link** and paste the link into your browser. Keep Aurafy open until sign-in completes. Use **Cancel sign-in** before starting a fresh attempt.
7. Use the device button in the player to see and select your Spotify Connect outputs. Aurafy is selected on startup.

Already connected? Your saved session is reused. Click your avatar for Profile. The player’s device button chooses the output. Settings contains General, Listening & sound, Appearance, Privacy, and Account. Developer connection details stay collapsed in Account.

Aurafy selects this computer and resumes music automatically on opening. To start paused instead, turn off **Resume music when Aurafy opens** in Settings → General.

Starting paused still selects Aurafy as the output. Press the main Play button to listen; choosing a playback device first is unnecessary.

After closing Aurafy, reopen the same executable. Your last page, search, song, and position are restored while Spotify reconnects. Press Play to resume the existing session. Selecting a song from the recent-search popup plays it directly; the × removes that history entry.

## Enable audio on Aurafy

Generate a personal key at https://developer.spotify.com/dashboard/soloist. In Settings → Account → Local player connection, enter it into the masked **Spotify Soloist API key** field and click **Save key & enable playback**. The key is stored in Windows Credential Manager.

The official playback engine is downloaded if needed. Once it starts, choose **Aurafy** in Spotify's device picker on your phone or desktop once to pair it. Keep both devices on the same local network for initial pairing. You can then transfer playback from the phone to Aurafy or back. Select Aurafy in the player’s device picker to bring a remote session here without selecting a new song.

Version 0.5.2 lets the native player restore its own saved session. Wait for verification, then use Play normally. **Reconnect player** retries without clearing the connection. **Settings → Account → Local player connection → Connection troubleshooting → Reset saved pairing** deliberately disconnects it and requires another phone selection; use this only when you intend to re-pair.

Do not paste a Client Secret into Aurafy. OAuth uses PKCE. On Windows, Aurafy stores the refresh token in Windows Credential Manager.

## Spotify account and API caveats

Spotify's Development Mode currently requires Premium for the app owner and limits new apps to five authorized users. Playback commands require Premium and an available Spotify Connect device. Some playlist entries may expose metadata only, because Spotify limits item retrieval to playlists you own or collaborate on. Spotify may change these requirements; consult the official migration guide linked in the README.

## Build a release

Double-click `BUILD_RELEASE.bat`, or run `cargo build --release`. The executable is `target\\release\\aurafy.exe`.

You can also push this folder to GitHub and use **Actions → Build Aurafy for Windows → Run workflow**. The workflow uploads a Windows x64 executable artifact.


**Demo data note:** Sample songs and playlist labels are built-in preview content. They are not read from your Spotify account.

**Sign-in troubleshooting:** Aurafy 0.4.0 opens the registered default browser directly and keeps errors visible in Settings → Account. A manual sign-in link remains available for the duration of the attempt, even if automatic browser launch fails. Register both Redirect URIs above if Spotify shows an invalid redirect error. If port 8888 is unavailable, Aurafy automatically tries 43821. If both are unavailable, close another Aurafy instance or the app using those ports.
