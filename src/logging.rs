use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Suppresses repeated identical messages within a time window.
pub struct RateLimiter {
    window: Duration,
    last_seen: HashMap<String, Instant>,
}

impl RateLimiter {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            last_seen: HashMap::new(),
        }
    }

    /// Returns true if the message with this key should be emitted.
    pub fn allow(&mut self, key: &str) -> bool {
        let now = Instant::now();
        match self.last_seen.get(key) {
            Some(t) if now.duration_since(*t) < self.window => false,
            _ => {
                self.last_seen.insert(key.to_string(), now);
                true
            }
        }
    }
}

/// A log file that rolls over to `<path>.old` when it exceeds `cap_bytes`.
pub struct RollingLog {
    path: PathBuf,
    cap_bytes: u64,
    file: File,
}

impl RollingLog {
    pub fn open(path: &Path, cap_bytes: u64) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            cap_bytes,
            file,
        })
    }

    pub fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        writeln!(self.file, "{}", line)?;
        self.file.flush()?;
        let size = self.file.metadata()?.len();
        if size >= self.cap_bytes {
            self.roll()?;
        }
        Ok(())
    }

    fn roll(&mut self) -> std::io::Result<()> {
        let old = self.path.with_extension("log.old");
        // Close current handle by replacing with a placeholder, then rename, then reopen.
        let tmp_path = self.path.with_extension("log.tmp");
        let placeholder = File::create(&tmp_path)?;
        let _old_handle = std::mem::replace(&mut self.file, placeholder);
        // Ensure the original handle is dropped before rename (needed on Windows).
        drop(_old_handle);
        if old.exists() {
            std::fs::remove_file(&old)?;
        }
        std::fs::rename(&self.path, &old)?;
        std::fs::rename(&tmp_path, &self.path)?;
        self.file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        Ok(())
    }
}
