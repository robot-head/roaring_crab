use crate::mixer::Voice;
use crate::patches::{adsr, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 800;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Warm pad: stacked sines an octave apart with slow tremolo. `direction = +1`
/// ramps up, `-1` ramps down.
fn pad(seed: u64, sample_rate: u32, root_hz: f32, direction: i32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-3.0..3.0);
    let trem_hz: f32 = rng.gen_range(3.0..6.0);
    let root = root_hz * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let mut env = adsr(t, total, 0.25, 0.20, 0.7, 0.35);
        if direction < 0 {
            env *= 1.0 - (t as f32 / total as f32);
        }
        let trem = 0.85 + 0.15 * phase(t, sample_rate, trem_hz).sin();
        let p1 = phase(t, sample_rate, root).sin();
        let p2 = 0.5 * phase(t, sample_rate, root * 2.0).sin();
        let p3 = 0.25 * phase(t, sample_rate, root * 3.0).sin();
        let sample = 0.18 * env * trem * (p1 + p2 + p3) / 1.75;
        (sample * 0.95, sample * 1.05)
    })
}

pub fn session_start(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 220.0, 1)
}

pub fn session_end(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 165.0, -1)
}

pub fn pre_compact(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-5.0..5.0);
    let root = 196.0 * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.15, 0.20, 0.6, 0.40);
        let progress = t as f32 / total as f32;
        let step_hz = if progress < 0.5 { root * 3.0 } else { root * 4.0 };
        let pad_s =
            phase(t, sample_rate, root).sin() + 0.5 * phase(t, sample_rate, root * 2.0).sin();
        let step = 0.3 * phase(t, sample_rate, step_hz).sin();
        let sample = 0.16 * env * (pad_s / 1.5 + step);
        (sample, sample)
    })
}
