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
    std::fs::write(&path, "master_volume = 0.5\nfuture_field = \"hello\"\n").unwrap();
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
