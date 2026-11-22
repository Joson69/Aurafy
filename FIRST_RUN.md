# Get started with Aurafy on Windows

## Open Aurafy

1. [Download the Windows x64 ZIP](https://gitlab.com/aurafy-project1/aurafy-downloads/-/raw/main/Aurafy-0.8.0-Windows-x64.zip) and extract it.
2. Open `aurafy.exe`. You can browse the sample library before connecting Spotify. Demo mode is silent.

Closing the window sends Aurafy to the Windows tray by default. To exit, use **Quit Aurafy** from the tray menu or press **Ctrl+Q**. Change close behaviour in **Settings → General**.

## Connect your Spotify account

1. Open the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) with an active Spotify Premium account. Create an app and select **Web API**.
2. Register both redirect URIs in the app settings and save:
   - `http://127.0.0.1:8888/callback`
   - `http://127.0.0.1:43821/callback`
3. Copy the app's **Client ID**. Open Aurafy's **Settings → Account → Connect your Spotify account**, paste the Client ID, and select **Save and connect Spotify**.
4. Approve access in your browser and keep Aurafy open until sign-in finishes.

Spotify permits up to five authorized accounts on a new [Development Mode](https://developer.spotify.com/documentation/web-api/concepts/quota-modes) app. If the account you want to connect differs from the app owner's, add it to the app's user list before signing in. Some Spotify endpoints limit which playlists Aurafy can read or edit.

## Choose where to listen

For a phone or another Spotify Connect device, start Spotify on that device, then select it from the device button in Aurafy's player. Playback control through the Spotify Web API requires Premium.

For audio on this PC:

1. Generate your personal [Spotify Soloist API key](https://developer.spotify.com/dashboard/soloist) with your Premium account.
2. Open **Settings → Account → Local player connection**, paste the key into the masked field, and select **Save key & enable playback**.
3. After Aurafy starts the official player, open Spotify's device picker on a phone or desktop on the same local network and select **Aurafy**. This pairs the player for subsequent launches.

Use the device button to transfer playback back and forth. To start paused, turn off **Resume music when Aurafy opens** in **Settings → General**.

## If setup gets stuck

| What you see | What to try |
| --- | --- |
| The sign-in page did not open | Select **Open sign-in page** or **Copy sign-in link** in Aurafy. |
| Spotify reports an invalid client or redirect URL | Check the Client ID and save both exact redirect URIs in the same Spotify app. |
| Spotify refuses access to the account | Verify the developer app owner still has Premium and the signing-in account is authorized in the dashboard. |
| Aurafy is missing from Spotify's device picker | Wait for the local player to finish starting, then confirm both devices are on the same local network. |
| Previously working local playback stopped | Select **Reconnect player** in **Settings → Account → Local player connection → Connection troubleshooting**. Spotify Soloist builds [expire after 90 days](https://developer.spotify.com/documentation/soloist/reference/downloads-and-updates). Follow the app's message if it asks for a new engine build. |

Aurafy uses browser-based Spotify sign-in. Enter a Client ID in the sign-in form and a Soloist API key in the local player form. Keep your Client Secret, sign-in URLs, and keys out of issue reports.

For building from source, see the [README](README.md#build-and-contribute).
