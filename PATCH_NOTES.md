## Passive repaint hotfix

- Performance/reduced-motion mode uses a once-per-second passive timeline refresh. Normal native clock ticks no longer force immediate repaints; state changes, seeks, volume changes and command confirmations remain immediate.
- Add frame count, UI-build timing and repaint-callsite diagnostics. Screenshot preview is now an explicit install verification option instead of running during every hotfix restart.
- A 15-second Home sample with smooth progress off measured 0.01% GUI CPU and 1.1 fps after the diagnostic capture finished. Interactive and smooth-mode load are not represented by that idle sample.

## Playlist editing hotfix

- Checked owned-playlist rows can remove a song, and owned playlist song menus expose Remove from this playlist. Pending writes block duplicate actions; membership changes only after confirmed success and remains retryable on failure. Counts and opened playlist rows refresh from Spotify.
- Use the current DELETE playlist items endpoint and object URI payload. Do not remove Liked Songs or alter the playing queue when editing playlist membership.
- Reserve a non-floating scrollbar gutter in the playlist picker so check/add controls stay clear.
- Validation: 126 automated checks pass, including request format, ownership rejection and removal success/failure state. Offscreen picker/error layouts reviewed. No real account songs were deleted for testing.

## Aurafy 0.8.0

- Replace the Home greeting and “Back in rotation” with plain section labels, compact playlist shortcuts and Spotify top music. Songs and Artists share an explicit period selector; cached results retain Spotify's ranking order. Missing scope prompts one additional OAuth approval. Failed requests require an explicit retry; signed-out/replaced sessions cannot refill private rankings.
- Add complete dark/black/light/system palettes, optional colour sampled from already loaded artwork, interface zoom, compact sidebar, playback refresh rate, tray colour variants and optional native Windows decorations. No dependency or artwork disk-cache expansion.
- Keep current queue, playback context/position, device selection, native credentials and startup/autoresume preferences.

# Aurafy 0.7.6 patch notes

- Add real requested-quality controls (Normal, High, Very high, Lossless) and volume normalization to Listening & sound. Save validated preferences and write the engine's global and existing per-user stores before receiver startup, after process ownership recovery. Update again after pairing creates a user store.
- Preserve unrelated preferences, migration markers and credentials. Use atomic file replacement; surface failures instead of claiming the audio configuration was applied. Existing defaults remain untouched until a control is selected.
- Explain restart timing and Premium/availability requirements at the controls. Do not show a delivered-lossless indicator: the engine provides no format telemetry.
- Remove the repeat explanatory subtitle from the queue.

Validation: 116 regression checks passed, including preference-file writes for both engine stores, safe upgrades with untouched defaults, loop occurrence handling, native playback controls and startup integration.

## Aurafy 0.7.5 patch notes

- Remove Aurafy's 20-entry native queue request limit. Read a complete `get_queue` snapshot with `limit: 0` on a separate RPC socket; short subscription broadcasts invalidate the snapshot instead of truncating it. Preserve duplicates and authoritative empty queues. Keep existing data on request failure and respect phone handoffs.
- Show one cycle of repeating contexts, using stable occurrence IDs rather than deduplicating song URIs. Keep legitimate repeated playlist rows and manual queue additions. Correct native repeat-one command ordering.
- Render only visible queue rows; avoid cloning the entire queue per frame.
- Add per-user Windows startup and startup-in-tray controls. Quote the actual executable path, modify only Aurafy's Run value, surface write errors, preserve window bounds, and keep music resume independent. Startup remains opt-in.
- Add configurable lyrics size/automatic following and keyboard seek/volume steps. Existing defaults and timed lyric clicks are retained.
- Explain the current audio-quality limitation: Soloist supports lossless but does not expose a public quality selector or delivered-format reporting.

Validation: 114 offline regression tests passed, including a 137-entry native queue with repeats, short-broadcast invalidation, preference migration, native seek/volume routing and an isolated registry round trip. A read-only check against the running native receiver returned 73 upcoming songs without changing playback.

## Aurafy 0.7.3 patch notes

- Start playlist and album rows using the collection URI plus the original row position, rather than a standalone song URI. Preserve offsets across unavailable playlist items, pagination and duplicate songs. Search/recent-song playback stays independent of previously opened collections.
- Target the selected Aurafy receiver explicitly for native collection starts. Native transport/state remain on Soloist; collection starts use Spotify's documented Web API context/offset request, as in Spotifast's Soloist integration. Activate the local receiver when needed and bound discovery retries.
- Add a round Play/Pause button to playlist headers and hovered/playing Home and Library cards. The active collection pauses/resumes; a new collection starts at its first row. Card playback does not navigate away.
- Refresh native context and queue state after collection starts.

Validation: 106 offline regressions passed. An explicit live test started faves row 1 (After Hours), verified row 2 (Faith) in the native context queue, advanced through the real track boundary, then started row 2 explicitly and verified row 3 (Blinding Lights) remained next. Existing restart, handoff, player geometry, artwork, library-write and privacy checks remain passing.

## Aurafy 0.7.2 patch notes

- Recover a leftover native player before starting a new one. Verify its recorded PID, executable path and creation time using a held process handle; preserve pairing data and leave unrelated/reused PIDs alone.
- Keep the selected local player's queue authoritative, including an empty queue. Do not replace it with Web API queue copies while local playback is selected. Preserve intentional duplicate entries and remote-device queues after handoff.
- Replay the displayed track when the native context is idle after completion. Keep ordinary paused resumes on the original context and position; do not restore an old end position onto a deliberate replay.
- Refresh the native queue after queue-affecting commands. Clear the previous local queue when its active device is released.

Validation: 102 regression tests passed, including verified process recovery, idle versus paused playback, actual player-button routing, queue ownership, and duplicate retention. Live installation reclaimed the leftover receiver and restored pairing without phone setup. Passive observation confirmed a real song completed and advanced to the expected next queued track, which was playing at about one second. Exhausted-context replay is covered by the player-button regression test; the live observation preserved the user's existing autoplay queue.

## Aurafy 0.7.1 patch notes

- Remove the full title/artist hover popup from queue songs. Keep row highlighting, artist navigation and double-click playback.

## Aurafy 0.7.0 patch notes

- Rename the app, Windows executable, tray menu and Spotify Connect device to Aurafy.
- Use Aurafy branding in the app header, executable, window, taskbar and tray. Package icon sizes from 16 to 256 pixels; load the small header texture once.
- Preserve existing Auralis settings, credentials, native player pairing, tray identity and single-instance behavior. The old launch script forwards to RUN_AURAFY.bat.
- Keep the approved 80-pixel player layout and 14-pixel play-button top clearance.

## Auralis 0.6.4 patch notes

- Replace the centered playlist dialog with an anchored, compact menu. Show artwork, song counts, search, New playlist, add/progress indicators and confirmed checkmarks. Keep the menu open after adds so another playlist can be chosen; close through Done, Escape or an outside click.
- Include only playlists owned by the signed-in Spotify user, comparing API owner IDs rather than display names or item-access flags. Recheck ownership before sending a write. Preserve followed/shared playlists in the normal library.
- Capture profile/playlist owner IDs, accept nullable owner descriptions/names and follow playlist metadata pagination. Owned playlists beyond the first page remain available without fetching every playlist's song contents.
- Reduce the player from 100 to 72 pixels, shrink covers from 54 to 44 pixels and the play button from 36 to 30. Retain artist navigation, transport, seeking, lyrics, queue, devices and mute/volume. Move the output caption into the device tooltip with an output indicator dot.

Validation: 83 automated checks pass (optional browser/live-lyrics checks excluded). Coverage includes strict owner-ID matching despite duplicate display names, nullable owner metadata, pagination past followed playlists, compact anchored menu bounds, foreign-write rejection, failed-add retry, confirmed checkmarks/duplicate prevention, 72-pixel player bounds across window sizes, and existing playback/window/lyrics/artwork/privacy regressions. UI previews cover the slim player and compact menu at wide and narrow sizes. The final menu-order adjustment also passed all 44 UI checks.

The final menu keeps playlist row order stable after confirmation so targets do not jump under the pointer.

## Retained changes from 0.6.3

- Fix Liked Songs saves/removals: Spotify requires `uris` in the `/me/library` query string. Keep playlist adds on the distinct JSON-body `/playlists/{id}/items` endpoint, and validate song URIs before writing.
- Commit saved state after confirmation; block duplicate pending writes, preserve prior state on failure, and refresh stale library snapshots instead of letting them overwrite newer saves.
- Add **Add to playlist** to the player's song menu. Make the picker filterable and scrollable; retain failed adds for retry, show progress, and close only on confirmed success. Preserve the same retry flow after creating a new playlist.
- Scroll the Collection heading, Liked Songs, Recently played and playlists together, while keeping primary navigation fixed.
- Integrate branding, search, profile and window controls into a single app header. Add header dragging, double-click maximize/restore and border/corner resizing. Retain close-to-tray, explicit quit, geometry persistence and playback behavior.

Validation: 79 automated checks pass (optional browser/live-lyrics checks excluded). Coverage includes loopback HTTP validation of the actual save/remove/playlist request builders; confirmed/error/duplicate library state; stale snapshots; playlist retry/confirmation and player-menu entry; collection scrolling; caption commands, header dragging and corner resizing; existing native playback, handoff, artwork and session regressions. Real egui renders cover wide/narrow integrated headers and the playlist picker/error states. No test song was added to the live Spotify account.

## Retained changes from 0.6.2

- Fix cover flashing across the player, queue, sidebar, library and other views. The old cache evicted an arbitrary image at capacity, including visible images, while clipped rows continued requesting new artwork.
- Request and retain only visible artwork. Protect the visible working set across frame boundaries and background uploads, and evict the least recently used offscreen textures. Updating an existing URL no longer evicts an unrelated image.
- Keep the 72-texture warm-cache target and four-download concurrency limit. If a large view needs more visible covers, retain them until they leave view instead of cycling them. Add aggregate cache/pending counts to the existing local runtime diagnostics; no artwork URLs are recorded there.

Regression coverage reproduces both old failures with generated covers: a 200-playlist/200-track view stops downloading once its visible rows settle, and new image completions never evict a visible song/playlist. Additional checks cover LRU order, duplicate uploads, a working set larger than the cache target, and releasing pins after leaving that view.

Validation: all 72 automated checks pass (two optional browser/live-service checks excluded). Both new UI regressions were first observed failing on the prior cache behavior. Existing lyrics, native playback, phone handoff, window restoration, privacy, settings and rendered-view checks remain passing.

## Retained changes from 0.6.1

- Add an in-app lyrics view, opened by the microphone button beside the queue. Support synced highlighting and following, click-to-seek, manual scrolling, plain lyrics, instrumental tracks, loading, missing-lyrics and retry states. Closing lyrics restores the previous page.
- Use LRCLIB's documented public API on demand, without Spotify credentials. Bound responses and the memory-only cache, serialize lookups, ignore stale song responses in the view, and honor server backoff. No extra dependency or API key is required.
- Balance player side reservations, center transport and timeline, align utility controls and output caption, add mute/unmute with volume restoration, and indicate repeat-one mode. Retain artist navigation and the compact device picker. Native secondary controls remain available during OAuth restoration.

Validation: 68 automated checks pass, including native seek/mute controls, paused-position lyric highlighting, stale-response handling, HTTP fallback/backoff, existing playback/session/window regressions, and rendered layouts at 1080, 1193, 1440 and 1920 pixels. The optional live LRCLIB lookup also passed with synced lines; the browser-opening smoke check was not run. UI lyric previews contain original fixture text.

## Retained changes from 0.6.0

- Replace the redundant Playback settings page with General, Listening & sound, Appearance, Privacy, and Account. Device selection and current volume stay in the player. Move connection setup and troubleshooting under Account.
- Add session sleep timers (15–120 minutes or end of the current song), cancellation, and an avatar-menu shortcut. Timers work in the tray, pause once, respect seeking/repeat, and never activate Auralis just to pause it.
- Remember local volume separately from phone volume, or use a chosen launch level. Apply native disk-cache limits of 100/250/500/1000 MB at the next launch. Link to Windows sound settings.
- Add startup-page choice, artwork visibility with cover-memory release, album-column visibility, and reduced motion. Retain compact lists, queue visibility, smooth progress, automatic resume, and background-close preferences.
- Add local search-history privacy and clearing. Disabled history neither records new queries/selected songs nor restores or migrates prior searches. Playback bookmarks and the Spotify library remain available.
- Keep unsupported native quality/EQ/crossfade knobs out of the UI.

Validation: 60 automated checks pass (browser-launch smoke check excluded). Coverage includes real settings clicks, one-shot tray timers without device activation, end-of-song/repeat/seek behavior, privacy persistence, independent startup/output preferences, artwork-cache release, native volume/cache arguments, and existing sign-in/playback/window/device regressions. Native UI previews cover all settings categories and a narrow window.

Installed-build verification: 0.6.0 reopened with the native session paired, the tray registered, the exact previous window bounds retained, and launch volume matching the pre-upgrade native volume.

## Retained changes from 0.5.9


- Open a compact device picker above the player button instead of navigating to Settings. Show the selected output, real available devices, refresh, and appropriate phone/computer icons; keep device discovery quiet.
- Save Windows normal restore bounds and maximized state. Restore the same workspace coordinates on launch, debounce saves after moving/resizing, and retain bounds while hidden in the tray. Windows recovers completely offscreen positions after display changes.

Validation: 53 automated checks passed (optional browser-launch test excluded). The picker click test checks its size/anchor and native transfer without page navigation. Disposable native Windows windows verify exact position/size, maximization, serialization, and restoration to a recreated window. UI previews cover the picker at 1440×900 and 1080×680.

Installed-build verification: reopening the actual app restored the exact pre-upgrade Windows bounds [278,127,1498,925] (1220×798 including the frame). Restore waits for the first native show event, which otherwise shifted the window after early restoration. The native player stayed paired and the tray remained registered.

## Retained changes from 0.5.8


- Move profile, settings, account, logout, and quit into the top-right avatar menu; remove the duplicate sidebar footer and separate settings gear.
- Put background-close and automatic-resume preferences directly in that menu, retaining their full settings controls.
- Center the transport and seek bar on the whole window, use fixed timestamp widths, and balance spacing across player controls.
- Make artist credits clickable in the player, song lists, cards, and queue. Browse songs by each credited artist inside Auralis without changing the current song or global search. Preserve separate credits from Spotify and native playback metadata.

Validation: 51 automated checks passed (optional browser-launch check excluded), including actual menu clicks, independent preferences, collaborator navigation without playback changes, and player alignment at 1080, 1440, and 1920 pixels with the queue open or closed. Native UI previews cover the account menu, artist songs, and narrow layouts.

## Retained changes from 0.5.7


- Embed the existing mint waveform brand as a multi-resolution Windows executable icon and window/taskbar icon. Include SVG, PNG, ICO, and transparent tray assets.
- Add a Windows tray menu with Open, Play/Pause, Previous, Next, and Quit. The tooltip follows the current song and playback state.
- Keep music playing when closing the window, with a close-to-tray preference under Interface. Only hide when Windows has registered a working tray icon. Quit and Ctrl+Q save the session and stop the owned player.
- Reopening Auralis restores the same window, including when it is hidden or minimized, without starting a second native player. Skip drawing the full interface while hidden.

Validation: 48 automated checks passed (optional browser-launch test excluded), including native tray transport routing, close/quit behavior, and second-launch signaling. The separate Windows lifecycle check registered an actual tray icon, canceled close, reopened the hidden window, then hid it again and quit successfully. Existing playback, saved-session, and handoff checks still pass.

Installed-build verification: extracted the embedded 0.5.7 executable icon, updated the existing pinned shortcut, and confirmed Windows registered the running app's tray icon. The actual window closed to the tray; launching the pin reopened the same app process and retained its native player. Music continued through the reopen check.

## Retained changes from 0.5.6

- Keep the selected output independent of the auto-resume preference and the native receiver's active-connection flag. Starting paused still selects Auralis.
- Route main Play and the selected local controls through the native player; activate it before playback when needed. An idle/paused release no longer sends controls to a missing Web API device.
- Show **Output: Auralis** and **Selected** while paused. Preserve remote-device choices and follow confirmed remote playback.
- Retain the separate automatic-resume checkbox and all session-store fixes.

Validation: all 45 automated checks passed. The paused-start UI regression clicks the main Play button after native activity is released and verifies local activation/resume without a Web API call. The user confirmed that main Play starts audible playback in 0.5.6 with auto-resume unchecked, without selecting a device or using Play here.

## Retained changes from 0.5.5

- Resume music automatically after native login is restored on opening. Activate Auralis if necessary, then resume without starting a new URI or replacing the queue. This also works when the restored receiver is already the active device.
- Add **Resume music when Auralis opens** in Playback settings, enabled by default. Disabling it preserves the paused-start behavior.
- Keep automatic activation/resume limited to startup so later phone handoffs remain respected.

Validation: 44 automated checks passed (optional browser-launch test excluded). The user confirmed that opening 0.5.5 from the pinned icon, closing it, and reopening it resumes audio on both launches without pressing Play, choosing a device, or pairing from a phone. Read-only native queries confirmed active playback across the two process instances with Spotify Desktop closed; no playback commands were injected during this check.

## Retained changes from 0.5.4

- Use one physical native-session store under `%USERPROFILE%\.auralis\player`. AppData writes inherited from the packaged Codex host had created a separate redirected store, allowing test launches and desktop launches to use different saved connections.
- Preserve existing account and device data during migration. Leave the legacy stores intact, omit stale PID/port/lock files, and never overwrite a newer shared session.
- Select Auralis once its native session is ready after launch. Do not reactivate after a subsequent phone handoff, or start playing merely because the app opened.
- Preserve a paused bookmark through default device activation and refresh it when the user later resumes.
- Show the app version in Settings and write a credential-free per-launch runtime snapshot for diagnosing launch-context differences.

Validation: 43 automated checks passed, including session migration and default-device/resume sequencing. Desktop-launch verification is recorded separately after the migration.

## Retained changes from 0.5.2

- Follow Spotifast's managed Soloist lifecycle: restore the engine's saved session, recheck authentication while starting, and verify readiness before dispatching playback. Normal startup no longer depends on a custom pairing marker or invokes `--pair`.
- Make **Play here** activate the signed-in native receiver directly, preserving the current context instead of requiring its presence in the Web API device list. Keep Auralis visible in the device list whenever its native connection is ready.
- Prefer local playback for a fresh launch's Play or selected song; respect an explicitly chosen phone or a subsequent phone handoff.
- Replace the prominent pairing reset with **Reconnect player**, which retains the saved connection. Move deliberate reset under Advanced with a clear explanation.
- Remove routine discovery toasts and loading flicker, keep existing device rows during refresh, and stop automatic discovery while Auralis is active. Show device lookup errors inline.
- Ignore delayed events from replaced native players. Keep only bounded, fixed startup diagnostic categories, never credentials or raw engine output.
- See THIRD_PARTY_NOTICES.md for source attribution.

Validation: all 40 automated checks passed; the optional browser-launch test is excluded. Tests cover delayed native login, controls waiting for readiness, activation without Web API device discovery, stale-worker events, and quiet device polling. Fourteen native UI previews were rendered, including the ready-player settings view. A real launch from the executable folder restored the native login automatically; direct native activation/resume started “Stressed Out” locally while Spotify Desktop was closed, without selecting Auralis from a phone.

## Retained changes from 0.5.1

- Restore the last song, position, page, and search on launch. Save session metadata on close and periodically during use, using an atomic config replacement. Keep credentials in Windows Credential Manager.
- Remove demo playback and sample home content from configured accounts while authentication and the native engine reconnect. Accept inactive native snapshots so the real paused song can appear before pressing Play.
- Resume the existing native playback context. Transfer remote playback to Auralis without issuing a new song request. Correct a same-song position reset during an explicit resume or observed handoff; respect user seeking and new song selections.
- Preserve progress when native track metadata refreshes for the same URI.
- Preserve the startup bookmark against zero-position paused API snapshots. Route a remembered, paused Auralis device through native resume, and retain the handoff bookmark through activation's paused snapshot until playback begins.
- Complete Soloist's explicit persistent-pairing mode before starting the regular receiver. Remember completion only after successful exit, retain the same device/data directory, and offer **Restart pairing** for recovery without entering the key again.
- Store selected search songs with artwork and artist metadata. Play directly from recent searches, remove individual songs, or clear the list. Keep the popup interactive after search-field focus moves to a song.
- Add startup, session-file replacement, native snapshot, resume-position, recent-history, and real egui click regression checks.

## Retained playback changes from 0.5.0

- Send explicit Content-Length: 0 for empty playback commands, fixing the HTTP 411 rejection on transport, seek, volume, shuffle, repeat, and queue requests. Preserve real JSON/form body lengths.
- Render readable HTTP errors instead of showing raw gateway HTML.
- Add native local audio through Spotify's official Windows Soloist engine, under the Auralis Connect device name. Keep the personal engine key in Windows Credential Manager.
- Add local playback setup and a **Switch playback here** control in Playback settings. The app pairs through Spotify Connect, receives live playback/device events, and follows phone handoffs without automatically reactivating the local receiver.
- Preserve the server's playback state during device transfer, wait for confirmed device changes, refresh available devices, and ignore stale remote playback snapshots while the local engine is active.
- Use loopback-only WebSocket control, a bounded audio cache, and owned-process cleanup when the app closes. Retain the paired session for later launches.
- Add request framing and native WebSocket/handoff regression tests.

## Retained interface changes from 0.4.0

- Replace the old home billboard and hard-coded shortcuts with real recently played music, a responsive artwork grid, and the user's playlists.
- Add distinct Liked Songs, Recently Played, Profile, and full-page Settings views. Separate Playback, Interface, and Account settings; hide connection setup after sign-in.
- Replace font-dependent icons with native vector geometry. Give the player explicit layout regions, slim seek/volume sliders, and working queue/device controls.
- Clip long titles and artist names to their columns and artwork cards. Reserve sidebar footer space so account/settings controls cannot overlap playlist rows at small window sizes.
- Search after typing pauses, support Ctrl+K / Ctrl+L, and show recent-search suggestions only when relevant. Add a playlist filter and per-song action menus.
- Keep the queue closed by default, stop its polling while hidden, and use a lighter default progress refresh rate. Persist interface preferences.
- Use the real Spotify profile image and profile link. Accept nullable profile names and local-track IDs instead of failing the entire response.
- Share the HTTP client with artwork downloads and cap streamed artwork before decoding. Ignore in-flight responses after account sign-out.
- Add UI navigation, configuration compatibility, account-state, and offscreen rendering checks, including narrow-window layouts.

Validation: all 33 automated checks passed (one optional browser-launch test excluded). Thirteen previews render the real native UI at 1440×900 and 1080×680, including profile, settings, search, recent-song popup, playlists, and the narrow queue layout. Playback protocol tests use a loopback mock; a real phone transfer also needs an account/device check.

Live startup check: Windows OAuth restoration and normal application/owned-engine shutdown succeeded with Spotify Desktop closed. After completing explicit pairing from the phone, the native engine restarted logged in automatically and reused the saved device/session directory. The user confirmed audible playback resumed at the saved 2:20 position on “From My Window” after reopening, without re-pairing. A full phone-to-Auralis-to-phone transfer round trip remains a separate device check.

## Retained sign-in fixes from 0.3.4

- Open the registered default browser directly instead of relying on Explorer to forward Spotify URLs.
- Try callback port 43821 when 8888 is unavailable. Settings shows both explicit Redirect URIs to register in Spotify Developer Dashboard.
- Open connection settings immediately and validate the Client ID before sign-in. Add a **Save and connect Spotify** button and save settings automatically when connecting.
- Keep the current sign-in link available to open or copy manually, including when automatic browser launch fails. Add cancellation and retry controls.
- Ignore unrelated, stale, malformed, and incomplete callback requests while waiting for Spotify. Check state on both success and denial. Only show browser success after the token exchange succeeds.
- Preserve Spotify's OAuth error description in connection errors.
- Reuse the Spotify HTTP client and avoid rewriting the same refresh token on each player poll.
- Rebuild changed source when using `RUN_AURALIS.bat`. Locate Rust's standard user installation if PATH is stale.
- Add local callback regression tests and an optional default-browser smoke test.

Validation: The Windows release build and all 11 callback regression tests passed locally. The default-browser smoke test also passed, and the user confirmed that the Spotify Developer Dashboard opened and the explicit fallback Redirect URI was accepted.

Account authorization still requires the user's approval in the browser. Auralis remains a Spotify Connect controller; it does not decode audio locally.
