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
    let cur_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    assert!(cur_size <= cap, "current log exceeded cap: {}", cur_size);
    let old = path.with_extension("log.old");
    assert!(old.exists(), "expected rolled-over .log.old to exist");
}
