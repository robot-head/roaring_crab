//! Resolves the local socket address used between client and daemon.
//!
//! Cross-platform via the `interprocess` crate's namespaced name:
//! - Unix: file path under runtime dir
//! - Windows: named pipe

use interprocess::local_socket::{GenericFilePath, GenericNamespaced, Name, ToFsName, ToNsName};

/// Returns the platform-appropriate socket name. Honors `RC_SOCKET_PATH` env var
/// for tests (overrides the default location).
pub fn socket_name() -> std::io::Result<Name<'static>> {
    if let Ok(override_path) = std::env::var("RC_SOCKET_PATH") {
        return Ok(override_path.to_fs_name::<GenericFilePath>()?.into_owned());
    }

    #[cfg(unix)]
    {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let p = runtime.join("roaring-crab.sock");
        Ok(p.to_fs_name::<GenericFilePath>()?.into_owned())
    }

    #[cfg(windows)]
    {
        if let Ok(pipe) = std::env::var("RC_SOCKET_PIPE") {
            return Ok(pipe.to_ns_name::<GenericNamespaced>()?.into_owned());
        }
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".to_string());
        let name = format!("roaring-crab-{}", user);
        Ok(name.to_ns_name::<GenericNamespaced>()?.into_owned())
    }
}

/// On Unix, the filesystem path the socket file lives at (used for lockfile sibling, cleanup).
/// On Windows, returns `None` because named pipes don't have a filesystem location.
pub fn socket_fs_path() -> Option<std::path::PathBuf> {
    #[cfg(unix)]
    {
        if let Ok(p) = std::env::var("RC_SOCKET_PATH") {
            return Some(std::path::PathBuf::from(p));
        }
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        Some(runtime.join("roaring-crab.sock"))
    }
    #[cfg(windows)]
    {
        None
    }
}
