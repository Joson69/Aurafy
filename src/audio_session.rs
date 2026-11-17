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
                    let pid = std::fs::read_to_string(
                        crate::local_player::directory().join("data/soloist.pid"),
                    )
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok())
                    .unwrap_or(0);
                    if pid == 0 {
                        return Ok(());
                    }
                    let enumerator: IMMDeviceEnumerator =
                        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                    let endpoints = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
                    for device in 0..endpoints.GetCount()? {
                        let endpoint = endpoints.Item(device)?;
                        let manager: IAudioSessionManager2 = endpoint.Activate(CLSCTX_ALL, None)?;
                        let sessions = manager.GetSessionEnumerator()?;
                        for i in 0..sessions.GetCount()? {
                            let session = sessions.GetSession(i)?;
                            let control: IAudioSessionControl2 = session.cast()?;
                            if control.GetProcessId()? != pid {
                                continue;
                            }
                            let identifier = control.GetSessionInstanceIdentifier()?;
                            let identity = identifier.to_string()?;
                            CoTaskMemFree(Some(identifier.0.cast()));
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
                            session
                                .SetDisplayName(windows::core::w!("Aurafy"), std::ptr::null())?;
                            session.SetIconPath(PCWSTR(wide.as_ptr()), std::ptr::null())?;
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
