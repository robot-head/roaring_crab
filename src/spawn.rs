//! Spawns the daemon as a detached background process.
//!
//! On Unix: setsid so the daemon is detached from the controlling terminal
//! and survives the client exiting.
//! On Windows: CREATE_NO_WINDOW | DETACHED_PROCESS so no console flashes and
//! the daemon is independent of the client's lifetime.

use std::path::Path;
use std::process::Command;

pub fn spawn_daemon(daemon_path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut cmd = Command::new(daemon_path);
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        cmd.spawn()?;
        Ok(())
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new(daemon_path)
            .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        Ok(())
    }
}

/// Given the path to the running client binary, returns the expected daemon
/// binary path next to it — e.g., `/path/to/roaring-crab` → `/path/to/roaring-crabd`
/// (with `.exe` on Windows).
pub fn daemon_sibling_path(client_path: &Path) -> std::path::PathBuf {
    let parent = client_path.parent().unwrap_or(Path::new("."));
    #[cfg(windows)]
    let name = "roaring-crabd.exe";
    #[cfg(not(windows))]
    let name = "roaring-crabd";
    parent.join(name)
}
