use roaring_crab::hook_event::HookEvent;
use roaring_crab::mixer::Mixer;
use roaring_crab::patches;

const SR: u32 = 48000;

/// Render the voice into a buffer until it's finished. Returns the buffer.
fn render_voice(voice: roaring_crab::mixer::Voice) -> Vec<f32> {
    let m = Mixer::new(SR);
    m.set_master_volume(1.0);
    m.push(voice);
    let mut out = Vec::with_capacity(SR as usize * 2 * 2);
    while m.voice_count() > 0 {
        let mut chunk = vec![0.0f32; 256 * 2];
        m.render(&mut chunk);
        out.extend_from_slice(&chunk);
    }
    out
}

fn assert_patch_contract(event: HookEvent, seed: u64) {
    let voice = patches::build(event, seed, SR);
    let samples = render_voice(voice);
    assert!(!samples.is_empty(), "{:?}: empty render", event);
    for s in &samples {
        assert!(s.is_finite(), "{:?}: NaN/inf sample", event);
        assert!(s.abs() <= 1.0, "{:?}: sample out of range: {}", event, s);
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!(rms > 0.001, "{:?}: silent output (rms={})", event, rms);
}

fn assert_variation(event: HookEvent) {
    let a = render_voice(patches::build(event, 1, SR));
    let b = render_voice(patches::build(event, 999, SR));
    let same = a.len() == b.len()
        && a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-7);
    assert!(!same, "{:?}: same audio for different seeds", event);
}

#[test] fn pre_tool_use_contract() { assert_patch_contract(HookEvent::PreToolUse, 1); }
#[test] fn pre_tool_use_varies() { assert_variation(HookEvent::PreToolUse); }
#[test] fn post_tool_use_contract() { assert_patch_contract(HookEvent::PostToolUse, 1); }
#[test] fn post_tool_use_varies() { assert_variation(HookEvent::PostToolUse); }
#[test] fn user_prompt_submit_contract() { assert_patch_contract(HookEvent::UserPromptSubmit, 1); }
#[test] fn user_prompt_submit_varies() { assert_variation(HookEvent::UserPromptSubmit); }

#[test] fn session_start_contract() { assert_patch_contract(HookEvent::SessionStart, 1); }
#[test] fn session_start_varies() { assert_variation(HookEvent::SessionStart); }
#[test] fn session_end_contract() { assert_patch_contract(HookEvent::SessionEnd, 1); }
#[test] fn session_end_varies() { assert_variation(HookEvent::SessionEnd); }
#[test] fn pre_compact_contract() { assert_patch_contract(HookEvent::PreCompact, 1); }
#[test] fn pre_compact_varies() { assert_variation(HookEvent::PreCompact); }

#[test] fn notification_contract() { assert_patch_contract(HookEvent::Notification, 1); }
#[test] fn notification_varies() { assert_variation(HookEvent::Notification); }
#[test] fn stop_contract() { assert_patch_contract(HookEvent::Stop, 1); }
#[test] fn stop_varies() { assert_variation(HookEvent::Stop); }
#[test] fn subagent_stop_contract() { assert_patch_contract(HookEvent::SubagentStop, 1); }
#[test] fn subagent_stop_varies() { assert_variation(HookEvent::SubagentStop); }
