use crate::mixer::Voice;

pub fn session_start(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}

pub fn session_end(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}

pub fn pre_compact(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}
