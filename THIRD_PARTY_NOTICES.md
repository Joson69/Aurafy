# Lato fonts

Aurafy embeds unmodified Lato Regular and Lato Bold for the player and lyrics.
Copyright (c) 2010-2014 by tyPoland Lukasz Dziedzic (team@latofonts.com),
with Reserved Font Name "Lato". Distributed under the SIL Open Font License 1.1.
The complete license is in `assets/fonts/OFL.txt` and embedded in the app under
Settings → Account → Font credits.

Source: https://github.com/google/fonts/tree/main/ofl/lato

# Playback integration reference

Aurafy's managed Soloist authentication polling and saved-session lifecycle
are informed by Spotifast's MIT-licensed implementation. Aurafy keeps its
own UI and playback controller; it does not bundle the Spotifast application.

Reference: https://github.com/crmne/spotifast/blob/58ec45608a5a2bb7f18c08afd63e4b931d2f6ba0/src/soloist/mod.rs

The leftover-process recovery design also follows Spotifast's approach of
checking the recorded process identity before reclaiming an abandoned player.
Aurafy uses its own Windows process-handle and creation-time checks.
Implementation reviewed on 2026-11-06:
https://github.com/crmne/spotifast/blob/main/src/soloist/mod.rs (reclaim).

Collection playback also follows the separation documented in Spotifast's
Soloist protocol module: native transport/state use the local API; context
starts retain their selected occurrence through the Spotify Web API.
Reviewed on 2026-11-06:
https://github.com/crmne/spotifast/blob/main/src/soloist/protocol.rs (commands).

Full native queue retrieval also follows Spotifast's documented `limit: 0`
query on a separate RPC socket, keeping capped broadcasts separate from queue
snapshots. Reviewed on 2026-11-10:
https://github.com/crmne/spotifast/blob/main/src/soloist/protocol.rs (Client::rpc_unlocked and ApiRequest::Queue).
Provider protocol: https://developer.spotify.com/documentation/soloist/reference/websocket-api

MIT License

Copyright (c) 2026 Carmine Paolino

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

# Windows tray libraries

The tray-icon and muda libraries are used under their MIT license.
Sources: https://github.com/tauri-apps/tray-icon and https://github.com/tauri-apps/muda

MIT License

Copyright (c) 2022-2022 Tauri Programme within The Commons Conservancy

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


# Soloist audio preference research

The startup preference key names, quality enumeration and per-user precedence
were researched from Music Assistant's Soloist integration (dev branch, reviewed
2026-11-10). Aurafy's Rust writer is implemented independently; no Python source
or Music Assistant runtime is included. Music Assistant is Apache-2.0 licensed.

https://github.com/music-assistant/server/blob/dev/music_assistant/providers/spotify_connect/soloist/prefs.py
https://github.com/music-assistant/server/blob/dev/music_assistant/providers/spotify_connect/soloist/README.md

The integration reports that crossfade is unreliable for actual album/playlist
contexts. Aurafy does not expose that control. Streaming quality remains a request,
not delivered-format telemetry.
