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
