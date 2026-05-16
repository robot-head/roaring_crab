use crate::mixer::Voice;

pub fn notification(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}

pub fn stop(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}

pub fn subagent_stop(_seed: u64, _sample_rate: u32) -> Voice {
    Voice::from_fn(1, |_| (0.0, 0.0))
}
