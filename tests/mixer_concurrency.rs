use roaring_crab::mixer::{Mixer, Voice, MAX_VOICES};

fn silent_voice(samples: u32) -> Voice {
    Voice::from_fn(samples, |_t| (0.0, 0.0))
}

fn constant_voice(samples: u32, value: f32) -> Voice {
    Voice::from_fn(samples, move |_t| (value, value))
}

#[test]
fn empty_mixer_produces_silence() {
    let m = Mixer::new(48000);
    let mut buf = vec![1.0f32; 100];
    m.render(&mut buf);
    assert!(buf.iter().all(|s| *s == 0.0));
}

#[test]
fn finished_voices_are_removed() {
    let m = Mixer::new(48000);
    m.push(silent_voice(2));
    let mut buf = vec![0.0; 2 * 2]; // 2 frames stereo
    m.render(&mut buf);
    m.render(&mut buf);
    assert_eq!(m.voice_count(), 0);
}

#[test]
fn voice_cap_is_enforced() {
    let m = Mixer::new(48000);
    for _ in 0..(MAX_VOICES + 5) {
        m.push(silent_voice(48000));
    }
    assert_eq!(m.voice_count(), MAX_VOICES);
}

#[test]
fn master_volume_scales_output() {
    let m = Mixer::new(48000);
    m.set_master_volume(0.5);
    m.push(constant_voice(10, 1.0));
    let mut buf = vec![0.0; 2 * 2];
    m.render(&mut buf);
    for s in &buf {
        assert!((s - 0.5).abs() < 1e-6, "got {}", s);
    }
}

#[test]
fn output_is_clamped_to_unit_range() {
    let m = Mixer::new(48000);
    for _ in 0..5 {
        m.push(constant_voice(10, 1.0));
    }
    let mut buf = vec![0.0; 2 * 2];
    m.render(&mut buf);
    for s in &buf {
        assert!(*s <= 1.0 && *s >= -1.0, "unclamped: {}", s);
    }
}
