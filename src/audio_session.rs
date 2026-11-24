//! Apply Aurafy branding to its native receiver's Windows audio session.
#[cfg(target_os = "windows")]
pub fn start() {
    std::thread::spawn(|| {
        use windows::{
            Win32::{Media::Audio::*, System::Com::*},
            core::{Interface, PCWSTR},
        };
        // COM objects stay on this dedicated thread and are never sent to the UI.
        unsafe {
            if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
                return;
            }
            let mut seen = std::collections::HashSet::new();
            loop {
                let result = (|| -> windows::core::Result<()> {
                    let Some(engine) = crate::local_player::existing_engine_path() else {
                        return Ok(());
                    };
                    let data = crate::local_player::directory().join("data");
                    let Some((pid, _receiver)) =
                        crate::player_process::recorded_receiver(&engine, &data, 0)
                    else {
                        return Ok(());
                    };
                    let enumerator: IMMDeviceEnumerator =
                        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                    let endpoints = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
                    for device in 0..endpoints.GetCount()? {
                        let Ok(endpoint) = endpoints.Item(device) else {
                            continue;
                        };
                        let Ok(manager) =
                            endpoint.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                        else {
                            continue;
                        };
                        let Ok(sessions) = manager.GetSessionEnumerator() else {
                            continue;
                        };
                        let Ok(count) = sessions.GetCount() else {
                            continue;
                        };
                        for i in 0..count {
                            let Ok(session) = sessions.GetSession(i) else {
                                continue;
                            };
                            let Ok(control) = session.cast::<IAudioSessionControl2>() else {
                                continue;
                            };
                            if control.GetProcessId().ok() != Some(pid) {
                                continue;
                            }
                            let Ok(identifier) = control.GetSessionInstanceIdentifier() else {
                                continue;
                            };
                            let identity = identifier.to_string();
                            CoTaskMemFree(Some(identifier.0.cast()));
                            let Ok(identity) = identity else {
                                continue;
                            };
                            if seen.contains(&identity) {
                                continue;
                            }
                            let path = std::env::current_exe()
                                .map_err(|_| windows::core::Error::from_win32())?;
                            let wide: Vec<u16> = path
                                .to_string_lossy()
                                .encode_utf16()
                                .chain(Some(0))
                                .collect();
                            if session
                                .SetDisplayName(windows::core::w!("Aurafy"), std::ptr::null())
                                .is_err()
                                || session
                                    .SetIconPath(PCWSTR(wide.as_ptr()), std::ptr::null())
                                    .is_err()
                            {
                                continue;
                            }
                            if seen.len() >= 64 {
                                seen.clear();
                            }
                            seen.insert(identity);
                            let _=std::fs::write(crate::local_player::directory().join("audio-session-status.json"),serde_json::json!({"pid":pid,"name":"Aurafy","icon":path,"applied":true}).to_string());
                        }
                    }
                    Ok(())
                })();
                let _ = result;
                std::thread::sleep(std::time::Duration::from_secs(10));
            }
        }
    });
}
