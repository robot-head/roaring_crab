//! Per-hook sound patches. Each patch is a function returning a `Voice` for the mixer.
//!
//! Patches are organized into three families:
//! - ambient: PreToolUse, PostToolUse, UserPromptSubmit (short blips, 100–200ms)
//! - lifecycle: SessionStart, SessionEnd, PreCompact (warm sweeps, 600–1000ms)
//! - alert: Notification, Stop, SubagentStop (melodic motifs, 400–800ms)

use crate::hook_event::HookEvent;
use crate::mixer::Voice;
use rand::SeedableRng;
use rand::rngs::StdRng;

pub mod ambient;
pub mod lifecycle;
pub mod alert;

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
