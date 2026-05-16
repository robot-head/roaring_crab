use crate::mixer::Voice;
use crate::patches::{adsr, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 600;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Rising 3-note arpeggio in a major-pentatonic-flavored shape.
pub fn notification(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-4.0..4.0);
    let root = 440.0 * 2.0f32.powf(detune / 1200.0);
    let intervals = [1.0f32, 1.2599210, 1.4983071]; // root, M3, P5
    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let idx = (progress * 3.0).floor().clamp(0.0, 2.0) as usize;
        let hz = root * intervals[idx];
        let env = adsr(t, total, 0.05, 0.15, 0.6, 0.45);
        let note_phase = (progress * 3.0).fract();
        let note_env = 1.0 - note_phase.max(0.0) * 0.3;
        let s = phase(t, sample_rate, hz).sin();
        let h = 0.3 * phase(t, sample_rate, hz * 2.0).sin();
        let sample = 0.20 * env * note_env * (s + h);
        (sample, sample)
    })
}

/// Resolved chord — root + 5th + octave with a soft attack and longer release.
pub fn stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-3.0..3.0);
    let root = 330.0 * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.08, 0.20, 0.65, 0.55);
        let a = phase(t, sample_rate, root).sin();
        let b = 0.85 * phase(t, sample_rate, root * 1.5).sin();
        let c = 0.6 * phase(t, sample_rate, root * 2.0).sin();
        let sample = 0.16 * env * (a + b + c) / 2.45;
        (sample * 0.97, sample * 1.03)
    })
}

/// A quieter two-note resolution, M3 → root descending.
pub fn subagent_stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-4.0..4.0);
    let root = 392.0 * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let hz = if progress < 0.45 { root * 1.2599210 } else { root };
        let env = adsr(t, total, 0.04, 0.20, 0.55, 0.45);
        let s = phase(t, sample_rate, hz).sin();
        let h = 0.2 * phase(t, sample_rate, hz * 2.0).sin();
        let sample = 0.14 * env * (s + h);
        (sample, sample)
    })
}
