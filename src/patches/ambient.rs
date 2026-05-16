use crate::mixer::Voice;
use crate::patches::{adsr, phase, rng_for};
use rand::Rng;

const DUR_MS: u32 = 160;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Short filtered blip. Slightly different center frequency + filter sweep per patch.
fn ambient_blip(seed: u64, sample_rate: u32, center_hz: f32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.gen_range(-12.0..12.0);
    let cutoff_drift: f32 = rng.gen_range(0.5..1.5);
    let base = center_hz * 2.0f32.powf(detune / 1200.0);
    Voice::from_fn(total, move |t| {
        let env = adsr(t, total, 0.10, 0.30, 0.4, 0.60);
        let p1 = phase(t, sample_rate, base);
        let p2 = phase(t, sample_rate, base * 2.0);
        let tri = 2.0 / std::f32::consts::PI * p1.sin();
        let saw = (p2.sin() * 0.3) * cutoff_drift;
        let sample = 0.18 * env * (tri + saw);
        (sample, sample)
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
