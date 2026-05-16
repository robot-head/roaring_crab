use crate::mixer::Voice;

pub fn pre_tool_use(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}

pub fn post_tool_use(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}

pub fn user_prompt_submit(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}
