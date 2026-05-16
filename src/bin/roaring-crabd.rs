use interprocess::local_socket::{traits::ListenerExt, ListenerOptions};
use parking_lot::Mutex;
use roaring_crab::audio_sink::{AudioCallback, AudioSink};
use roaring_crab::config::Config;
use roaring_crab::lockfile::{Lock, LockResult};
use roaring_crab::logging::RollingLog;
use roaring_crab::mixer::Mixer;
use roaring_crab::patches;
use roaring_crab::protocol::read_frame;
use roaring_crab::socket_path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn log_dir() -> std::path::PathBuf {
    Config::default_path()
        .and_then(|p| p.parent().map(|x| x.to_path_buf()))
        .unwrap_or_else(std::env::temp_dir)
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
    if let Ok(p) = std::env::var("RC_LOCKFILE") {
        return std::path::PathBuf::from(p);
    }
    socket_path::socket_fs_path()
        .map(|p| p.with_extension("lock"))
        .unwrap_or_else(|| std::env::temp_dir().join("roaring-crab.lock"))
}

fn now_micros() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

fn main() {
    let log_path = log_dir().join("roaring-crabd.log");
    let log = match RollingLog::open(&log_path, 1_000_000) {
        Ok(l) => Arc::new(Mutex::new(l)),
        Err(_) => return,
    };
    let _ = log.lock().write_line("daemon starting");

    // Acquire lockfile
    let _lock_guard = match Lock::try_acquire(&lockfile_path()) {
        Ok(LockResult::Acquired(g)) => g,
        Ok(LockResult::Busy) => {
            let _ = log.lock().write_line("another daemon already running, exiting");
            return;
        }
        Err(e) => {
            let _ = log.lock().write_line(&format!("lockfile error: {}", e));
            return;
        }
    };

    let mixer = Arc::new(Mixer::new(48000));
    let last_event = Arc::new(AtomicI64::new(now_micros()));

    // Open audio sink
    let mixer_for_cb = mixer.clone();
    let callback: AudioCallback = Box::new(move |buf: &mut [f32]| mixer_for_cb.render(buf));

    let _sink: Arc<dyn AudioSink> = if use_null_audio() {
        #[cfg(feature = "null-audio")]
        {
            roaring_crab::audio_sink::null::NullSink::open(callback, 48000)
        }
        #[cfg(not(feature = "null-audio"))]
        {
            // RC_NULL_AUDIO requested but feature not compiled in.
            // Fall back to opening real cpal anyway (this is best-effort).
            match roaring_crab::audio_sink::real::CpalSink::open(callback) {
                Ok(s) => s,
                Err(e) => {
                    let _ = log.lock().write_line(&format!("cpal open failed: {}", e));
                    return;
                }
            }
        }
    } else {
        #[cfg(not(feature = "null-audio"))]
        {
            match roaring_crab::audio_sink::real::CpalSink::open(callback) {
                Ok(s) => s,
                Err(e) => {
                    let _ = log.lock().write_line(&format!("cpal open failed: {}", e));
                    return;
                }
            }
        }
        #[cfg(feature = "null-audio")]
        {
            roaring_crab::audio_sink::null::NullSink::open(callback, 48000)
        }
    };

    // Bind socket
    let name = match socket_path::socket_name() {
        Ok(n) => n,
        Err(e) => {
            let _ = log.lock().write_line(&format!("socket name: {}", e));
            return;
        }
    };
    let socket = match ListenerOptions::new().name(name).create_sync() {
        Ok(l) => l,
        Err(e) => {
            let _ = log.lock().write_line(&format!("socket bind: {}", e));
            return;
        }
    };
    let _ = log.lock().write_line("daemon ready, accepting connections");

    // Accept loop — move the listener into its own thread
    let mixer_for_accept = mixer.clone();
    let last_event_for_accept = last_event.clone();
    let log_for_accept = log.clone();
    std::thread::spawn(move || {
        for conn in socket.incoming() {
            match conn {
                Ok(mut stream) => match read_frame(&mut stream) {
                    Ok(play) => {
                        let voice = patches::build(
                            play.event,
                            play.seed,
                            mixer_for_accept.sample_rate(),
                        );
                        mixer_for_accept.set_master_volume(play.volume);
                        mixer_for_accept.push(voice);
                        last_event_for_accept.store(now_micros(), Ordering::Relaxed);
                    }
                    Err(e) => {
                        let _ = log_for_accept.lock().write_line(&format!("frame: {}", e));
                    }
                },
                Err(e) => {
                    let _ = log_for_accept.lock().write_line(&format!("accept: {}", e));
                }
            }
        }
    });

    // Idle watchdog (main thread)
    let idle = idle_timeout();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let last = last_event.load(Ordering::Relaxed);
        let elapsed = Duration::from_micros((now_micros() - last).max(0) as u64);
        if elapsed >= idle && mixer.voice_count() == 0 {
            let _ = log.lock().write_line("idle-exiting");
            std::process::exit(0);
        }
    }
}
