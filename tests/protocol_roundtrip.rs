use roaring_crab::hook_event::HookEvent;
use roaring_crab::protocol::{read_frame, write_frame, FrameError, PlayEvent, MAX_FRAME_SIZE};
use std::io::Cursor;

#[test]
fn play_event_roundtrips_through_frame() {
    let original = PlayEvent {
        event: HookEvent::Stop,
        seed: 0xDEADBEEFCAFEBABE,
        volume: 0.42,
        repeat_secs: None,
    };
    let mut buf = Vec::new();
    write_frame(&mut buf, &original).unwrap();
    let decoded = read_frame(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn trailing_garbage_after_frame_is_ignored_by_read_frame() {
    let event = PlayEvent {
        event: HookEvent::PreToolUse,
        seed: 1,
        volume: 0.5,
        repeat_secs: None,
    };
    let mut buf = Vec::new();
    write_frame(&mut buf, &event).unwrap();
    buf.extend_from_slice(b"trailing junk that should not affect the read");
    let decoded = read_frame(&mut Cursor::new(&buf)).unwrap();
    assert_eq!(decoded, event);
}

#[test]
fn oversized_frame_is_rejected() {
    let mut buf = Vec::new();
    buf.extend_from_slice(&((MAX_FRAME_SIZE + 1) as u32).to_be_bytes());
    let err = read_frame(&mut Cursor::new(&buf)).unwrap_err();
    assert!(matches!(err, FrameError::TooLarge(_)));
}

#[test]
fn all_hook_event_variants_serialize() {
    for event in HookEvent::ALL {
        let p = PlayEvent {
            event,
            seed: 7,
            volume: 0.5,
            repeat_secs: None,
        };
        let mut buf = Vec::new();
        write_frame(&mut buf, &p).unwrap();
        let decoded = read_frame(&mut Cursor::new(&buf)).unwrap();
        assert_eq!(decoded, p);
    }
}
