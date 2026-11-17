use windows::{
    Win32::{Media::Audio::*, System::Com::*},
    core::Interface,
};
fn main() -> windows::core::Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
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
                if control.GetProcessId()?
                    == std::fs::read_to_string(
                        dirs::home_dir()
                            .unwrap()
                            .join(".auralis/player/data/soloist.pid"),
                    )
                    .unwrap()
                    .trim()
                    .parse::<u32>()
                    .unwrap()
                {
                    let path = std::env::current_dir()
                        .unwrap()
                        .join("target/release/aurafy.exe");
                    let wide: Vec<u16> = path
                        .to_string_lossy()
                        .encode_utf16()
                        .chain(Some(0))
                        .collect();
                    session.SetDisplayName(windows::core::w!("Aurafy"), std::ptr::null())?;
                    session.SetIconPath(windows::core::PCWSTR(wide.as_ptr()), std::ptr::null())?;
                    let icon = session.GetIconPath()?;
                    println!("Aurafy session icon: {}", icon.to_string()?);
                    CoTaskMemFree(Some(icon.0.cast()));
                }
            }
        }
    }
    Ok(())
}
