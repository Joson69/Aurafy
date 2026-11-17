//! Per-user Windows startup. Only Aurafy's own Run value is read or changed.
use std::path::Path;

pub fn command(executable: &Path, in_tray: bool) -> Result<String, String> {
    let path = executable
        .to_str()
        .ok_or("The app path is not valid Unicode")?;
    if path.contains(['"', '\0']) || !executable.is_absolute() {
        return Err("The app path is not valid for Windows startup".into());
    }
    Ok(format!(
        "\"{path}\" --windows-startup{}",
        if in_tray { " --start-in-tray" } else { "" }
    ))
}

#[cfg(target_os = "windows")]
mod windows {
    use windows_sys::Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::*,
    };
    const RUN: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    const NAME: &str = "Aurafy";
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    struct Key(HKEY);
    impl Drop for Key {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }
    fn error(code: u32) -> String {
        format!(
            "Could not change Windows startup: {}",
            std::io::Error::from_raw_os_error(code as i32)
        )
    }
    pub(super) fn read_at(path: &str, name: &str) -> Result<Option<String>, String> {
        let mut handle = std::ptr::null_mut();
        let result = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                wide(path).as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut handle,
            )
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if result != ERROR_SUCCESS {
            return Err(error(result));
        }
        let key = Key(handle);
        let mut kind = 0;
        let mut bytes = 0;
        let result = unsafe {
            RegQueryValueExW(
                key.0,
                wide(name).as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut bytes,
            )
        };
        if result == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if result != ERROR_SUCCESS {
            return Err(error(result));
        }
        if kind != REG_SZ || bytes > 32768 {
            return Err("Aurafy's Windows startup entry is invalid".into());
        }
        let mut value = vec![0u16; (bytes as usize).div_ceil(2)];
        let result = unsafe {
            RegQueryValueExW(
                key.0,
                wide(name).as_ptr(),
                std::ptr::null(),
                &mut kind,
                value.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if result != ERROR_SUCCESS {
            return Err(error(result));
        }
        value.truncate(bytes as usize / 2);
        while value.last() == Some(&0) {
            value.pop();
        }
        Ok(Some(String::from_utf16_lossy(&value)))
    }
    pub(super) fn write_at(path: &str, name: &str, command: Option<&str>) -> Result<(), String> {
        let mut handle = std::ptr::null_mut();
        // No administrator privileges or scheduled task; scoped to this user's Run key.
        let result = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(path).as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut handle,
                std::ptr::null_mut(),
            )
        };
        if result != ERROR_SUCCESS {
            return Err(error(result));
        }
        let key = Key(handle);
        let result = if let Some(command) = command {
            let data = wide(command);
            unsafe {
                RegSetValueExW(
                    key.0,
                    wide(name).as_ptr(),
                    0,
                    REG_SZ,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                )
            }
        } else {
            unsafe { RegDeleteValueW(key.0, wide(name).as_ptr()) }
        };
        if result == ERROR_SUCCESS || (command.is_none() && result == ERROR_FILE_NOT_FOUND) {
            Ok(())
        } else {
            Err(error(result))
        }
    }
    pub fn enabled() -> Result<bool, String> {
        Ok(read_at(RUN, NAME)?.is_some())
    }
    pub fn set(enabled: bool, in_tray: bool) -> Result<(), String> {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let value = super::command(&executable, in_tray)?;
        write_at(RUN, NAME, enabled.then_some(value.as_str()))
    }
}

#[cfg(target_os = "windows")]
pub use windows::{enabled, set};
#[cfg(not(target_os = "windows"))]
pub fn enabled() -> Result<bool, String> {
    Ok(false)
}
#[cfg(not(target_os = "windows"))]
pub fn set(_: bool, _: bool) -> Result<(), String> {
    Err("Windows startup is only available on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn startup_command_quotes_paths_and_tray_is_explicit() {
        let path = Path::new("C:\\Music apps\\Aurafy\\aurafy.exe");
        assert_eq!(
            command(path, false).unwrap(),
            "\"C:\\Music apps\\Aurafy\\aurafy.exe\" --windows-startup"
        );
        assert!(command(path, true).unwrap().ends_with(" --start-in-tray"));
        assert!(command(Path::new("relative.exe"), false).is_err());
    }
    #[cfg(target_os = "windows")]
    #[test]
    fn startup_registry_round_trip_preserves_other_values() {
        // Test outside the real startup key; nothing here can run at sign-in.
        let path = format!("Software\\AurafyTests\\Startup-{}", rand::random::<u64>());
        windows::write_at(&path, "Other", Some("untouched")).unwrap();
        windows::write_at(&path, "Aurafy", Some("quoted test command")).unwrap();
        assert_eq!(
            windows::read_at(&path, "Aurafy").unwrap().as_deref(),
            Some("quoted test command")
        );
        windows::write_at(&path, "Aurafy", None).unwrap();
        assert!(windows::read_at(&path, "Aurafy").unwrap().is_none());
        assert_eq!(
            windows::read_at(&path, "Other").unwrap().as_deref(),
            Some("untouched")
        );
        use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RegDeleteKeyW};
        let path: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        unsafe {
            RegDeleteKeyW(HKEY_CURRENT_USER, path.as_ptr());
        }
    }
}
