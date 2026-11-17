//! Recover a native player left behind by an abruptly terminated app.
//! Only the process recorded in this data directory with the expected image
//! and a creation time no newer than that PID file can be reclaimed.
#[cfg(windows)]
pub fn reclaim(executable: &std::path::Path, data: &std::path::Path) -> Result<(), String> {
    use std::{
        fs,
        os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
        time::{Duration, UNIX_EPOCH},
    };
    use windows_sys::Win32::{
        Foundation::{FILETIME, WAIT_OBJECT_0},
        System::Threading::{
            GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
        },
    };
    let pid_file = data.join("soloist.pid");
    let Some(pid) = fs::read_to_string(&pid_file)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|pid| *pid != 0 && *pid != std::process::id())
    else {
        return Ok(());
    };
    // Keep the handle throughout verification and termination, avoiding PID reuse.
    let raw = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
            0,
            pid,
        )
    };
    if raw.is_null() {
        return Ok(());
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(raw) };
    let mut path = vec![0u16; 32768];
    let mut length = path.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(handle.as_raw_handle(), 0, path.as_mut_ptr(), &mut length)
    } == 0
    {
        return Ok(());
    }
    let actual = std::path::PathBuf::from(String::from_utf16_lossy(&path[..length as usize]));
    let (Ok(actual), Ok(expected)) = (actual.canonicalize(), executable.canonicalize()) else {
        return Ok(());
    };
    if !actual
        .to_string_lossy()
        .eq_ignore_ascii_case(&expected.to_string_lossy())
    {
        return Ok(());
    }
    let mut times = [FILETIME::default(); 4];
    let [created, exited, kernel, user] = &mut times;
    if unsafe { GetProcessTimes(handle.as_raw_handle(), created, exited, kernel, user) } == 0 {
        return Ok(());
    }
    let ticks = (u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime);
    let Some(unix_ticks) = ticks.checked_sub(116_444_736_000_000_000) else {
        return Ok(());
    };
    let created_at = UNIX_EPOCH + Duration::from_nanos(unix_ticks.saturating_mul(100));
    let Some(recorded_at) = fs::metadata(&pid_file).ok().and_then(|m| m.modified().ok()) else {
        return Ok(());
    };
    if recorded_at < created_at {
        return Ok(());
    }
    if unsafe { WaitForSingleObject(handle.as_raw_handle(), 0) } == WAIT_OBJECT_0 {
        return Ok(());
    }
    if unsafe { TerminateProcess(handle.as_raw_handle(), 0) } == 0
        || unsafe { WaitForSingleObject(handle.as_raw_handle(), 5000) } != WAIT_OBJECT_0
    {
        return Err("The previous Aurafy player could not close. Quit that player, then reconnect local playback.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn reclaim(_: &std::path::Path, _: &std::path::Path) -> Result<(), String> {
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::{
        fs,
        process::{Command, Stdio},
        time::{Duration, SystemTime},
    };

    #[test]
    fn reclaim_checks_image_and_pid_file_age_before_stopping_a_recorded_player() {
        use std::os::windows::process::CommandExt;
        let root = std::env::temp_dir().join(format!("aurafy-reclaim-{}", rand::random::<u64>()));
        fs::create_dir(&root).unwrap();
        let executable = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut child = Command::new(&executable)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let record = root.join("soloist.pid");
        fs::write(&record, child.id().to_string()).unwrap();
        let wrong_image = std::env::current_exe().unwrap();
        reclaim(&wrong_image, &root).unwrap();
        assert!(
            child.try_wait().unwrap().is_none(),
            "unrelated executable must survive"
        );
        let file = fs::File::options().write(true).open(&record).unwrap();
        file.set_times(
            fs::FileTimes::new().set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
        )
        .unwrap();
        reclaim(&executable, &root).unwrap();
        assert!(
            child.try_wait().unwrap().is_none(),
            "stale PID record must not target a reused PID"
        );
        file.set_times(fs::FileTimes::new().set_modified(SystemTime::now()))
            .unwrap();
        drop(file);
        reclaim(&executable, &root).unwrap();
        assert!(child.wait().is_ok());
        reclaim(&executable, &root).unwrap();
        fs::remove_file(record).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
