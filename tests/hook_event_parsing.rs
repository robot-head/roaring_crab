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
