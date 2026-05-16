use parking_lot::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub const MAX_VOICES: usize = 16;

/// A single voice produces (left, right) samples per call. When `samples_remaining`
/// hits zero, the voice is finished and the mixer removes it.
pub struct Voice {
    pub(crate) samples_remaining: u32,
    t_samples: u32,
    f: Box<dyn FnMut(u32) -> (f32, f32) + Send>,
}

impl Voice {
    pub fn from_fn<F: FnMut(u32) -> (f32, f32) + Send + 'static>(
        samples: u32,
        f: F,
    ) -> Self {
        Self {
            samples_remaining: samples,
            t_samples: 0,
            f: Box::new(f),
        }
    }

    fn pump(&mut self) -> Option<(f32, f32)> {
        if self.samples_remaining == 0 {
            return None;
        }
        let s = (self.f)(self.t_samples);
        self.t_samples = self.t_samples.saturating_add(1);
        self.samples_remaining -= 1;
        Some(s)
    }
}

pub struct Mixer {
    sample_rate: u32,
    voices: Arc<Mutex<Vec<Voice>>>,
    /// Master volume stored as Q15 (0..=32768).
    master_volume_q15: AtomicU32,
}

impl Mixer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            voices: Arc::new(Mutex::new(Vec::new())),
            master_volume_q15: AtomicU32::new((0.7f32 * 32768.0) as u32),
        }
    }

    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn voice_count(&self) -> usize { self.voices.lock().len() }

    pub fn set_master_volume(&self, v: f32) {
        let q = (v.clamp(0.0, 1.0) * 32768.0) as u32;
        self.master_volume_q15.store(q, Ordering::Relaxed);
    }

    pub fn master_volume(&self) -> f32 {
        (self.master_volume_q15.load(Ordering::Relaxed) as f32) / 32768.0
    }

    pub fn push(&self, voice: Voice) {
        let mut voices = self.voices.lock();
        if voices.len() >= MAX_VOICES {
            voices.remove(0); // drop oldest
        }
        voices.push(voice);
    }

    /// Fill `buf` with interleaved stereo samples.
    pub fn render(&self, buf: &mut [f32]) {
        let vol = self.master_volume();
        let mut voices = self.voices.lock();
        let frames = buf.len() / 2;
        for frame in 0..frames {
            let mut l = 0.0f32;
            let mut r = 0.0f32;
            for v in voices.iter_mut() {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| v.pump()));
                match result {
                    Ok(Some((vl, vr))) => {
                        l += vl;
                        r += vr;
                    }
                    Ok(None) => {}
                    Err(_) => {
                        // Voice panicked; mark for removal and continue mixing.
                        v.samples_remaining = 0;
                    }
                }
            }
            buf[frame * 2] = (l * vol).clamp(-1.0, 1.0);
            buf[frame * 2 + 1] = (r * vol).clamp(-1.0, 1.0);
        }
        voices.retain(|v| v.samples_remaining > 0);
    }
}

impl Default for Mixer {
    fn default() -> Self { Self::new(48000) }
}
