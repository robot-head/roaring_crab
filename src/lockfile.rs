//! A simple pidfile-based lock with stale-process detection.
//!
//! Writes the current process's PID to a file. On second acquire, reads the PID
//! and checks if it's still alive — if not, reclaims the lock.

use std::path::{Path, PathBuf};

pub struct Lock {
    path: PathBuf,
}

pub enum LockResult {
    Acquired(Lock),
    Busy,
}

impl Lock {
    pub fn try_acquire(path: &Path) -> std::io::Result<LockResult> {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(pid) = content.trim().parse::<u32>() {
                if pid_alive(pid) {
                    return Ok(LockResult::Busy);
                }
            }
            // Stale or unparseable — overwrite below.
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, std::process::id().to_string())?;
        Ok(LockResult::Acquired(Lock { path: path.to_path_buf() }))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
fn pid_alive(pid: u32) -> bool {
    unsafe {
        if libc::kill(pid as libc::pid_t, 0) == 0 {
            return true;
        }
        errno::errno().0 != libc::ESRCH
    }
}

#[cfg(windows)]
fn pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code == STILL_ACTIVE
    }
}
