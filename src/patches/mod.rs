//! Per-hook sound patches. Each patch is a function returning a `Voice` for the mixer.
//!
//! Patches are organized into three families:
//! - ambient: PreToolUse, PostToolUse, UserPromptSubmit (short blips, 100–200ms)
//! - lifecycle: SessionStart, SessionEnd, PreCompact (warm sweeps, 600–1000ms)
//! - alert: Notification, Stop, SubagentStop (melodic motifs, 400–800ms)

use crate::hook_event::HookEvent;
use crate::mixer::Voice;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub mod alert;
pub mod ambient;
pub mod lifecycle;

/// Produces a voice for the given hook event using the given RNG seed.
pub fn build(event: HookEvent, seed: u64, sample_rate: u32) -> Voice {
    match event {
        HookEvent::PreToolUse => ambient::pre_tool_use(seed, sample_rate),
        HookEvent::PostToolUse => ambient::post_tool_use(seed, sample_rate),
        HookEvent::UserPromptSubmit => ambient::user_prompt_submit(seed, sample_rate),
        HookEvent::SessionStart => lifecycle::session_start(seed, sample_rate),
        HookEvent::SessionEnd => lifecycle::session_end(seed, sample_rate),
        HookEvent::PreCompact => lifecycle::pre_compact(seed, sample_rate),
        HookEvent::Notification => alert::notification(seed, sample_rate),
        HookEvent::Stop => alert::stop(seed, sample_rate),
        HookEvent::SubagentStop => alert::subagent_stop(seed, sample_rate),
    }
}

/// ADSR envelope. Outputs a multiplier in [0, 1] given the current sample index and total length.
#[allow(dead_code)]
pub(crate) fn adsr(t: u32, total: u32, attack: f32, decay: f32, sustain: f32, release: f32) -> f32 {
    let tn = t as f32 / total as f32;
    if tn < attack {
        tn / attack
    } else if tn < attack + decay {
        let dt = (tn - attack) / decay;
        1.0 + (sustain - 1.0) * dt
    } else if tn < 1.0 - release {
        sustain
    } else {
        let rt = (tn - (1.0 - release)) / release;
        sustain * (1.0 - rt)
    }
}

#[allow(dead_code)]
pub(crate) fn rng_for(seed: u64) -> StdRng {
    StdRng::seed_from_u64(seed)
}

/// Quick utility: phase (in radians) at sample index `t` for a `hz`-cycle tone.
#[allow(dead_code)]
pub(crate) fn phase(t: u32, sample_rate: u32, hz: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    two_pi * (t as f32 / sample_rate as f32) * hz
}

/// Convert cents to a frequency ratio. 100 cents = one semitone.
#[allow(dead_code)]
pub(crate) fn cents(c: f32) -> f32 {
    2.0f32.powf(c / 1200.0)
}

/// One-pole lowpass coefficient suitable for `prev + alpha * (sample - prev)`.
/// `cutoff_hz` is the -3 dB point.
#[allow(dead_code)]
pub(crate) fn lp_alpha(sample_rate: u32, cutoff_hz: f32) -> f32 {
    let dt = 1.0 / sample_rate as f32;
    let rc = 1.0 / (std::f32::consts::TAU * cutoff_hz.max(1.0));
    dt / (rc + dt)
}

/// Given normalized progress in [0, 1] and a chord count, returns
/// (chord_index, fractional_position_within_chord).
#[allow(dead_code)]
pub(crate) fn chord_step(progress: f32, n_chords: usize) -> (usize, f32) {
    if n_chords == 0 {
        return (0, 0.0);
    }
    let scaled = progress.clamp(0.0, 0.99999) * n_chords as f32;
    let idx = (scaled.floor() as usize).min(n_chords - 1);
    (idx, (scaled - idx as f32).clamp(0.0, 1.0))
}

/// Quick attack envelope applied at each chord change. `frac` is the
/// fractional position within the current chord segment. Returns a multiplier
/// in [0, 1] that ramps up over the first `attack` fraction of each chord.
#[allow(dead_code)]
pub(crate) fn chord_attack(frac: f32, attack: f32) -> f32 {
    if frac < attack {
        (frac / attack).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Common chord shapes as frequency ratios relative to a root.
/// Each is a triad in just-intonation-ish form (good enough for our purposes).
#[allow(dead_code)]
pub(crate) mod chords {
    /// Minor triad: root, m3, P5
    pub const MINOR: [f32; 3] = [1.0, 1.189_207, 1.498_307];
    /// Major triad: root, M3, P5
    pub const MAJOR: [f32; 3] = [1.0, 1.259_921, 1.498_307];
    /// Sus4: root, P4, P5
    pub const SUS4: [f32; 3] = [1.0, 1.334_84, 1.498_307];
    /// Sus2: root, M2, P5
    pub const SUS2: [f32; 3] = [1.0, 1.122_462, 1.498_307];
    /// Minor 7: root, m3, P5, m7
    pub const MINOR7: [f32; 4] = [1.0, 1.189_207, 1.498_307, 1.781_797];
    /// Dominant 7: root, M3, P5, m7
    pub const DOM7: [f32; 4] = [1.0, 1.259_921, 1.498_307, 1.781_797];
    /// Major 7: root, M3, P5, M7
    pub const MAJOR7: [f32; 4] = [1.0, 1.259_921, 1.498_307, 1.887_749];
}

/// A single chord in a progression: a root multiplier (relative to the patch's
/// fundamental) plus a slice of interval ratios (from `chords::*`).
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct ProgChord {
    /// Root multiplier (e.g., 1.0 = i, 1.498 = v, 1.189 = III in a minor key).
    pub root: f32,
    /// Reference to a chord-shape constant.
    pub shape: &'static [f32],
}
