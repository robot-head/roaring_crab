//! Integration test: spawn daemon, connect directly, send a PlayEvent, kill daemon.
//!
//! Cross-platform: Unix uses RC_SOCKET_PATH (file path); Windows uses RC_SOCKET_PIPE (pipe name).

use roaring_crab::hook_event::HookEvent;
use roaring_crab::protocol::{write_frame, PlayEvent};
use std::time::{Duration, Instant};

fn cargo_bin(name: &str) -> std::path::PathBuf {
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

fn unique_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let micros = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros();
    format!("rc-test-{}", micros)
}

/// Sets env vars on a Command and returns the socket name/path for the test's own connect.
fn setup_socket_env(
    cmd: &mut std::process::Command,
    tmp_dir: &std::path::Path,
    id: &str,
) -> String {
    let lockfile = tmp_dir.join(format!("{}.lock", id));
    cmd.env("RC_LOCKFILE", &lockfile);
    cmd.env("RC_NULL_AUDIO", "1");
    if cfg!(windows) {
        let pipe = format!("roaring-crab-{}", id);
        cmd.env("RC_SOCKET_PIPE", &pipe);
        pipe
    } else {
        let sock = tmp_dir.join(format!("{}.sock", id));
        cmd.env("RC_SOCKET_PATH", sock.to_str().unwrap());
        sock.to_string_lossy().into_owned()
    }
}

/// Connect to the socket using the interprocess API cross-platform.
fn connect_to(
    name_or_path: &str,
) -> std::io::Result<interprocess::local_socket::Stream> {
    use interprocess::local_socket::traits::Stream as StreamTrait;
    use interprocess::local_socket::Stream;
    if cfg!(windows) {
        use interprocess::local_socket::{GenericNamespaced, ToNsName};
        let name = name_or_path
            .to_ns_name::<GenericNamespaced>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
            .into_owned();
        Stream::connect(name)
    } else {
        use interprocess::local_socket::{GenericFilePath, ToFsName};
        let name = name_or_path
            .to_fs_name::<GenericFilePath>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?
            .into_owned();
        Stream::connect(name)
    }
}

/// Poll-connect until the daemon is responsive or the deadline passes.
fn wait_for_daemon(socket: &str, deadline: Instant) -> bool {
    while Instant::now() < deadline {
        if let Ok(conn) = connect_to(socket) {
            drop(conn);
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn daemon_accepts_play_event_over_socket() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let id = unique_id();

    let mut cmd = std::process::Command::new(cargo_bin("roaring-crabd"));
    let socket_name = setup_socket_env(&mut cmd, tmp.path(), &id);
    cmd.env("RC_IDLE_SECS", "60");

    let mut daemon = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn daemon");

    // Wait up to 4s for the daemon to be ready
    let deadline = Instant::now() + Duration::from_secs(4);
    assert!(
        wait_for_daemon(&socket_name, deadline),
        "daemon did not become responsive in time"
    );

    // Send a PlayEvent directly
    let play = PlayEvent {
        event: HookEvent::Stop,
        seed: 42,
        volume: 0.5,
    };

    let mut stream = connect_to(&socket_name).expect("connect for send");
    write_frame(&mut stream, &play).expect("write_frame");
    drop(stream);

    // Brief pause then kill daemon
    std::thread::sleep(Duration::from_millis(100));
    let _ = daemon.kill();
    let status = daemon.wait().expect("wait daemon");

    // Daemon was killed — that's expected; the key assertion is that nothing panicked.
    // On Windows kill() always produces a non-zero exit; on Unix it may be signal-terminated.
    // We just verify the process is gone (wait succeeded).
    let _ = status; // don't assert exit code — killed process exit code varies
}
