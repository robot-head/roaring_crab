use crate::mixer::Voice;
use crate::patches::{adsr, cents, lp_alpha, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 1100;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Wide detuned-stack pad. Three sine oscillators per channel, each cents-detuned
/// in opposite directions for left/right, plus a slow LFO-driven lowpass for a
/// breathing motion. `direction = +1` swells in, `-1` decays out.
fn pad(seed: u64, sample_rate: u32, root_hz: f32, direction: i32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let coarse_detune: f32 = rng.gen_range(-4.0..4.0);
    let spread: f32 = rng.gen_range(6.0..14.0); // cents spread between voices
    let trem_hz: f32 = rng.gen_range(2.3..4.7);
    let filter_lfo_hz: f32 = rng.gen_range(0.18..0.35);
    let cutoff_low: f32 = rng.gen_range(700.0..1100.0);
    let cutoff_high: f32 = rng.gen_range(2200.0..3400.0);

    let root = root_hz * cents(coarse_detune);
    // Six voices: three per channel, each at root, ±spread cents.
    let f_l = [root, root * cents(spread), root * cents(-spread * 0.7)];
    let f_r = [
        root * cents(spread * 0.8),
        root * cents(-spread),
        root * cents(spread * 0.4),
    ];

    let mut lp_l = 0.0f32;
    let mut lp_r = 0.0f32;

    Voice::from_fn(total, move |t| {
        let mut env = adsr(t, total, 0.30, 0.18, 0.70, 0.40);
        if direction < 0 {
            // Layer an additional descending taper so SessionEnd genuinely fades.
            env *= 1.0 - 0.6 * (t as f32 / total as f32);
        }
        let trem = 0.84 + 0.16 * phase(t, sample_rate, trem_hz).sin();

        // Sum stack per channel, including a sub-octave on the bottom for body.
        let sub = 0.35 * phase(t, sample_rate, root * 0.5).sin();
        let mut l = sub;
        let mut r = sub;
        for &f in &f_l {
            l += phase(t, sample_rate, f).sin();
        }
        for &f in &f_r {
            r += phase(t, sample_rate, f).sin();
        }
        // Add a soft 2nd-harmonic spice on both sides for a "shimmer" suggestion.
        let shimmer = 0.18 * phase(t, sample_rate, root * 4.0).sin();
        l += shimmer;
        r += shimmer * 0.9;

        // Normalize the stack and apply envelope.
        let mix_l = 0.13 * env * trem * l / 4.0;
        let mix_r = 0.13 * env * trem * r / 4.0;

        // Filter LFO sweeps cutoff between low and high.
        let lfo = 0.5 + 0.5 * phase(t, sample_rate, filter_lfo_hz).sin();
        let cutoff = cutoff_low + (cutoff_high - cutoff_low) * lfo;
        let alpha = lp_alpha(sample_rate, cutoff);
        lp_l += alpha * (mix_l - lp_l);
        lp_r += alpha * (mix_r - lp_r);
        (lp_l, lp_r)
    })
}

pub fn session_start(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 220.0, 1)
}

pub fn session_end(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 165.0, -1)
}

/// PreCompact: a two-note rising motif (root × 3 → root × 4) over a sub-octave
/// pad, with a slight pitch wobble and a darker filter than the session pad.
pub fn pre_compact(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let coarse_detune: f32 = rng.gen_range(-6.0..6.0);
    let wobble_hz: f32 = rng.gen_range(4.5..7.0);
    let wobble_depth: f32 = rng.gen_range(2.5..5.5); // cents
    let root = 196.0 * cents(coarse_detune);

    let mut lp = 0.0f32;
    let alpha = lp_alpha(sample_rate, 2200.0);

    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.22, 0.20, 0.65, 0.42);
        let progress = t as f32 / total as f32;
        let wob = cents(wobble_depth * phase(t, sample_rate, wobble_hz).sin());
        let step_hz = if progress < 0.5 {
            root * 3.0 * wob
        } else {
            root * 4.0 * wob
        };
        let pad_s =
            phase(t, sample_rate, root).sin() + 0.5 * phase(t, sample_rate, root * 2.0).sin();
        let sub = 0.4 * phase(t, sample_rate, root * 0.5).sin();
        let step = 0.35 * phase(t, sample_rate, step_hz).sin();
        let raw = 0.14 * env * (pad_s / 1.5 + sub + step);
        lp += alpha * (raw - lp);
        (lp * 0.96, lp * 1.04)
    })
}
