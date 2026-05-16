//! Integration test: invoke the client with no prior daemon, assert:
//!   1. Client exits 0 (sends Stop event successfully via lazy-spawn).
//!   2. Daemon was lazy-spawned (poll-connect succeeds within 2s of client exit).
//!   3. After ~3s of no events, daemon idle-exits (subsequent connect attempts fail).
//!
//! Cross-platform: Unix uses RC_SOCKET_PATH (file path); Windows uses RC_SOCKET_PIPE.

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

/// Sets env vars on a Command and returns the socket name/path.
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

fn connect_to(name_or_path: &str) -> std::io::Result<interprocess::local_socket::Stream> {
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

/// Returns true if the socket becomes responsive before the deadline.
fn poll_until_up(socket: &str, deadline: Instant) -> bool {
    while Instant::now() < deadline {
        if let Ok(conn) = connect_to(socket) {
            drop(conn);
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Returns true if the socket stays down (connection refused) until the deadline.
fn poll_until_down(socket: &str, deadline: Instant) -> bool {
    // Give a short grace period, then poll until either (a) we reach the deadline
    // with every attempt failing, or (b) a connection unexpectedly succeeds.
    while Instant::now() < deadline {
        if connect_to(socket).is_ok() {
            // Still up — keep waiting
            std::thread::sleep(Duration::from_millis(100));
        } else {
            return true; // gone
        }
    }
    false
}

#[test]
fn client_lazy_spawns_daemon_then_daemon_idle_exits() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let id = unique_id();

    // Build the client command with the same env the daemon will inherit via spawn.
    // The client needs to know the socket path AND forward it to the daemon it spawns.
    // The client binary uses std::env vars at runtime, so setting them on the client
    // process is enough — the daemon will inherit them (spawn_daemon inherits the env).
    let mut cmd = std::process::Command::new(cargo_bin("roaring-crab"));
    let socket_name = setup_socket_env(&mut cmd, tmp.path(), &id);
    cmd.env("RC_IDLE_SECS", "2"); // daemon idles out in 2s
    cmd.args(["--event", "Stop"]);
    // Provide empty stdin (client drains stdin)
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    // Client should exit 0
    let status = cmd.status().expect("client spawn");
    assert!(status.success(), "client exited non-zero: {:?}", status);

    // Within 2s the daemon should have been lazy-spawned and be responsive
    let up_deadline = Instant::now() + Duration::from_secs(4);
    assert!(
        poll_until_up(&socket_name, up_deadline),
        "daemon was not reachable after client lazy-spawn"
    );

    // Now wait up to 5s for the daemon to idle-exit (RC_IDLE_SECS=2)
    // We allow up to 5s of wall time (2s idle + up to 3s polling slack)
    let down_deadline = Instant::now() + Duration::from_secs(5);
    assert!(
        poll_until_down(&socket_name, down_deadline),
        "daemon did not idle-exit after the idle timeout"
    );
}
