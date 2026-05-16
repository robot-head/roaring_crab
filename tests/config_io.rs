use roaring_crab::config::{Config, ConfigError};
use roaring_crab::hook_event::HookEvent;
use tempfile::TempDir;

#[test]
fn attention_events_get_volume_boost() {
    let cfg = Config {
        master_volume: 0.5,
        attention_volume_boost: 1.6,
        ..Config::default()
    };
    assert!((cfg.volume_for(HookEvent::PreToolUse) - 0.5).abs() < 1e-6);
    assert!((cfg.volume_for(HookEvent::SessionStart) - 0.5).abs() < 1e-6);
    assert!((cfg.volume_for(HookEvent::Notification) - 0.8).abs() < 1e-6);
    assert!((cfg.volume_for(HookEvent::Stop) - 0.8).abs() < 1e-6);
}

#[test]
fn attention_boost_is_clamped_to_unit_range() {
    let cfg = Config {
        master_volume: 0.9,
        attention_volume_boost: 3.0,
        ..Config::default()
    };
    assert_eq!(cfg.volume_for(HookEvent::Notification), 1.0);
}

#[test]
fn notification_repeat_secs_round_trips_through_toml() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
master_volume = 0.5
muted = false
attention_volume_boost = 2.0
notification_repeat_secs = 45
"#,
    )
    .unwrap();
    let cfg = Config::load_or_default(&path).unwrap();
    assert_eq!(cfg.notification_repeat_secs, Some(45));
    assert!((cfg.attention_volume_boost - 2.0).abs() < 1e-6);
}

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
