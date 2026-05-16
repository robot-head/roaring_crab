//! Integration test: spawn daemon, fire 20 concurrent client invocations,
//! assert all succeed and the daemon is still alive afterward.
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

const CONCURRENT_CLIENTS: usize = 20;

#[test]
fn twenty_concurrent_clients_all_succeed_daemon_stays_alive() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let id = unique_id();

    // Spawn the daemon
    let mut daemon_cmd = std::process::Command::new(cargo_bin("roaring-crabd"));
    let socket_name = setup_socket_env(&mut daemon_cmd, tmp.path(), &id);
    daemon_cmd.env("RC_IDLE_SECS", "60"); // keep alive for the whole test

    let mut daemon = daemon_cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn daemon");

    // Wait for daemon to be ready
    let deadline = Instant::now() + Duration::from_secs(4);
    assert!(
        wait_for_daemon(&socket_name, deadline),
        "daemon did not become responsive in time"
    );

    // Collect the per-client env: each client inherits the same socket + lockfile
    // (lockfile already held by daemon so clients won't spawn a second one).
    let tmp_path = tmp.path().to_path_buf();
    let id_clone = id.clone();
    let socket_name_clone = socket_name.clone();

    // Spawn 20 client threads, each running the client binary
    let handles: Vec<_> = (0..CONCURRENT_CLIENTS)
        .map(|i| {
            let tmp_path = tmp_path.clone();
            let id = id_clone.clone();
            let client_bin = cargo_bin("roaring-crab");
            let _socket = socket_name_clone.clone();
            std::thread::spawn(move || {
                let mut cmd = std::process::Command::new(&client_bin);
                // Use the same socket and lockfile as the running daemon
                let lockfile = tmp_path.join(format!("{}.lock", id));
                cmd.env("RC_LOCKFILE", &lockfile);
                cmd.env("RC_NULL_AUDIO", "1");
                if cfg!(windows) {
                    let pipe = format!("roaring-crab-{}", id);
                    cmd.env("RC_SOCKET_PIPE", &pipe);
                } else {
                    let sock = tmp_path.join(format!("{}.sock", id));
                    cmd.env("RC_SOCKET_PATH", sock.to_str().unwrap());
                }
                // Alternate between different events to exercise the dispatcher
                let events = [
                    "Stop",
                    "PreToolUse",
                    "PostToolUse",
                    "Notification",
                    "UserPromptSubmit",
                ];
                let event = events[i % events.len()];
                cmd.args(["--event", event]);
                cmd.stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                cmd.status()
            })
        })
        .collect();

    // Collect results — all clients must exit 0
    let mut failures = 0usize;
    for handle in handles {
        match handle.join() {
            Ok(Ok(status)) => {
                if !status.success() {
                    failures += 1;
                    eprintln!("client exited non-zero: {:?}", status);
                }
            }
            Ok(Err(e)) => {
                failures += 1;
                eprintln!("client spawn error: {}", e);
            }
            Err(_) => {
                failures += 1;
                eprintln!("thread panicked");
            }
        }
    }

    assert_eq!(
        failures, 0,
        "{} out of {} concurrent clients failed",
        failures, CONCURRENT_CLIENTS
    );

    // Daemon must still be alive
    let still_alive = daemon.try_wait().expect("try_wait").is_none();
    assert!(
        still_alive,
        "daemon exited prematurely while RC_IDLE_SECS=60"
    );

    // Verify one more connection can still be made
    assert!(
        connect_to(&socket_name).is_ok(),
        "daemon socket no longer responsive after concurrent load"
    );

    // Clean up
    let _ = daemon.kill();
    let _ = daemon.wait();
}
