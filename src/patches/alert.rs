use crate::mixer::Voice;
use crate::patches::{adsr, cents, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 750;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Rising 3-note arpeggio (root → M3 → P5) with each note slightly chorused
/// (detuned twin) for thickness and a soft per-note attack.
pub fn notification(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-5.0..5.0);
    let chorus_cents: f32 = rng.gen_range(5.0..10.0);
    let root = 440.0 * cents(detune);
    let intervals = [1.0f32, 1.259_921, 1.4983071]; // root, M3, P5
    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let idx = (progress * 3.0).floor().clamp(0.0, 2.0) as usize;
        let hz = root * intervals[idx];
        let env = adsr(t, total, 0.05, 0.15, 0.6, 0.45);
        let note_phase = (progress * 3.0).fract();
        // Per-note micro-attack: drop amplitude at note boundaries.
        let note_env = (note_phase * 8.0).min(1.0);
        // Chorus: a second oscillator detuned a few cents up.
        let a = phase(t, sample_rate, hz).sin();
        let b = phase(t, sample_rate, hz * cents(chorus_cents)).sin() * 0.85;
        let h = 0.28 * phase(t, sample_rate, hz * 2.0).sin();
        let sample = 0.16 * env * note_env * (a + b + h);
        // Slight stereo phase offset on the chorus for width.
        let stereo_offset = 0.04 * phase(t, sample_rate, hz * cents(-chorus_cents)).sin();
        (sample - stereo_offset, sample + stereo_offset)
    })
}

/// Resolved chord — root + M3 + P5 + octave, with a soft attack, slow tremolo,
/// and a high-frequency shimmer that rides on top of the longer release.
pub fn stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-4.0..4.0);
    let trem_hz: f32 = rng.gen_range(4.5..7.0);
    let shimmer_amount: f32 = rng.gen_range(0.05..0.12);
    let root = 330.0 * cents(detune);
    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.08, 0.20, 0.7, 0.55);
        let trem = 0.92 + 0.08 * phase(t, sample_rate, trem_hz).sin();
        let r = phase(t, sample_rate, root).sin();
        let m3 = 0.7 * phase(t, sample_rate, root * 1.2599).sin();
        let p5 = 0.85 * phase(t, sample_rate, root * 1.5).sin();
        let oct = 0.6 * phase(t, sample_rate, root * 2.0).sin();
        let chord = (r + m3 + p5 + oct) / 3.15;
        // Shimmer: an octave-up sine that pulses slowly.
        let shimmer = shimmer_amount * phase(t, sample_rate, root * 4.0).sin();
        let sample = 0.18 * env * trem * (chord + shimmer);
        // Stereo widen: tiny pitch offset between channels.
        let widen = 0.02 * phase(t, sample_rate, root * cents(3.0)).sin();
        (sample - widen, sample + widen)
    })
}

/// A quiet two-note resolution, M3 → root descending, with a long ringing tail
/// to lean on the master reverb. Subtler than Stop — for nested agent exits.
pub fn subagent_stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-5.0..5.0);
    let chorus_cents: f32 = rng.gen_range(4.0..8.0);
    let root = 392.0 * cents(detune);
    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let hz = if progress < 0.45 {
            root * 1.259_921
        } else {
            root
        };
        let env = adsr(t, total, 0.05, 0.22, 0.55, 0.55);
        let s = phase(t, sample_rate, hz).sin();
        let twin = 0.7 * phase(t, sample_rate, hz * cents(chorus_cents)).sin();
        let h = 0.2 * phase(t, sample_rate, hz * 2.0).sin();
        let sample = 0.12 * env * (s + twin + h);
        let widen = 0.03 * phase(t, sample_rate, hz * cents(-chorus_cents)).sin();
        (sample - widen, sample + widen)
    })
}
