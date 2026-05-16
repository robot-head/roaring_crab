use clap::Parser;
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
    use interprocess::local_socket::traits::Stream as StreamTrait;
    use interprocess::local_socket::Stream;
    let name = socket_path::socket_name()?;
    let mut stream = Stream::connect(name)?;
    write_frame(&mut stream, &play).map_err(std::io::Error::other)?;
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
        return;
    }

    // Only Notification carries a repeat_secs; for all other events the field
    // is None and the daemon's repeat state stays inactive (or gets cleared).
    let repeat_secs = if cli.event == HookEvent::Notification {
        cfg.notification_repeat_secs
    } else {
        None
    };
    let play = PlayEvent {
        event: cli.event,
        seed: rand::random(),
        volume: cfg.volume_for(cli.event),
        repeat_secs,
    };

    if try_connect_and_send(play).is_ok() {
        return;
    }

    // Daemon probably not running — spawn and retry.
    let client_path =
        std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("roaring-crab"));
    let daemon_path = spawn::daemon_sibling_path(&client_path);
    if let Err(e) = spawn::spawn_daemon(&daemon_path) {
        eprintln!("roaring-crab: daemon spawn failed: {}", e);
        return;
    }

    // WASAPI cold-start on Windows can take several hundred ms before the
    // daemon binds its socket. Give it up to 1.5s on first spawn.
    let deadline = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < deadline {
        if try_connect_and_send(play).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    eprintln!("roaring-crab: daemon slow to start, skipping event");
}
