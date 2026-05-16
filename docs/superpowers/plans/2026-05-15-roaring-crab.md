# roaring-crab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform Claude Code plugin in Rust that plays generated analog-modeling synth sounds on hook events, using a lazy-spawned long-lived daemon.

**Architecture:** Single Cargo workspace producing two binaries (`roaring-crab` client, `roaring-crabd` daemon) plus a shared library. Client is invoked by every hook, sends a `PlayEvent` over a local socket, exits. Daemon owns `cpal` audio output and a `fundsp`-powered voice mixer; lazy-spawned by client on first event; idle-exits after 5 minutes.

**Tech Stack:** Rust (stable), `cpal` (audio output), `fundsp` (DSP/synthesis), `interprocess` (cross-platform local sockets), `bincode` 1.x (wire format), `serde` + `toml` (config), `clap` (CLI), `rand` (per-event seeds), `directories` (per-OS config paths), GitHub Actions (CI/release).

**Reference spec:** `docs/superpowers/specs/2026-05-15-roaring-crab-design.md`

---

## File Structure

```
roaring_crab/
├── Cargo.toml
├── .gitignore
├── README.md
├── hooks.json
├── bin/
│   ├── launch.sh                    (Unix dispatcher)
│   ├── launch.cmd                   (Windows dispatcher)
│   ├── linux-x86_64/                (populated by release CI)
│   ├── linux-aarch64/
│   ├── macos-x86_64/
│   ├── macos-aarch64/
│   └── windows-x86_64/
├── src/
│   ├── lib.rs                       (re-exports modules)
│   ├── hook_event.rs                (HookEvent enum, 9 variants)
│   ├── protocol.rs                  (PlayEvent + framed bincode)
│   ├── config.rs                    (TOML load/save, defaults)
│   ├── socket_path.rs               (per-OS socket address)
│   ├── lockfile.rs                  (pidfile w/ stale detection)
│   ├── logging.rs                   (rate-limited stderr + rolling daemon log)
│   ├── audio_sink.rs                (cpal output or null sink behind feature)
│   ├── mixer.rs                     (Voice + voice list + audio callback)
│   ├── spawn.rs                     (cross-platform detached daemon spawn)
│   ├── patches/
│   │   ├── mod.rs                   (dispatch by HookEvent, shared helpers)
│   │   ├── ambient.rs               (PreToolUse, PostToolUse, UserPromptSubmit)
│   │   ├── lifecycle.rs             (SessionStart, SessionEnd, PreCompact)
│   │   └── alert.rs                 (Notification, Stop, SubagentStop)
│   └── bin/
│       ├── roaring-crab.rs          (client)
│       └── roaring-crabd.rs         (daemon)
├── tests/
│   ├── protocol_roundtrip.rs
│   ├── config_io.rs
│   ├── patches_render.rs
│   ├── mixer_concurrency.rs
│   ├── client_to_daemon.rs
│   ├── lazy_spawn.rs
│   └── concurrent_events.rs
├── examples/
│   └── audition.rs
└── .github/
    └── workflows/
        ├── ci.yml
        └── release.yml
```

Each module has a single, well-bounded responsibility. The `lib` crate exposes everything; both `[[bin]]` targets are thin glue.

---

## Task 1: Scaffold project

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `src/lib.rs`
- Create: `src/bin/roaring-crab.rs`
- Create: `src/bin/roaring-crabd.rs`

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "roaring-crab"
version = "0.1.0"
edition = "2021"
description = "Claude Code plugin that plays generated analog-modeling synth sounds on hook events"
license = "MIT OR Apache-2.0"

[lib]
path = "src/lib.rs"

[[bin]]
name = "roaring-crab"
path = "src/bin/roaring-crab.rs"

[[bin]]
name = "roaring-crabd"
path = "src/bin/roaring-crabd.rs"

[features]
default = []
# Used in tests/CI to swap real cpal output for an in-memory ring buffer.
null-audio = []

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
bincode = "1.3"
toml = "0.8"
rand = "0.8"
directories = "5"
cpal = "0.15"
fundsp = "0.18"
interprocess = "2"
parking_lot = "0.12"
once_cell = "1"
log = "0.4"
anyhow = "1"
thiserror = "1"

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.52", features = [
    "Win32_System_Threading",
    "Win32_Foundation",
    "Win32_System_ProcessStatus",
] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Write `.gitignore`**

```
/target
Cargo.lock.bak
*.swp
.DS_Store
```

(Don't gitignore `Cargo.lock` — this is a binary crate.)

- [ ] **Step 3: Write minimal `src/lib.rs`**

```rust
pub mod hook_event;
pub mod protocol;
pub mod config;
pub mod socket_path;
pub mod lockfile;
pub mod logging;
pub mod audio_sink;
pub mod mixer;
pub mod spawn;
pub mod patches;
```

(Modules don't exist yet — this will fail to compile until later tasks fill them in. That's expected. Use stub modules below to make this task self-contained.)

Replace with stub:
```rust
pub mod hook_event { }
pub mod protocol { }
pub mod config { }
pub mod socket_path { }
pub mod lockfile { }
pub mod logging { }
pub mod audio_sink { }
pub mod mixer { }
pub mod spawn { }
pub mod patches { }
```

- [ ] **Step 4: Write stub binaries**

`src/bin/roaring-crab.rs`:
```rust
fn main() {
    eprintln!("roaring-crab client stub");
}
```

`src/bin/roaring-crabd.rs`:
```rust
fn main() {
    eprintln!("roaring-crabd daemon stub");
}
```

- [ ] **Step 5: Verify it builds**

Run: `cargo build --bins`
Expected: Success, two binaries produced under `target/debug/`.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/lib.rs src/bin/
git commit -m "Scaffold roaring-crab workspace"
```

---

## Task 2: HookEvent enum

**Files:**
- Replace: `src/lib.rs` (remove `pub mod hook_event { }` stub, add real module)
- Create: `src/hook_event.rs`
- Create: `tests/hook_event_parsing.rs`

- [ ] **Step 1: Write failing test**

`tests/hook_event_parsing.rs`:
```rust
use roaring_crab::hook_event::HookEvent;
use std::str::FromStr;

#[test]
fn all_nine_variants_parse_from_their_canonical_names() {
    let cases = [
        ("SessionStart", HookEvent::SessionStart),
        ("SessionEnd", HookEvent::SessionEnd),
        ("UserPromptSubmit", HookEvent::UserPromptSubmit),
        ("PreToolUse", HookEvent::PreToolUse),
        ("PostToolUse", HookEvent::PostToolUse),
        ("Notification", HookEvent::Notification),
        ("Stop", HookEvent::Stop),
        ("SubagentStop", HookEvent::SubagentStop),
        ("PreCompact", HookEvent::PreCompact),
    ];
    for (s, expected) in cases {
        assert_eq!(HookEvent::from_str(s).unwrap(), expected, "{}", s);
    }
}

#[test]
fn unknown_event_string_returns_error() {
    assert!(HookEvent::from_str("Foobar").is_err());
}

#[test]
fn serde_roundtrip() {
    let json = serde_json::to_string(&HookEvent::Stop).unwrap();
    let back: HookEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(back, HookEvent::Stop);
}
```

Add `serde_json = "1"` to `[dev-dependencies]` in `Cargo.toml` for this test.

- [ ] **Step 2: Run test, verify it fails**

Run: `cargo test --test hook_event_parsing`
Expected: FAIL — `hook_event` module is empty.

- [ ] **Step 3: Implement `HookEvent`**

In `src/lib.rs`, replace the stub `pub mod hook_event { }` with `pub mod hook_event;`.

`src/hook_event.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum,
)]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    Notification,
    Stop,
    SubagentStop,
    PreCompact,
}

impl HookEvent {
    pub const ALL: [HookEvent; 9] = [
        HookEvent::SessionStart,
        HookEvent::SessionEnd,
        HookEvent::UserPromptSubmit,
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::Notification,
        HookEvent::Stop,
        HookEvent::SubagentStop,
        HookEvent::PreCompact,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            HookEvent::SessionStart => "SessionStart",
            HookEvent::SessionEnd => "SessionEnd",
            HookEvent::UserPromptSubmit => "UserPromptSubmit",
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::Notification => "Notification",
            HookEvent::Stop => "Stop",
            HookEvent::SubagentStop => "SubagentStop",
            HookEvent::PreCompact => "PreCompact",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown hook event: {0}")]
pub struct UnknownHookEvent(pub String);

impl FromStr for HookEvent {
    type Err = UnknownHookEvent;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for h in Self::ALL {
            if h.as_str() == s {
                return Ok(h);
            }
        }
        Err(UnknownHookEvent(s.to_string()))
    }
}
```

- [ ] **Step 4: Run test, verify it passes**

Run: `cargo test --test hook_event_parsing`
Expected: All 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/hook_event.rs tests/hook_event_parsing.rs Cargo.toml Cargo.lock
git commit -m "Add HookEvent enum with parsing + serde"
```

---

## Task 3: Wire protocol (PlayEvent + framing)

**Files:**
- Replace: `src/lib.rs` (un-stub `protocol`)
- Create: `src/protocol.rs`
- Create: `tests/protocol_roundtrip.rs`

- [ ] **Step 1: Write failing tests**

`tests/protocol_roundtrip.rs`:
```rust
use roaring_crab::hook_event::HookEvent;
use roaring_crab::protocol::{read_frame, write_frame, PlayEvent, FrameError, MAX_FRAME_SIZE};
use std::io::Cursor;

#[test]
fn play_event_roundtrips_through_frame() {
    let original = PlayEvent {
        event: HookEvent::Stop,
        seed: 0xDEADBEEFCAFEBABE,
        volume: 0.42,
    };
    let mut buf = Vec::new();
    write_frame(&mut buf, &original).unwrap();
    let decoded = read_frame(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn trailing_garbage_after_frame_is_ignored_by_read_frame() {
    let event = PlayEvent {
        event: HookEvent::PreToolUse,
        seed: 1,
        volume: 0.5,
    };
    let mut buf = Vec::new();
    write_frame(&mut buf, &event).unwrap();
    buf.extend_from_slice(b"trailing junk that should not affect the read");
    let decoded = read_frame(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn oversized_frame_is_rejected() {
    // Craft a frame with a length-prefix declaring a payload over MAX_FRAME_SIZE.
    let mut buf = Vec::new();
    buf.extend_from_slice(&((MAX_FRAME_SIZE + 1) as u32).to_be_bytes());
    // No payload needed — the size check comes first.
    let err = read_frame(&mut Cursor::new(&buf)).unwrap_err();
    matches!(err, FrameError::TooLarge(_));
}

#[test]
fn all_hook_event_variants_serialize() {
    for event in HookEvent::ALL {
        let p = PlayEvent { event, seed: 7, volume: 0.5 };
        let mut buf = Vec::new();
        write_frame(&mut buf, &p).unwrap();
        let decoded = read_frame(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(decoded, p);
    }
}
```

- [ ] **Step 2: Run test, verify it fails**

Run: `cargo test --test protocol_roundtrip`
Expected: FAIL — `protocol` module is empty.

- [ ] **Step 3: Implement protocol**

In `src/lib.rs`, replace `pub mod protocol { }` with `pub mod protocol;`.

`src/protocol.rs`:
```rust
use crate::hook_event::HookEvent;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

pub const MAX_FRAME_SIZE: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayEvent {
    pub event: HookEvent,
    pub seed: u64,
    pub volume: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("frame larger than {0} bytes")]
    TooLarge(usize),
    #[error("decode error: {0}")]
    Decode(#[from] bincode::Error),
}

pub fn write_frame<W: Write>(w: &mut W, event: &PlayEvent) -> Result<(), FrameError> {
    let bytes = bincode::serialize(event)?;
    if bytes.len() > MAX_FRAME_SIZE {
        return Err(FrameError::TooLarge(MAX_FRAME_SIZE));
    }
    w.write_all(&(bytes.len() as u32).to_be_bytes())?;
    w.write_all(&bytes)?;
    Ok(())
}

pub fn read_frame<R: Read>(r: &mut R) -> Result<PlayEvent, FrameError> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(FrameError::TooLarge(MAX_FRAME_SIZE));
    }
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)?;
    let event = bincode::deserialize(&payload)?;
    Ok(event)
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test protocol_roundtrip`
Expected: All 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/protocol.rs tests/protocol_roundtrip.rs
git commit -m "Add PlayEvent wire protocol with length-prefixed framing"
```

---

## Task 4: Config (TOML load/save with defaults)

**Files:**
- Replace: `src/lib.rs` (un-stub `config`)
- Create: `src/config.rs`
- Create: `tests/config_io.rs`

- [ ] **Step 1: Write failing tests**

`tests/config_io.rs`:
```rust
use roaring_crab::config::{Config, ConfigError};
use roaring_crab::hook_event::HookEvent;
use tempfile::TempDir;

#[test]
fn missing_file_yields_defaults_and_writes_them() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    let cfg = Config::load_or_default(&path).unwrap();
    assert!((cfg.master_volume - 0.7).abs() < f32::EPSILON);
    assert!(!cfg.muted);
    for h in HookEvent::ALL {
        assert!(cfg.is_enabled(h), "{:?} should default to enabled", h);
    }
    assert!(path.exists(), "defaults should be persisted on first load");
}

#[test]
fn malformed_toml_returns_error() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    std::fs::write(&path, "master_volume = \"not a number\"").unwrap();
    let err = Config::load_or_default(&path).unwrap_err();
    assert!(matches!(err, ConfigError::Parse(_)));
}

#[test]
fn unknown_top_level_field_is_tolerated() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    std::fs::write(
        &path,
        "master_volume = 0.5\nfuture_field = \"hello\"\n",
    )
    .unwrap();
    let cfg = Config::load_or_default(&path).unwrap();
    assert!((cfg.master_volume - 0.5).abs() < f32::EPSILON);
}

#[test]
fn per_hook_disable_round_trips() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
master_volume = 0.3
muted = false

[enabled_hooks]
Stop = false
"#,
    )
    .unwrap();
    let cfg = Config::load_or_default(&path).unwrap();
    assert!(!cfg.is_enabled(HookEvent::Stop));
    assert!(cfg.is_enabled(HookEvent::Notification));
}
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test --test config_io`
Expected: FAIL — `config` module is empty.

- [ ] **Step 3: Implement config**

In `src/lib.rs`, replace `pub mod config { }` with `pub mod config;`.

`src/config.rs`:
```rust
use crate::hook_event::HookEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub master_volume: f32,
    pub muted: bool,
    pub enabled_hooks: HashMap<String, bool>,
}

impl Default for Config {
    fn default() -> Self {
        let mut enabled_hooks = HashMap::new();
        for h in HookEvent::ALL {
            enabled_hooks.insert(h.as_str().to_string(), true);
        }
        Self {
            master_volume: 0.7,
            muted: false,
            enabled_hooks,
        }
    }
}

impl Config {
    pub fn is_enabled(&self, hook: HookEvent) -> bool {
        self.enabled_hooks
            .get(hook.as_str())
            .copied()
            .unwrap_or(true)
    }

    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let cfg: Config = toml::from_str(&text).map_err(ConfigError::Parse)?;
                Ok(cfg)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let cfg = Config::default();
                cfg.save(path)?;
                Ok(cfg)
            }
            Err(e) => Err(ConfigError::Io(e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(ConfigError::Io)?;
        }
        let text = toml::to_string_pretty(self).map_err(ConfigError::Serialize)?;
        std::fs::write(path, text).map_err(ConfigError::Io)?;
        Ok(())
    }

    pub fn default_path() -> Option<std::path::PathBuf> {
        let dirs = directories::ProjectDirs::from("", "", "roaring-crab")?;
        Some(dirs.config_dir().join("config.toml"))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("toml parse error: {0}")]
    Parse(toml::de::Error),
    #[error("toml serialize error: {0}")]
    Serialize(toml::ser::Error),
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test config_io`
Expected: All 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/config.rs tests/config_io.rs
git commit -m "Add TOML config with defaults and per-hook enable map"
```

---

## Task 5: Socket path resolver

**Files:**
- Replace: `src/lib.rs` (un-stub `socket_path`)
- Create: `src/socket_path.rs`

(No standalone test — exercised indirectly via integration tests in Task 17.)

- [ ] **Step 1: Implement socket path**

In `src/lib.rs`, replace `pub mod socket_path { }` with `pub mod socket_path;`.

`src/socket_path.rs`:
```rust
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
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build`
Expected: Success on current platform.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs src/socket_path.rs
git commit -m "Add cross-platform socket address resolver"
```

---

## Task 6: Lockfile module

**Files:**
- Replace: `src/lib.rs` (un-stub `lockfile`)
- Create: `src/lockfile.rs`
- Create: `tests/lockfile.rs`

- [ ] **Step 1: Write failing tests**

`tests/lockfile.rs`:
```rust
use roaring_crab::lockfile::{Lock, LockResult};
use tempfile::TempDir;

#[test]
fn acquire_succeeds_when_file_missing() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("rc.lock");
    let result = Lock::try_acquire(&path).unwrap();
    assert!(matches!(result, LockResult::Acquired(_)));
}

#[test]
fn second_acquire_with_live_pid_is_busy() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("rc.lock");
    let _guard = match Lock::try_acquire(&path).unwrap() {
        LockResult::Acquired(g) => g,
        _ => panic!("first acquire failed"),
    };
    let second = Lock::try_acquire(&path).unwrap();
    assert!(matches!(second, LockResult::Busy));
}

#[test]
fn stale_lock_with_dead_pid_is_reclaimed() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("rc.lock");
    // Write a clearly-dead PID. 0xFFFFFFFF is reserved on Windows; on Unix
    // /proc/<that> won't exist.
    std::fs::write(&path, "4294967295\n").unwrap();
    let result = Lock::try_acquire(&path).unwrap();
    assert!(matches!(result, LockResult::Acquired(_)));
}

#[test]
fn dropping_guard_releases_lock() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("rc.lock");
    {
        let _g = match Lock::try_acquire(&path).unwrap() {
            LockResult::Acquired(g) => g,
            _ => panic!(),
        };
    }
    let result = Lock::try_acquire(&path).unwrap();
    assert!(matches!(result, LockResult::Acquired(_)));
}
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test --test lockfile`
Expected: FAIL — `lockfile` module is empty.

- [ ] **Step 3: Implement lockfile**

In `src/lib.rs`, replace `pub mod lockfile { }` with `pub mod lockfile;`.

`src/lockfile.rs`:
```rust
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
    // kill(pid, 0) returns 0 if the process exists and we have permission to signal it.
    // Returns ESRCH if no such process. Any other errno means the process exists.
    unsafe {
        let res = libc::kill(pid as libc::pid_t, 0);
        if res == 0 {
            return true;
        }
        let errno = *libc::__errno_location();
        errno != libc::ESRCH
    }
}

#[cfg(target_os = "macos")]
fn pid_alive_macos(pid: u32) -> bool {
    // macOS uses __error() instead of __errno_location()
    unsafe {
        let res = libc::kill(pid as libc::pid_t, 0);
        if res == 0 {
            return true;
        }
        let errno = *libc::__error();
        errno != libc::ESRCH
    }
}

#[cfg(windows)]
fn pid_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, STILL_ACTIVE,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if handle.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut code);
        CloseHandle(handle);
        ok != 0 && code as i32 == STILL_ACTIVE as i32
    }
}
```

Note: the macOS branch above uses a different errno function. Consolidate by using the `errno` crate, OR adjust the `#[cfg]` blocks so the Linux version uses `__errno_location` and macOS uses `__error`. Simplest fix: add `errno = "0.3"` to deps and use `errno::errno().0`. Update Cargo.toml accordingly and use:

```rust
#[cfg(unix)]
fn pid_alive(pid: u32) -> bool {
    unsafe {
        if libc::kill(pid as libc::pid_t, 0) == 0 {
            return true;
        }
        errno::errno().0 != libc::ESRCH
    }
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test lockfile`
Expected: All 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/lockfile.rs tests/lockfile.rs Cargo.toml Cargo.lock
git commit -m "Add pidfile-based lock with stale process detection"
```

---

## Task 7: Logging module

**Files:**
- Replace: `src/lib.rs` (un-stub `logging`)
- Create: `src/logging.rs`
- Create: `tests/logging.rs`

- [ ] **Step 1: Write failing tests**

`tests/logging.rs`:
```rust
use roaring_crab::logging::{RateLimiter, RollingLog};
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn rate_limiter_allows_first_then_suppresses_within_window() {
    let mut rl = RateLimiter::new(Duration::from_secs(60));
    assert!(rl.allow("key-A"));
    assert!(!rl.allow("key-A"));
    assert!(rl.allow("key-B"));
}

#[test]
fn rate_limiter_allows_again_after_window() {
    let mut rl = RateLimiter::new(Duration::from_millis(10));
    assert!(rl.allow("k"));
    std::thread::sleep(Duration::from_millis(20));
    assert!(rl.allow("k"));
}

#[test]
fn rolling_log_writes_and_rolls_at_size_cap() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("daemon.log");
    let cap = 200u64;
    let mut log = RollingLog::open(&path, cap).unwrap();
    for _ in 0..50 {
        log.write_line("a line of text that adds up").unwrap();
    }
    // Either current or old should exist; total size of current <= cap.
    let cur_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    assert!(cur_size <= cap, "current log exceeded cap: {}", cur_size);
    let old = path.with_extension("log.old");
    assert!(old.exists(), "expected rolled-over .log.old to exist");
}
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test --test logging`
Expected: FAIL — `logging` module is empty.

- [ ] **Step 3: Implement logging**

In `src/lib.rs`, replace `pub mod logging { }` with `pub mod logging;`.

`src/logging.rs`:
```rust
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
        Self { window, last_seen: HashMap::new() }
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
        Ok(Self { path: path.to_path_buf(), cap_bytes, file })
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
        // Close current handle, rename, reopen.
        drop(std::mem::replace(
            &mut self.file,
            File::create(self.path.with_extension("log.tmp"))?,
        ));
        if old.exists() {
            std::fs::remove_file(&old)?;
        }
        std::fs::rename(&self.path, &old)?;
        std::fs::rename(self.path.with_extension("log.tmp"), &self.path)?;
        self.file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        Ok(())
    }
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test logging`
Expected: All 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/logging.rs tests/logging.rs
git commit -m "Add rate-limited logger and rolling daemon log"
```

---

## Task 8: Audio sink abstraction

**Files:**
- Replace: `src/lib.rs` (un-stub `audio_sink`)
- Create: `src/audio_sink.rs`

(No standalone test — exercised via mixer/daemon integration tests.)

- [ ] **Step 1: Implement audio sink**

In `src/lib.rs`, replace `pub mod audio_sink { }` with `pub mod audio_sink;`.

`src/audio_sink.rs`:
```rust
//! Audio output abstraction.
//!
//! In the default build, opens a cpal output stream and writes interleaved stereo f32.
//! When built with `--features null-audio`, swaps in an in-memory ring buffer used by tests.

use std::sync::Arc;

pub trait AudioSink: Send + Sync {
    /// Sample rate, in Hz.
    fn sample_rate(&self) -> u32;
    /// Number of channels (always 2 for now).
    fn channels(&self) -> u16 { 2 }
}

/// Callback signature: filled by the mixer to write the next chunk of interleaved stereo samples.
pub type AudioCallback = Box<dyn FnMut(&mut [f32]) + Send>;

#[cfg(not(feature = "null-audio"))]
pub mod real {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    pub struct CpalSink {
        _stream: cpal::Stream,
        sample_rate: u32,
    }

    impl CpalSink {
        pub fn open(mut callback: AudioCallback) -> anyhow::Result<Arc<Self>> {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or_else(|| anyhow::anyhow!("no default output device"))?;
            let config: cpal::StreamConfig = device.default_output_config()?.into();
            let sample_rate = config.sample_rate.0;
            let stream = device.build_output_stream(
                &config,
                move |buf: &mut [f32], _| callback(buf),
                |err| eprintln!("cpal stream error: {}", err),
                None,
            )?;
            stream.play()?;
            Ok(Arc::new(Self { _stream: stream, sample_rate }))
        }
    }

    impl AudioSink for CpalSink {
        fn sample_rate(&self) -> u32 { self.sample_rate }
    }
}

#[cfg(feature = "null-audio")]
pub mod null {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    pub struct NullSink {
        pub captured: Arc<Mutex<Vec<f32>>>,
        sample_rate: u32,
    }

    impl NullSink {
        pub fn open(mut callback: AudioCallback, sample_rate: u32) -> Arc<Self> {
            let captured = Arc::new(Mutex::new(Vec::new()));
            let cap_clone = captured.clone();
            // Spawn a background thread that pumps the callback periodically.
            std::thread::spawn(move || {
                let chunk = vec![0.0f32; 512 * 2];
                let mut chunk = chunk;
                loop {
                    for s in &mut chunk { *s = 0.0; }
                    callback(&mut chunk);
                    cap_clone.lock().extend_from_slice(&chunk);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            });
            Arc::new(Self { captured, sample_rate })
        }
    }

    impl AudioSink for NullSink {
        fn sample_rate(&self) -> u32 { self.sample_rate }
    }
}
```

- [ ] **Step 2: Verify both feature variants build**

Run: `cargo build`
Expected: Success.
Run: `cargo build --features null-audio`
Expected: Success.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs src/audio_sink.rs
git commit -m "Add audio sink abstraction with null-audio test backend"
```

---

## Task 9: Mixer

**Files:**
- Replace: `src/lib.rs` (un-stub `mixer`)
- Create: `src/mixer.rs`
- Create: `tests/mixer_concurrency.rs`

- [ ] **Step 1: Write failing tests**

`tests/mixer_concurrency.rs`:
```rust
use roaring_crab::mixer::{Mixer, Voice, MAX_VOICES};

fn silent_voice(samples: u32) -> Voice {
    Voice::from_fn(samples, |_t| (0.0, 0.0))
}

fn constant_voice(samples: u32, value: f32) -> Voice {
    Voice::from_fn(samples, move |_t| (value, value))
}

#[test]
fn empty_mixer_produces_silence() {
    let mut m = Mixer::new(48000);
    let mut buf = vec![1.0f32; 100];
    m.render(&mut buf);
    assert!(buf.iter().all(|s| *s == 0.0));
}

#[test]
fn finished_voices_are_removed() {
    let mut m = Mixer::new(48000);
    m.push(silent_voice(2));
    let mut buf = vec![0.0; 2 * 2]; // 2 frames of stereo
    m.render(&mut buf);
    m.render(&mut buf);
    assert_eq!(m.voice_count(), 0);
}

#[test]
fn voice_cap_is_enforced() {
    let mut m = Mixer::new(48000);
    for _ in 0..(MAX_VOICES + 5) {
        m.push(silent_voice(48000));
    }
    assert_eq!(m.voice_count(), MAX_VOICES);
}

#[test]
fn master_volume_scales_output() {
    let mut m = Mixer::new(48000);
    m.set_master_volume(0.5);
    m.push(constant_voice(10, 1.0));
    let mut buf = vec![0.0; 2 * 2];
    m.render(&mut buf);
    for s in &buf {
        assert!((s - 0.5).abs() < 1e-6, "got {}", s);
    }
}

#[test]
fn output_is_clamped_to_unit_range() {
    let mut m = Mixer::new(48000);
    for _ in 0..5 {
        m.push(constant_voice(10, 1.0)); // 5x sum = 5.0 before clamp
    }
    let mut buf = vec![0.0; 2 * 2];
    m.render(&mut buf);
    for s in &buf {
        assert!(*s <= 1.0 && *s >= -1.0, "unclamped: {}", s);
    }
}
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test --test mixer_concurrency`
Expected: FAIL — `mixer` module is empty.

- [ ] **Step 3: Implement mixer**

In `src/lib.rs`, replace `pub mod mixer { }` with `pub mod mixer;`.

`src/mixer.rs`:
```rust
use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub const MAX_VOICES: usize = 16;

/// A single voice produces (left, right) samples per call. When `samples_remaining`
/// hits zero, the voice is finished and the mixer removes it.
pub struct Voice {
    samples_remaining: u32,
    t_samples: u32,
    f: Box<dyn FnMut(u32) -> (f32, f32) + Send>,
}

impl Voice {
    pub fn from_fn<F: FnMut(u32) -> (f32, f32) + Send + 'static>(
        samples: u32,
        f: F,
    ) -> Self {
        Self {
            samples_remaining: samples,
            t_samples: 0,
            f: Box::new(f),
        }
    }

    fn pump(&mut self) -> Option<(f32, f32)> {
        if self.samples_remaining == 0 {
            return None;
        }
        let s = (self.f)(self.t_samples);
        self.t_samples += 1;
        self.samples_remaining -= 1;
        Some(s)
    }
}

pub struct Mixer {
    sample_rate: u32,
    voices: Arc<Mutex<Vec<Voice>>>,
    master_volume_q15: AtomicU32, // store as fixed-point to avoid atomic-f32 dance
}

impl Mixer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            voices: Arc::new(Mutex::new(Vec::new())),
            master_volume_q15: AtomicU32::new(((0.7f32 * 32768.0) as u32) & 0xFFFF),
        }
    }

    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn voice_count(&self) -> usize { self.voices.lock().len() }

    pub fn set_master_volume(&self, v: f32) {
        let q = ((v.clamp(0.0, 1.0) * 32768.0) as u32) & 0xFFFF;
        self.master_volume_q15.store(q, Ordering::Relaxed);
    }

    pub fn master_volume(&self) -> f32 {
        (self.master_volume_q15.load(Ordering::Relaxed) as f32) / 32768.0
    }

    pub fn push(&self, voice: Voice) {
        let mut voices = self.voices.lock();
        if voices.len() >= MAX_VOICES {
            voices.remove(0); // drop oldest
        }
        voices.push(voice);
    }

    /// Fill `buf` with interleaved stereo samples.
    pub fn render(&self, buf: &mut [f32]) {
        let vol = self.master_volume();
        let mut voices = self.voices.lock();
        let frames = buf.len() / 2;
        for frame in 0..frames {
            let mut l = 0.0f32;
            let mut r = 0.0f32;
            for v in voices.iter_mut() {
                if let Some((vl, vr)) = v.pump() {
                    l += vl;
                    r += vr;
                }
            }
            buf[frame * 2] = (l * vol).clamp(-1.0, 1.0);
            buf[frame * 2 + 1] = (r * vol).clamp(-1.0, 1.0);
        }
        voices.retain(|v| v.samples_remaining > 0);
    }
}

impl Default for Mixer {
    fn default() -> Self { Self::new(48000) }
}
```

The `render` is exposed by `&self` (atomic + Mutex inside) so the cpal callback closure can hold an `Arc<Mixer>`.

For the tests above, `Mixer::new(48000)` is bound via `let mut m =` — that's a misuse of `&mut self`; rewrite the tests to use `let m = Mixer::new(...)`:

```rust
let m = Mixer::new(48000);
let mut buf = vec![1.0f32; 100];
m.render(&mut buf);
```

Update tests accordingly (remove the `mut` on `m`).

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test mixer_concurrency`
Expected: All 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/mixer.rs tests/mixer_concurrency.rs
git commit -m "Add voice mixer with cap, clamping, and master volume"
```

---

## Task 10: Patches — shared helpers

**Files:**
- Replace: `src/lib.rs` (un-stub `patches`)
- Create: `src/patches/mod.rs`

- [ ] **Step 1: Implement patches module skeleton + dispatch**

In `src/lib.rs`, replace `pub mod patches { }` with `pub mod patches;`.

`src/patches/mod.rs`:
```rust
//! Per-hook sound patches. Each patch is a function returning a `Voice` for the mixer.
//!
//! Patches are organized into three families:
//! - ambient: PreToolUse, PostToolUse, UserPromptSubmit (short blips, 100–200ms)
//! - lifecycle: SessionStart, SessionEnd, PreCompact (warm sweeps, 600–1000ms)
//! - alert: Notification, Stop, SubagentStop (melodic motifs, 400–800ms)

use crate::hook_event::HookEvent;
use crate::mixer::Voice;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub mod ambient;
pub mod lifecycle;
pub mod alert;

/// Produces a voice for the given hook event using the given RNG seed.
pub fn build(event: HookEvent, seed: u64, sample_rate: u32) -> Voice {
    match event {
        HookEvent::PreToolUse => ambient::pre_tool_use(seed, sample_rate),
        HookEvent::PostToolUse => ambient::post_tool_use(seed, sample_rate),
        HookEvent::UserPromptSubmit => ambient::user_prompt_submit(seed, sample_rate),
        HookEvent::SessionStart => lifecycle::session_start(seed, sample_rate),
        HookEvent::SessionEnd => lifecycle::session_end(seed, sample_rate),
        HookEvent::PreCompact => lifecycle::pre_compact(seed, sample_rate),
        HookEvent::Notification => alert::notification(seed, sample_rate),
        HookEvent::Stop => alert::stop(seed, sample_rate),
        HookEvent::SubagentStop => alert::subagent_stop(seed, sample_rate),
    }
}

/// ADSR envelope. Outputs a multiplier in [0, 1] given the current sample index and total length.
pub(crate) fn adsr(t: u32, total: u32, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
    let tn = t as f32 / total as f32;
    if tn < attack {
        tn / attack
    } else if tn < attack + decay {
        let dt = (tn - attack) / decay;
        1.0 + (sustain - 1.0) * dt
    } else if tn < 1.0 - release {
        sustain
    } else {
        let rt = (tn - (1.0 - release)) / release;
        sustain * (1.0 - rt)
    }
}

pub(crate) fn rng_for(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

/// Quick utility: hz value for sample index.
pub(crate) fn phase(t: u32, sample_rate: u32, hz: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    two_pi * (t as f32 / sample_rate as f32) * hz
}
```

- [ ] **Step 2: Add submodule stubs so the crate builds**

`src/patches/ambient.rs`:
```rust
use crate::mixer::Voice;

pub fn pre_tool_use(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
pub fn post_tool_use(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
pub fn user_prompt_submit(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
```

`src/patches/lifecycle.rs`:
```rust
use crate::mixer::Voice;

pub fn session_start(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
pub fn session_end(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
pub fn pre_compact(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
```

`src/patches/alert.rs`:
```rust
use crate::mixer::Voice;

pub fn notification(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
pub fn stop(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
pub fn subagent_stop(_seed: u64, _sr: u32) -> Voice { Voice::from_fn(1, |_| (0.0, 0.0)) }
```

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: Success.

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs src/patches/
git commit -m "Add patches module skeleton with stubs and dispatcher"
```

---

## Task 11: Ambient patches (3 patches with shared TDD contract)

**Files:**
- Modify: `src/patches/ambient.rs`
- Create: `tests/patches_render.rs` (covers all 9 patches; built incrementally)

- [ ] **Step 1: Write the patch contract test, applied to the three ambient patches**

`tests/patches_render.rs`:
```rust
use roaring_crab::mixer::{Mixer, Voice};
use roaring_crab::patches;
use roaring_crab::hook_event::HookEvent;

const SR: u32 = 48000;

/// Render the voice into a buffer until it's finished. Returns the buffer.
fn render_voice(voice: Voice) -> Vec<f32> {
    let m = Mixer::new(SR);
    m.set_master_volume(1.0);
    m.push(voice);
    let mut out = Vec::with_capacity(SR as usize * 2 * 2);
    while m.voice_count() > 0 {
        let mut chunk = vec![0.0f32; 256 * 2];
        m.render(&mut chunk);
        out.extend_from_slice(&chunk);
    }
    out
}

fn assert_patch_contract(event: HookEvent, seed: u64) {
    let voice = patches::build(event, seed, SR);
    let samples = render_voice(voice);
    assert!(!samples.is_empty(), "{:?}: empty render", event);
    for s in &samples {
        assert!(s.is_finite(), "{:?}: NaN/inf sample", event);
        assert!(s.abs() <= 1.0, "{:?}: sample out of range: {}", event, s);
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!(rms > 0.001, "{:?}: silent output (rms={})", event, rms);
}

fn assert_variation(event: HookEvent) {
    let a = render_voice(patches::build(event, 1, SR));
    let b = render_voice(patches::build(event, 999, SR));
    // Compare in coarse buckets — two distinct seeds should not produce identical audio.
    let same = a.len() == b.len()
        && a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-7);
    assert!(!same, "{:?}: same audio for different seeds", event);
}

#[test] fn pre_tool_use_contract() { assert_patch_contract(HookEvent::PreToolUse, 1); }
#[test] fn pre_tool_use_varies()   { assert_variation(HookEvent::PreToolUse); }
#[test] fn post_tool_use_contract() { assert_patch_contract(HookEvent::PostToolUse, 1); }
#[test] fn post_tool_use_varies()   { assert_variation(HookEvent::PostToolUse); }
#[test] fn user_prompt_submit_contract() { assert_patch_contract(HookEvent::UserPromptSubmit, 1); }
#[test] fn user_prompt_submit_varies()   { assert_variation(HookEvent::UserPromptSubmit); }
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test --test patches_render`
Expected: FAIL (current stubs produce silence, fail contract).

- [ ] **Step 3: Implement ambient patches**

Replace `src/patches/ambient.rs`:
```rust
use crate::mixer::Voice;
use crate::patches::{adsr, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 160;

fn dur_samples(sample_rate: u32) -> u32 { (DUR_MS * sample_rate / 1000).max(1) }

/// Short filtered blip. Slightly different center frequency + filter sweep per patch.
fn ambient_blip(seed: u64, sample_rate: u32, center_hz: f32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-12.0..12.0);
    let cutoff_drift: f32 = rng.gen_range(0.5..1.5);
    let base = center_hz * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.10, 0.30, 0.4, 0.60);
        let p1 = phase(t, sample_rate, base);
        let p2 = phase(t, sample_rate, base * 2.0);
        // Simple lowpass-ish: blend a triangle (filtered fundamental) and saw harmonic.
        let tri = 2.0 / std::f32::consts::PI * p1.sin();
        let saw = (p2.sin() * 0.3) * cutoff_drift;
        let sample = 0.18 * env * (tri + saw);
        (sample, sample)
    })
}

pub fn pre_tool_use(seed: u64, sample_rate: u32) -> Voice {
    ambient_blip(seed, sample_rate, 660.0)
}

pub fn post_tool_use(seed: u64, sample_rate: u32) -> Voice {
    ambient_blip(seed, sample_rate, 880.0)
}

pub fn user_prompt_submit(seed: u64, sample_rate: u32) -> Voice {
    ambient_blip(seed, sample_rate, 1100.0)
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test patches_render -- pre_tool_use post_tool_use user_prompt_submit`
Expected: 6 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/patches/ambient.rs tests/patches_render.rs
git commit -m "Add ambient patches (PreToolUse, PostToolUse, UserPromptSubmit)"
```

---

## Task 12: Lifecycle patches

**Files:**
- Modify: `src/patches/lifecycle.rs`
- Modify: `tests/patches_render.rs` (add 6 tests)

- [ ] **Step 1: Add failing tests for lifecycle patches**

Append to `tests/patches_render.rs`:
```rust
#[test] fn session_start_contract() { assert_patch_contract(HookEvent::SessionStart, 1); }
#[test] fn session_start_varies()   { assert_variation(HookEvent::SessionStart); }
#[test] fn session_end_contract() { assert_patch_contract(HookEvent::SessionEnd, 1); }
#[test] fn session_end_varies()   { assert_variation(HookEvent::SessionEnd); }
#[test] fn pre_compact_contract() { assert_patch_contract(HookEvent::PreCompact, 1); }
#[test] fn pre_compact_varies()   { assert_variation(HookEvent::PreCompact); }
```

- [ ] **Step 2: Verify they fail**

Run: `cargo test --test patches_render -- session pre_compact`
Expected: FAIL (stubs).

- [ ] **Step 3: Implement lifecycle patches**

Replace `src/patches/lifecycle.rs`:
```rust
use crate::mixer::Voice;
use crate::patches::{adsr, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 800;

fn dur_samples(sample_rate: u32) -> u32 { (DUR_MS * sample_rate / 1000).max(1) }

/// Warm pad: stacked sines an octave apart with slow tremolo. `direction = +1` ramps up, `-1` ramps down.
fn pad(seed: u64, sample_rate: u32, root_hz: f32, direction: i32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-3.0..3.0);
    let trem_hz: f32 = rng.gen_range(3.0..6.0);
    let root = root_hz * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let mut env = adsr(t, total, 0.25, 0.20, 0.7, 0.35);
        if direction < 0 {
            // For session_end, invert so it fades down rather than up.
            env *= 1.0 - (t as f32 / total as f32);
        }
        let trem = 0.85 + 0.15 * phase(t, sample_rate, trem_hz).sin();
        let p1 = phase(t, sample_rate, root).sin();
        let p2 = 0.5 * phase(t, sample_rate, root * 2.0).sin();
        let p3 = 0.25 * phase(t, sample_rate, root * 3.0).sin();
        let sample = 0.18 * env * trem * (p1 + p2 + p3) / 1.75;
        (sample * 0.95, sample * 1.05) // slight stereo spread
    })
}

pub fn session_start(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 220.0, 1)
}

pub fn session_end(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 165.0, -1)
}

pub fn pre_compact(seed: u64, sample_rate: u32) -> Voice {
    // Pre-compact = a brief two-step rising motif over a sustained pad
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-5.0..5.0);
    let root = 196.0 * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.15, 0.20, 0.6, 0.40);
        let progress = t as f32 / total as f32;
        let step_hz = if progress < 0.5 { root * 3.0 } else { root * 4.0 };
        let pad_s = phase(t, sample_rate, root).sin() + 0.5 * phase(t, sample_rate, root * 2.0).sin();
        let step = 0.3 * phase(t, sample_rate, step_hz).sin();
        let sample = 0.16 * env * (pad_s / 1.5 + step);
        (sample, sample)
    })
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test patches_render -- session pre_compact`
Expected: 6 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/patches/lifecycle.rs tests/patches_render.rs
git commit -m "Add lifecycle patches (SessionStart, SessionEnd, PreCompact)"
```

---

## Task 13: Alert patches

**Files:**
- Modify: `src/patches/alert.rs`
- Modify: `tests/patches_render.rs` (add 6 tests)

- [ ] **Step 1: Add failing tests**

Append to `tests/patches_render.rs`:
```rust
#[test] fn notification_contract() { assert_patch_contract(HookEvent::Notification, 1); }
#[test] fn notification_varies()   { assert_variation(HookEvent::Notification); }
#[test] fn stop_contract() { assert_patch_contract(HookEvent::Stop, 1); }
#[test] fn stop_varies()   { assert_variation(HookEvent::Stop); }
#[test] fn subagent_stop_contract() { assert_patch_contract(HookEvent::SubagentStop, 1); }
#[test] fn subagent_stop_varies()   { assert_variation(HookEvent::SubagentStop); }
```

- [ ] **Step 2: Verify they fail**

Run: `cargo test --test patches_render -- notification stop subagent_stop`
Expected: FAIL.

- [ ] **Step 3: Implement alert patches**

Replace `src/patches/alert.rs`:
```rust
use crate::mixer::Voice;
use crate::patches::{adsr, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 600;

fn dur_samples(sample_rate: u32) -> u32 { (DUR_MS * sample_rate / 1000).max(1) }

/// Notification: rising 3-note arpeggio in a major-pentatonic-flavored shape.
pub fn notification(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-4.0..4.0);
    let root = 440.0 * 2.0f32.powf(detune / 1200.0);
    let intervals = [1.0f32, 1.2599210, 1.4983071]; // root, M3, P5
    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let idx = (progress * 3.0).floor().clamp(0.0, 2.0) as usize;
        let hz = root * intervals[idx];
        let env = adsr(t, total, 0.05, 0.15, 0.6, 0.45);
        // Each new note re-attacks (small bump in amplitude on note transition).
        let note_phase = (progress * 3.0).fract();
        let note_env = 1.0 - (note_phase - 0.0).max(0.0) * 0.3;
        let s = phase(t, sample_rate, hz).sin();
        let h = 0.3 * phase(t, sample_rate, hz * 2.0).sin();
        let sample = 0.20 * env * note_env * (s + h);
        (sample, sample)
    })
}

/// Stop: resolved chord — root + 5th + octave with a soft attack, longer release.
pub fn stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-3.0..3.0);
    let root = 330.0 * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.08, 0.20, 0.65, 0.55);
        let a = phase(t, sample_rate, root).sin();
        let b = 0.85 * phase(t, sample_rate, root * 1.5).sin();
        let c = 0.6 * phase(t, sample_rate, root * 2.0).sin();
        let sample = 0.16 * env * (a + b + c) / 2.45;
        (sample * 0.97, sample * 1.03)
    })
}

/// SubagentStop: a quieter two-note resolution, M3 → root descending.
pub fn subagent_stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-4.0..4.0);
    let root = 392.0 * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let hz = if progress < 0.45 { root * 1.2599210 } else { root };
        let env = adsr(t, total, 0.04, 0.20, 0.55, 0.45);
        let s = phase(t, sample_rate, hz).sin();
        let h = 0.2 * phase(t, sample_rate, hz * 2.0).sin();
        let sample = 0.14 * env * (s + h);
        (sample, sample)
    })
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test --test patches_render`
Expected: All 18 patch tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/patches/alert.rs tests/patches_render.rs
git commit -m "Add alert patches (Notification, Stop, SubagentStop)"
```

---

## Task 14: Daemon spawn helper

**Files:**
- Replace: `src/lib.rs` (un-stub `spawn`)
- Create: `src/spawn.rs`

- [ ] **Step 1: Implement spawn**

In `src/lib.rs`, replace `pub mod spawn { }` with `pub mod spawn;`.

`src/spawn.rs`:
```rust
//! Spawns the daemon as a detached background process.
//!
//! On Unix: double-fork pattern + setsid so the daemon is reparented to init
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
                // Detach from controlling terminal and parent process group.
                if libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        // Close stdio so the daemon's output doesn't tie back to the client.
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

/// Given the path to the running binary (the client), returns the expected daemon
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
```

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: Success.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs src/spawn.rs
git commit -m "Add cross-platform detached daemon spawn helper"
```

---

## Task 15: Daemon binary main

**Files:**
- Replace: `src/bin/roaring-crabd.rs`

(Will be integration-tested in Task 17 — manual smoke check here.)

- [ ] **Step 1: Implement daemon**

`src/bin/roaring-crabd.rs`:
```rust
use interprocess::local_socket::{traits::ListenerExt, ListenerOptions};
use parking_lot::Mutex;
use roaring_crab::config::Config;
use roaring_crab::lockfile::{Lock, LockResult};
use roaring_crab::logging::{RateLimiter, RollingLog};
use roaring_crab::mixer::Mixer;
use roaring_crab::patches;
use roaring_crab::protocol::read_frame;
use roaring_crab::socket_path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn log_dir() -> std::path::PathBuf {
    Config::default_path()
        .and_then(|p| p.parent().map(|x| x.to_path_buf()))
        .unwrap_or_else(|| std::env::temp_dir())
}

fn idle_timeout() -> Duration {
    let secs = std::env::var("RC_IDLE_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300);
    Duration::from_secs(secs)
}

fn use_null_audio() -> bool {
    cfg!(feature = "null-audio") || std::env::var_os("RC_NULL_AUDIO").is_some()
}

fn lockfile_path() -> std::path::PathBuf {
    socket_path::socket_fs_path()
        .map(|p| p.with_extension("lock"))
        .unwrap_or_else(|| std::env::temp_dir().join("roaring-crab.lock"))
}

fn main() {
    let log_path = log_dir().join("roaring-crabd.log");
    let mut log = match RollingLog::open(&log_path, 1_000_000) {
        Ok(l) => l,
        Err(_) => return, // can't log → exit silently
    };
    let mut rl = RateLimiter::new(Duration::from_secs(60));
    let mut warn = |key: &str, msg: &str, log: &mut RollingLog| {
        if rl.allow(key) {
            let _ = log.write_line(msg);
        }
    };

    let _lock = match Lock::try_acquire(&lockfile_path()) {
        Ok(LockResult::Acquired(g)) => g,
        Ok(LockResult::Busy) => {
            let _ = log.write_line("another daemon already running, exiting");
            return;
        }
        Err(e) => {
            let _ = log.write_line(&format!("lockfile error: {}", e));
            return;
        }
    };

    let mixer = Arc::new(Mixer::new(48000));
    let last_event = Arc::new(AtomicI64::new(Instant::now_micros()));

    // Audio backend
    let mixer_for_cb = mixer.clone();
    let callback: roaring_crab::audio_sink::AudioCallback =
        Box::new(move |buf: &mut [f32]| mixer_for_cb.render(buf));

    let _sink: Arc<dyn roaring_crab::audio_sink::AudioSink> = if use_null_audio() {
        #[cfg(feature = "null-audio")]
        { roaring_crab::audio_sink::null::NullSink::open(callback, 48000) }
        #[cfg(not(feature = "null-audio"))]
        {
            let _ = log.write_line("null-audio requested but feature not compiled in");
            return;
        }
    } else {
        #[cfg(not(feature = "null-audio"))]
        {
            match roaring_crab::audio_sink::real::CpalSink::open(callback) {
                Ok(s) => s,
                Err(e) => {
                    let _ = log.write_line(&format!("cpal open failed: {}", e));
                    return;
                }
            }
        }
        #[cfg(feature = "null-audio")]
        { roaring_crab::audio_sink::null::NullSink::open(callback, 48000) }
    };

    // Accept loop
    let socket = match ListenerOptions::new()
        .name(match socket_path::socket_name() {
            Ok(n) => n,
            Err(e) => { let _ = log.write_line(&format!("socket name: {}", e)); return; }
        })
        .create_sync()
    {
        Ok(l) => l,
        Err(e) => {
            let _ = log.write_line(&format!("socket bind: {}", e));
            return;
        }
    };
    let _ = log.write_line("daemon started");

    let log = Arc::new(Mutex::new(log));
    let mixer_for_accept = mixer.clone();
    let last_event_for_accept = last_event.clone();
    let log_for_accept = log.clone();

    let accept_thread = std::thread::spawn(move || {
        for conn in socket.incoming() {
            match conn {
                Ok(mut stream) => {
                    match read_frame(&mut stream) {
                        Ok(play) => {
                            let voice = patches::build(play.event, play.seed, mixer_for_accept.sample_rate());
                            mixer_for_accept.set_master_volume(play.volume);
                            mixer_for_accept.push(voice);
                            last_event_for_accept.store(Instant::now_micros(), Ordering::Relaxed);
                        }
                        Err(e) => {
                            let _ = log_for_accept.lock().write_line(&format!("frame: {}", e));
                        }
                    }
                }
                Err(e) => {
                    let _ = log_for_accept.lock().write_line(&format!("accept: {}", e));
                }
            }
        }
    });

    // Idle watchdog
    let idle = idle_timeout();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let last = last_event.load(Ordering::Relaxed);
        let elapsed = Duration::from_micros((Instant::now_micros() - last) as u64);
        if elapsed >= idle && mixer.voice_count() == 0 {
            let _ = log.lock().write_line("idle-exiting");
            std::process::exit(0);
        }
    }

    #[allow(unreachable_code)]
    let _ = accept_thread.join();
}

// Tiny helper since stdlib Instant doesn't expose epoch micros directly.
trait InstantExt {
    fn now_micros() -> i64;
}
impl InstantExt for Instant {
    fn now_micros() -> i64 {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros() as i64)
            .unwrap_or(0)
    }
}
```

- [ ] **Step 2: Verify it builds in both configurations**

Run: `cargo build --bin roaring-crabd`
Expected: Success.
Run: `cargo build --bin roaring-crabd --features null-audio`
Expected: Success.

- [ ] **Step 3: Smoke-test the daemon manually**

Run (Unix): `RC_NULL_AUDIO=1 RC_IDLE_SECS=3 RC_SOCKET_PATH=/tmp/rc-smoke.sock cargo run --features null-audio --bin roaring-crabd`
Run (Windows PowerShell): `$env:RC_NULL_AUDIO=1; $env:RC_IDLE_SECS=3; cargo run --features null-audio --bin roaring-crabd`
Expected: Daemon starts, no panic, exits cleanly after ~3 seconds idle.

- [ ] **Step 4: Commit**

```bash
git add src/bin/roaring-crabd.rs
git commit -m "Implement daemon: socket accept, mixer, cpal/null sink, idle exit"
```

---

## Task 16: Client binary main

**Files:**
- Replace: `src/bin/roaring-crab.rs`

- [ ] **Step 1: Implement client**

`src/bin/roaring-crab.rs`:
```rust
use clap::Parser;
use interprocess::local_socket::{traits::Stream as StreamTrait, Stream};
use roaring_crab::config::Config;
use roaring_crab::hook_event::HookEvent;
use roaring_crab::protocol::{write_frame, PlayEvent};
use roaring_crab::socket_path;
use roaring_crab::spawn;
use std::io::Read;
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Cli {
    #[arg(long, value_enum)]
    event: HookEvent,
}

fn drain_stdin() {
    let mut sink = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut sink);
}

fn try_connect_and_send(play: PlayEvent) -> std::io::Result<()> {
    let name = socket_path::socket_name()?;
    let mut stream = Stream::connect(name)?;
    write_frame(&mut stream, &play).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    drain_stdin();

    let cfg_path = match Config::default_path() {
        Some(p) => p,
        None => {
            eprintln!("roaring-crab: no config dir on this platform");
            std::process::exit(0);
        }
    };
    let cfg = match Config::load_or_default(&cfg_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("roaring-crab: config load failed ({}), using defaults", e);
            Config::default()
        }
    };

    if cfg.muted || !cfg.is_enabled(cli.event) {
        return; // silent skip
    }

    let play = PlayEvent {
        event: cli.event,
        seed: rand::random(),
        volume: cfg.master_volume,
    };

    if try_connect_and_send(play).is_ok() {
        return;
    }

    // Daemon probably not running — spawn and retry.
    let client_path = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("roaring-crab"));
    let daemon_path = spawn::daemon_sibling_path(&client_path);
    if let Err(e) = spawn::spawn_daemon(&daemon_path) {
        eprintln!("roaring-crab: daemon spawn failed: {}", e);
        return;
    }

    let deadline = Instant::now() + Duration::from_millis(200);
    while Instant::now() < deadline {
        if try_connect_and_send(play).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    eprintln!("roaring-crab: daemon slow to start, skipping event");
}
```

- [ ] **Step 2: Verify it builds**

Run: `cargo build --bin roaring-crab`
Expected: Success.

- [ ] **Step 3: Smoke-test full client→daemon path manually (Unix)**

```bash
# Terminal A:
RC_NULL_AUDIO=1 RC_IDLE_SECS=30 RC_SOCKET_PATH=/tmp/rc-smoke.sock \
  cargo run --features null-audio --bin roaring-crabd
# Terminal B (while A is running):
RC_SOCKET_PATH=/tmp/rc-smoke.sock cargo run --bin roaring-crab -- --event Stop
```

Expected: client exits immediately, daemon log shows "frame received" / mixer voice added.

- [ ] **Step 4: Commit**

```bash
git add src/bin/roaring-crab.rs
git commit -m "Implement client: drain stdin, load config, send PlayEvent, lazy-spawn daemon"
```

---

## Task 17: Integration test — client to daemon (in-process)

**Files:**
- Create: `tests/client_to_daemon.rs`

- [ ] **Step 1: Write the test**

`tests/client_to_daemon.rs`:
```rust
//! Drives the wire protocol end-to-end in-process: spawn the daemon as a child,
//! connect from a test, send a PlayEvent, verify the daemon log mentions it.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn cargo_bin(name: &str) -> std::path::PathBuf {
    // The cargo test runner places binaries under target/debug/.
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    if cfg!(windows) {
        p.push(format!("{}.exe", name));
    } else {
        p.push(name);
    }
    p
}

#[test]
#[cfg(unix)] // Socket-path override is a real fs path; Windows uses named pipes.
fn daemon_accepts_play_event_over_socket() {
    let tmp = TempDir::new().unwrap();
    let socket = tmp.path().join("rc.sock");
    let config_dir = tmp.path().join("conf");

    // Pre-build binaries
    let status = Command::new(env!("CARGO"))
        .args(&["build", "--features", "null-audio", "--bin", "roaring-crabd"])
        .status()
        .unwrap();
    assert!(status.success());

    let mut daemon = Command::new(cargo_bin("roaring-crabd"))
        .env("RC_SOCKET_PATH", &socket)
        .env("RC_NULL_AUDIO", "1")
        .env("RC_IDLE_SECS", "30")
        .env("XDG_CONFIG_HOME", &config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Wait for socket to appear.
    let deadline = Instant::now() + Duration::from_secs(2);
    while !socket.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(socket.exists(), "daemon did not create socket");

    // Connect and send a frame.
    use roaring_crab::hook_event::HookEvent;
    use roaring_crab::protocol::{write_frame, PlayEvent};
    use interprocess::local_socket::{traits::Stream as StreamTrait, GenericFilePath, Stream, ToFsName};
    let name = socket.to_str().unwrap().to_fs_name::<GenericFilePath>().unwrap();
    let mut stream = Stream::connect(name).unwrap();
    let play = PlayEvent { event: HookEvent::Stop, seed: 42, volume: 0.5 };
    write_frame(&mut stream, &play).unwrap();
    drop(stream);

    // Give the daemon a beat to process.
    std::thread::sleep(Duration::from_millis(200));

    daemon.kill().unwrap();
    let _ = daemon.wait();
}
```

- [ ] **Step 2: Run and verify it passes**

Run: `cargo test --features null-audio --test client_to_daemon`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/client_to_daemon.rs
git commit -m "Add integration test: daemon accepts PlayEvent over socket"
```

---

## Task 18: Integration test — lazy spawn

**Files:**
- Create: `tests/lazy_spawn.rs`

- [ ] **Step 1: Write the test**

`tests/lazy_spawn.rs`:
```rust
//! Verifies: no daemon running → invoking the client spawns one → socket becomes
//! responsive → daemon idle-exits after `RC_IDLE_SECS`.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn cargo_bin(name: &str) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    if cfg!(windows) { p.push(format!("{}.exe", name)); } else { p.push(name); }
    p
}

#[test]
#[cfg(unix)]
fn client_spawns_daemon_when_socket_missing_and_daemon_idle_exits() {
    let tmp = TempDir::new().unwrap();
    let socket = tmp.path().join("rc.sock");
    let config_dir = tmp.path().join("conf");

    let status = Command::new(env!("CARGO"))
        .args(&["build", "--features", "null-audio"])
        .status()
        .unwrap();
    assert!(status.success());

    // No daemon running. Invoke client; it should spawn the daemon.
    let client_status = Command::new(cargo_bin("roaring-crab"))
        .args(&["--event", "Stop"])
        .env("RC_SOCKET_PATH", &socket)
        .env("RC_NULL_AUDIO", "1")
        .env("RC_IDLE_SECS", "2")
        .env("XDG_CONFIG_HOME", &config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(client_status.success());

    // Socket should exist now (created by daemon).
    let deadline = Instant::now() + Duration::from_secs(2);
    while !socket.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(socket.exists());

    // Wait for daemon to idle-exit.
    std::thread::sleep(Duration::from_secs(3));
    // After idle exit, socket file may be removed.
    // Just verify a subsequent connect fails (or socket is gone).
    let still_alive = std::os::unix::net::UnixStream::connect(&socket).is_ok();
    assert!(!still_alive, "daemon should have idle-exited");
}
```

- [ ] **Step 2: Run and verify**

Run: `cargo test --features null-audio --test lazy_spawn`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/lazy_spawn.rs
git commit -m "Add integration test: lazy daemon spawn + idle exit"
```

---

## Task 19: Integration test — concurrent events

**Files:**
- Create: `tests/concurrent_events.rs`

- [ ] **Step 1: Write the test**

`tests/concurrent_events.rs`:
```rust
//! Fire 20 client invocations concurrently; assert no crash, no deadlock,
//! and the daemon survives.

use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn cargo_bin(name: &str) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    if cfg!(windows) { p.push(format!("{}.exe", name)); } else { p.push(name); }
    p
}

#[test]
#[cfg(unix)]
fn twenty_concurrent_events_do_not_crash_daemon() {
    let tmp = TempDir::new().unwrap();
    let socket = Arc::new(tmp.path().join("rc.sock"));
    let config_dir = Arc::new(tmp.path().join("conf"));

    let status = Command::new(env!("CARGO"))
        .args(&["build", "--features", "null-audio"])
        .status()
        .unwrap();
    assert!(status.success());

    // Pre-spawn the daemon.
    let mut daemon = Command::new(cargo_bin("roaring-crabd"))
        .env("RC_SOCKET_PATH", &*socket)
        .env("RC_NULL_AUDIO", "1")
        .env("RC_IDLE_SECS", "60")
        .env("XDG_CONFIG_HOME", &*config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    while !socket.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(socket.exists());

    let mut handles = Vec::new();
    for _ in 0..20 {
        let socket = socket.clone();
        let config_dir = config_dir.clone();
        handles.push(std::thread::spawn(move || {
            Command::new(cargo_bin("roaring-crab"))
                .args(&["--event", "PreToolUse"])
                .env("RC_SOCKET_PATH", &*socket)
                .env("XDG_CONFIG_HOME", &*config_dir)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap()
        }));
    }
    for h in handles {
        let status = h.join().unwrap();
        assert!(status.success(), "client failed");
    }

    // Daemon should still be alive.
    match daemon.try_wait() {
        Ok(None) => {}
        other => panic!("daemon exited unexpectedly: {:?}", other),
    }
    daemon.kill().unwrap();
    let _ = daemon.wait();
}
```

- [ ] **Step 2: Run and verify**

Run: `cargo test --features null-audio --test concurrent_events`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/concurrent_events.rs
git commit -m "Add integration test: 20 concurrent events"
```

---

## Task 20: Plugin manifest and launcher scripts

**Files:**
- Create: `hooks.json`
- Create: `bin/launch.sh`
- Create: `bin/launch.cmd`

- [ ] **Step 1: Write `bin/launch.sh`**

```sh
#!/usr/bin/env bash
# roaring-crab Unix launcher. Picks the right prebuilt binary based on OS+arch.
set -e

EVENT="$1"
if [ -z "$EVENT" ]; then
  echo "usage: launch.sh <HookEvent>" >&2
  exit 2
fi

case "$(uname -s)" in
  Linux*)  OS=linux ;;
  Darwin*) OS=macos ;;
  *) exit 0 ;;  # unsupported, fail silently to not break hooks
esac

case "$(uname -m)" in
  x86_64|amd64) ARCH=x86_64 ;;
  arm64|aarch64) ARCH=aarch64 ;;
  *) exit 0 ;;
esac

BIN="${CLAUDE_PLUGIN_ROOT:-$(dirname "$0")/..}/bin/${OS}-${ARCH}/roaring-crab"
if [ ! -x "$BIN" ]; then
  exit 0  # binary missing for platform → silent skip
fi
exec "$BIN" --event "$EVENT"
```

- [ ] **Step 2: Write `bin/launch.cmd`**

```bat
@echo off
rem roaring-crab Windows launcher.
setlocal
if "%~1"=="" exit /b 2

set "EVENT=%~1"
set "BIN=%CLAUDE_PLUGIN_ROOT%\bin\windows-x86_64\roaring-crab.exe"
if not exist "%BIN%" exit /b 0
"%BIN%" --event %EVENT%
```

- [ ] **Step 3: Write `hooks.json`**

```json
{
  "hooks": {
    "SessionStart": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" SessionStart" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" SessionStart" }
    ],
    "SessionEnd": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" SessionEnd" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" SessionEnd" }
    ],
    "UserPromptSubmit": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" UserPromptSubmit" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" UserPromptSubmit" }
    ],
    "PreToolUse": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" PreToolUse" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" PreToolUse" }
    ],
    "PostToolUse": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" PostToolUse" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" PostToolUse" }
    ],
    "Notification": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" Notification" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" Notification" }
    ],
    "Stop": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" Stop" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" Stop" }
    ],
    "SubagentStop": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" SubagentStop" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" SubagentStop" }
    ],
    "PreCompact": [
      { "type": "command", "command": "bash \"${CLAUDE_PLUGIN_ROOT}/bin/launch.sh\" PreCompact" },
      { "type": "command", "command": "cmd /c \"%CLAUDE_PLUGIN_ROOT%\\bin\\launch.cmd\" PreCompact" }
    ]
  }
}
```

The dual entries are intentional: on Unix, the `cmd /c ...` entry fails silently with a non-zero exit and Claude tolerates it. On Windows, the `bash` entry fails silently when bash isn't available (or runs the Unix launcher which fails its OS check). Document this in the README.

- [ ] **Step 4: Make launcher executable on Unix**

```bash
chmod +x bin/launch.sh
```

- [ ] **Step 5: Smoke check**

Unix:
```bash
mkdir -p bin/$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/amd64/x86_64/;s/arm64/aarch64/')
cp target/debug/roaring-crab bin/$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/amd64/x86_64/;s/arm64/aarch64/')/
cp target/debug/roaring-crabd bin/$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m | sed 's/amd64/x86_64/;s/arm64/aarch64/')/
CLAUDE_PLUGIN_ROOT="$PWD" RC_NULL_AUDIO=1 bash bin/launch.sh Stop
echo "exit: $?"
```
Expected: exit 0, no crash.

- [ ] **Step 6: Commit**

```bash
git add hooks.json bin/launch.sh bin/launch.cmd
git commit -m "Add plugin hooks manifest and platform-detecting launchers"
```

---

## Task 21: Audition example

**Files:**
- Create: `examples/audition.rs`

- [ ] **Step 1: Write the example**

`examples/audition.rs`:
```rust
//! Plays every patch with three seeds for sound design iteration.
//!
//! Run: `cargo run --release --example audition`

use roaring_crab::audio_sink::real::CpalSink;
use roaring_crab::hook_event::HookEvent;
use roaring_crab::mixer::Mixer;
use roaring_crab::patches;
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let mixer = Arc::new(Mixer::new(48000));
    mixer.set_master_volume(0.7);
    let mixer_for_cb = mixer.clone();
    let _sink = CpalSink::open(Box::new(move |buf| mixer_for_cb.render(buf)))
        .expect("open audio output");

    for event in HookEvent::ALL {
        println!("---- {:?} ----", event);
        for seed in [1u64, 42, 9001] {
            println!("  seed {}", seed);
            mixer.push(patches::build(event, seed, mixer.sample_rate()));
            std::thread::sleep(Duration::from_millis(1100));
        }
    }
    std::thread::sleep(Duration::from_secs(1));
}
```

- [ ] **Step 2: Build to verify**

Run: `cargo build --release --example audition`
Expected: Success.

- [ ] **Step 3: Manual play (optional, requires audio device)**

Run: `cargo run --release --example audition`
Expected: hear all 27 sounds.

- [ ] **Step 4: Commit**

```bash
git add examples/audition.rs
git commit -m "Add audition example for sound design iteration"
```

---

## Task 22: CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write CI workflow**

`.github/workflows/ci.yml`:
```yaml
name: CI

on:
  push:
    branches: [main, master]
  pull_request:

jobs:
  test:
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Install ALSA dev headers (Linux only)
        if: matrix.os == 'ubuntu-latest'
        run: sudo apt-get update && sudo apt-get install -y libasound2-dev
      - name: Build
        run: cargo build --features null-audio --bins --tests --examples
      - name: Unit + integration tests
        run: cargo test --features null-audio
      - name: cargo fmt
        run: cargo fmt --check
        continue-on-error: false
      - name: cargo clippy
        run: cargo clippy --features null-audio -- -D warnings
        continue-on-error: false
```

- [ ] **Step 2: Verify yaml parses**

Run: `cargo build --features null-audio` (local sanity)
Expected: Success. (Actual CI run happens on push.)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "Add CI matrix: build + test + fmt + clippy on Linux/macOS/Windows"
```

---

## Task 23: Release workflow

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write release workflow**

`.github/workflows/release.yml`:
```yaml
name: Release

on:
  push:
    tags: ['v*']
  workflow_dispatch:

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            dir: linux-x86_64
            ext: ""
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            dir: linux-aarch64
            ext: ""
            cross: true
          - target: x86_64-apple-darwin
            os: macos-latest
            dir: macos-x86_64
            ext: ""
          - target: aarch64-apple-darwin
            os: macos-latest
            dir: macos-aarch64
            ext: ""
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            dir: windows-x86_64
            ext: ".exe"
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Install ALSA dev headers (Linux x86_64)
        if: matrix.target == 'x86_64-unknown-linux-gnu'
        run: sudo apt-get update && sudo apt-get install -y libasound2-dev
      - name: Install cross + ALSA (Linux aarch64)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          cargo install cross --git https://github.com/cross-rs/cross
          sudo apt-get update && sudo apt-get install -y libasound2-dev
      - name: Build (cross)
        if: matrix.cross
        run: cross build --release --target ${{ matrix.target }} --bins
      - name: Build (native)
        if: '!matrix.cross'
        run: cargo build --release --target ${{ matrix.target }} --bins
      - name: Ad-hoc sign (macOS only)
        if: startsWith(matrix.target, 'aarch64-apple') || startsWith(matrix.target, 'x86_64-apple')
        run: |
          codesign --force --sign - target/${{ matrix.target }}/release/roaring-crab
          codesign --force --sign - target/${{ matrix.target }}/release/roaring-crabd
      - name: Stage binaries
        shell: bash
        run: |
          mkdir -p artifacts/bin/${{ matrix.dir }}
          cp target/${{ matrix.target }}/release/roaring-crab${{ matrix.ext }} artifacts/bin/${{ matrix.dir }}/
          cp target/${{ matrix.target }}/release/roaring-crabd${{ matrix.ext }} artifacts/bin/${{ matrix.dir }}/
      - uses: actions/upload-artifact@v4
        with:
          name: bin-${{ matrix.dir }}
          path: artifacts/bin/${{ matrix.dir }}

  publish:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Move binaries into bin/<platform>/
        run: |
          for dir in artifacts/bin-*; do
            name="${dir#artifacts/bin-}"
            mkdir -p "bin/$name"
            cp -r "$dir"/* "bin/$name/"
            chmod +x "bin/$name"/* || true
          done
      - name: Commit binaries (release tag only)
        if: startsWith(github.ref, 'refs/tags/v')
        run: |
          git config user.name "github-actions"
          git config user.email "github-actions@github.com"
          git add bin/
          git commit -m "Release binaries for ${GITHUB_REF_NAME}" || echo "no changes"
          git push origin HEAD:main
      - name: Create release
        if: startsWith(github.ref, 'refs/tags/v')
        uses: softprops/action-gh-release@v2
        with:
          files: artifacts/**/*
          generate_release_notes: true
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "Add release workflow: build + ad-hoc-sign + publish prebuilt binaries"
```

---

## Task 24: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write README**

`README.md`:
````markdown
# roaring-crab

A Claude Code plugin that plays generated analog-modeling synth sounds on hook events. Cross-platform (Linux, macOS, Windows). Sounds are abstract and musical — themed signature patches per hook with seeded variation.

## What it does

Every hook event in Claude Code (PreToolUse, Stop, Notification, etc.) triggers a short generated sound. Sounds are not files — they're synthesized in real time from oscillators, filters, and envelopes. Each hook has its own signature patch; each invocation seeds slight variation so it never sounds identical twice.

## Install

This plugin ships prebuilt binaries under `bin/<platform>/`. No Rust toolchain required to use it.

1. Clone or add this repo to Claude Code as a plugin (per Claude Code's plugin install docs).
2. That's it. The first hook event will lazy-spawn the daemon; subsequent events reuse it.

### Platform notes

- **macOS**: binaries are ad-hoc signed in CI. On first run you may need to right-click → Open in Finder once to clear Gatekeeper, or run `xattr -dr com.apple.quarantine bin/macos-*/roaring-crab*`.
- **Linux**: requires an ALSA-compatible audio stack (default on most distros; PipeWire and PulseAudio work via ALSA emulation).
- **Windows**: no setup beyond installing the plugin.

## Config

User config lives at the platform-appropriate location:

- Linux: `~/.config/roaring-crab/config.toml`
- macOS: `~/Library/Application Support/roaring-crab/config.toml`
- Windows: `%APPDATA%\roaring-crab\config.toml`

Defaults are written on first run. Example:

```toml
master_volume = 0.7   # 0.0 – 1.0
muted = false

[enabled_hooks]
SessionStart      = true
SessionEnd        = true
UserPromptSubmit  = true
PreToolUse        = true
PostToolUse       = true
Notification      = true
Stop              = true
SubagentStop      = true
PreCompact        = true
```

Set `muted = true` to silence everything. Disable individual hooks under `[enabled_hooks]` to keep, say, Stop alerts but mute the ambient PreToolUse ticks.

## Verify install

Run the launcher directly:

```bash
# Unix
CLAUDE_PLUGIN_ROOT="$PWD" bash bin/launch.sh Stop

# Windows PowerShell
$env:CLAUDE_PLUGIN_ROOT = (Get-Location).Path
& bin\launch.cmd Stop
```

You should hear the Stop chord within a second or two (longer on the very first invocation as the daemon spawns).

## How it works

A long-lived daemon (`roaring-crabd`) owns the audio output device and a voice mixer. Each hook fires a short client (`roaring-crab --event <name>`) that sends a single `PlayEvent` message to the daemon over a local socket and exits in ~10–15ms. The daemon synthesizes the patch in real time with `fundsp` and mixes it into its `cpal` output stream. Overlapping events layer naturally. The daemon idle-exits after 5 minutes of no events.

## Build from source (developers only)

```bash
cargo build --release
cargo test --features null-audio
cargo run --release --example audition  # plays every patch
```

See `docs/superpowers/specs/2026-05-15-roaring-crab-design.md` for the design.

## License

MIT OR Apache-2.0
````

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "Add README with install, config, and verify instructions"
```

---

## Self-Review

I reviewed this plan against the spec. Coverage check:

| Spec section | Where it's implemented |
|---|---|
| Goals: all 9 hooks supported | Task 2 (HookEvent enum), Task 20 (hooks.json) |
| Goals: per-hook on/off + master volume | Task 4 (Config) |
| Goals: analog synth, themed families, variation | Tasks 10–13 (patches) |
| Goals: ~10ms hook return | Task 16 (client design), Task 18 (lazy_spawn timing) |
| Goals: overlapping events layer | Task 9 (mixer voice list) |
| Goals: no build toolchain for users | Task 23 (release workflow ships prebuilt binaries) |
| Architecture: two processes, shared crate | Tasks 1, 15, 16 |
| Architecture: lazy spawn + idle exit | Task 14 (spawn), Task 15 (daemon idle loop), Task 18 (test) |
| Components: protocol | Task 3 |
| Components: config | Task 4 |
| Components: hook_event | Task 2 |
| Components: patches (3 families) | Tasks 10, 11, 12, 13 |
| Components: mixer | Task 9 |
| Components: socket path | Task 5 |
| Components: lockfile | Task 6 |
| Components: logging w/ rate-limit + rolling | Task 7 |
| Components: spawn (detached, no console flash on Windows) | Task 14 |
| Components: audio sink with null-audio | Task 8 |
| Data flow | Tasks 15 + 16 wire it together; Task 17 tests it |
| Error handling: client never exits nonzero on runtime audio fail | Task 16 |
| Error handling: daemon log file, rate-limited | Task 7 + Task 15 |
| Error handling: panic-safe voices | Task 9 (clamp) + Task 15 (catch_unwind — not yet added, see fix below) |
| Cross-platform gotchas | Task 14 (Windows flags), Task 24 (README macOS notes), Task 22 (Linux ALSA) |
| Testing: unit (5 modules) | Tasks 2, 3, 4, 6, 7, 9 |
| Testing: patches contract | Tasks 11, 12, 13 |
| Testing: client_to_daemon | Task 17 |
| Testing: lazy_spawn | Task 18 |
| Testing: concurrent_events | Task 19 |
| Audition example | Task 21 |
| README verify-install | Task 24 |
| CI matrix | Task 22 |
| Release workflow (binaries + ad-hoc sign) | Task 23 |

**Gap fixed inline:** the spec calls for `panic::catch_unwind` per voice in the audio callback. The mixer in Task 9 doesn't yet wrap voice pumps in `catch_unwind`. Add this note to Task 9 Step 3 implementation — wrap the `v.pump()` call:

```rust
for v in voices.iter_mut() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.pump()));
    if let Ok(Some((vl, vr))) = result {
        l += vl;
        r += vr;
    } else if result.is_err() {
        v.samples_remaining = 0; // mark for removal
    }
}
```

**Placeholder scan:** Searched for "TBD", "TODO", "implement later", "appropriate", "similar to". None present.

**Type consistency:** `HookEvent::ALL`, `Voice::from_fn`, `Mixer::push`, `Mixer::render`, `Mixer::voice_count`, `Mixer::set_master_volume`, `MAX_VOICES`, `write_frame`, `read_frame`, `PlayEvent`, `Config::load_or_default`, `Config::is_enabled`, `Config::default_path`, `Lock::try_acquire`, `LockResult::{Acquired, Busy}`, `RateLimiter::{new, allow}`, `RollingLog::{open, write_line}` — all consistent across tasks.

---

Plan complete and saved to `docs/superpowers/plans/2026-05-15-roaring-crab.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
