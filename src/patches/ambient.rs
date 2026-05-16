use crate::mixer::Voice;
use crate::patches::{adsr, cents, lp_alpha, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 220;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Filtered FM blip: a carrier modulated by a sub-octave at varying index,
/// then run through a one-pole lowpass that sweeps from bright → dark over
/// the envelope. Each invocation perturbs the detune, FM index, and filter
/// trajectory so successive fires never sound identical.
fn ambient_blip(seed: u64, sample_rate: u32, center_hz: f32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-14.0..14.0);
    let fm_index: f32 = rng.gen_range(0.6..1.4);
    let cutoff_start: f32 = rng.gen_range(4200.0..6200.0);
    let cutoff_end: f32 = rng.gen_range(700.0..1400.0);
    let stereo_drift: f32 = rng.gen_range(0.06..0.16);

    let base = center_hz * cents(detune);
    let mod_hz = base * 0.5;

    let mut lp_state_l = 0.0f32;
    let mut lp_state_r = 0.0f32;

    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.08, 0.22, 0.55, 0.65);
        let progress = t as f32 / total as f32;

        // Sweep cutoff (and thus alpha) over the voice lifetime.
        let cutoff = cutoff_start + (cutoff_end - cutoff_start) * progress;
        let alpha = lp_alpha(sample_rate, cutoff);

        // FM: carrier sine modulated by a sub-octave sine.
        let mod_sig = phase(t, sample_rate, mod_hz).sin();
        let carrier_phase = phase(t, sample_rate, base) + fm_index * mod_sig;
        let car = carrier_phase.sin();

        // A small saw harmonic adds buzz that the filter then tames.
        let saw = (phase(t, sample_rate, base * 2.0).sin()) * 0.25;

        let raw = 0.22 * env * (car + saw);

        // One-pole LP per channel with slight stereo cutoff offset.
        let raw_r = raw * (1.0 - stereo_drift);
        let raw_l = raw * (1.0 + stereo_drift);
        lp_state_l += alpha * (raw_l - lp_state_l);
        lp_state_r += alpha * (raw_r - lp_state_r);

        (lp_state_l, lp_state_r)
    })
}

pub fn pre_tool_use(seed: u64, sample_rate: u32) -> Voice {
    ambient_blip(seed, sample_rate, 660.0)
}

pub fn post_tool_use(seed: u64, sample_rate: u32) -> Voice {
    ambient_blip(seed, sample_rate, 880.0)
}

pub fn user_prompt_submit(seed: u64, sample_rate: u32) -> Voice {
    ambient_blip(seed, sample_rate, 1100.0)
}
