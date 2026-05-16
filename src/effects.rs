//! Master-bus effects: a Schroeder-style reverb and a stereo ping-pong delay.
//!
//! Both are simple ring-buffer-based DSP units sized at construction time.
//! They process interleaved stereo samples in-place via `MasterFx::process`
//! and are owned by the mixer (single audio thread, no shared mutability).

const REVERB_COMB_DELAYS_MS: [f32; 4] = [29.7, 37.1, 41.1, 43.7];
const REVERB_COMB_FEEDBACKS: [f32; 4] = [0.825, 0.835, 0.815, 0.805];
const REVERB_ALLPASS_DELAYS_MS: [f32; 2] = [5.0, 1.7];
const REVERB_ALLPASS_FEEDBACK: f32 = 0.5;

/// A single feedback comb filter.
struct Comb {
    buf: Vec<f32>,
    pos: usize,
    feedback: f32,
}

impl Comb {
    fn new(delay_samples: usize, feedback: f32) -> Self {
        Self {
            buf: vec![0.0; delay_samples.max(1)],
            pos: 0,
            feedback,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.buf[self.pos];
        self.buf[self.pos] = x + y * self.feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        y
    }
}

/// A single Schroeder allpass.
struct Allpass {
    buf: Vec<f32>,
    pos: usize,
    feedback: f32,
}

impl Allpass {
    fn new(delay_samples: usize, feedback: f32) -> Self {
        Self {
            buf: vec![0.0; delay_samples.max(1)],
            pos: 0,
            feedback,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let d = self.buf[self.pos];
        let y = -self.feedback * x + d;
        self.buf[self.pos] = x + d * self.feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        y
    }
}

/// Schroeder reverb: 4 parallel combs feed 2 series allpasses, mono in / mono out.
struct Reverb {
    combs: [Comb; 4],
    allpasses: [Allpass; 2],
}

impl Reverb {
    fn new(sample_rate: u32) -> Self {
        let ms_to_samples = |ms: f32| (ms * sample_rate as f32 / 1000.0) as usize;
        Self {
            combs: [
                Comb::new(
                    ms_to_samples(REVERB_COMB_DELAYS_MS[0]),
                    REVERB_COMB_FEEDBACKS[0],
                ),
                Comb::new(
                    ms_to_samples(REVERB_COMB_DELAYS_MS[1]),
                    REVERB_COMB_FEEDBACKS[1],
                ),
                Comb::new(
                    ms_to_samples(REVERB_COMB_DELAYS_MS[2]),
                    REVERB_COMB_FEEDBACKS[2],
                ),
                Comb::new(
                    ms_to_samples(REVERB_COMB_DELAYS_MS[3]),
                    REVERB_COMB_FEEDBACKS[3],
                ),
            ],
            allpasses: [
                Allpass::new(
                    ms_to_samples(REVERB_ALLPASS_DELAYS_MS[0]),
                    REVERB_ALLPASS_FEEDBACK,
                ),
                Allpass::new(
                    ms_to_samples(REVERB_ALLPASS_DELAYS_MS[1]),
                    REVERB_ALLPASS_FEEDBACK,
                ),
            ],
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let mut y = 0.0;
        for c in &mut self.combs {
            y += c.process(x);
        }
        y *= 0.25;
        for a in &mut self.allpasses {
            y = a.process(y);
        }
        y
    }
}

/// Stereo ping-pong delay: L feeds R's delay line and vice versa, producing
/// a bouncing-between-ears tail with a shared feedback amount.
struct PingPong {
    left: Vec<f32>,
    right: Vec<f32>,
    pos_l: usize,
    pos_r: usize,
    feedback: f32,
}

impl PingPong {
    fn new(sample_rate: u32, left_ms: f32, right_ms: f32, feedback: f32) -> Self {
        let l = (left_ms * sample_rate as f32 / 1000.0) as usize;
        let r = (right_ms * sample_rate as f32 / 1000.0) as usize;
        Self {
            left: vec![0.0; l.max(1)],
            right: vec![0.0; r.max(1)],
            pos_l: 0,
            pos_r: 0,
            feedback,
        }
    }

    /// Process a stereo frame, returning the wet (delay-only) component.
    fn process(&mut self, l_in: f32, r_in: f32) -> (f32, f32) {
        let l_out = self.left[self.pos_l];
        let r_out = self.right[self.pos_r];
        // Cross-feedback so each side's tail is fed by the other side's previous sample.
        self.left[self.pos_l] = r_in + r_out * self.feedback;
        self.right[self.pos_r] = l_in + l_out * self.feedback;
        self.pos_l = (self.pos_l + 1) % self.left.len();
        self.pos_r = (self.pos_r + 1) % self.right.len();
        (l_out, r_out)
    }
}

/// Master FX chain. Wraps a per-channel reverb and a stereo ping-pong delay.
pub struct MasterFx {
    reverb_l: Reverb,
    reverb_r: Reverb,
    delay: PingPong,
    reverb_wet: f32,
    delay_wet: f32,
}

impl MasterFx {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            reverb_l: Reverb::new(sample_rate),
            reverb_r: Reverb::new(sample_rate),
            // Slightly offset L/R delay times for stereo width; modest feedback.
            delay: PingPong::new(sample_rate, 380.0, 570.0, 0.42),
            reverb_wet: 0.32,
            delay_wet: 0.22,
        }
    }

    /// Process a stereo frame in place. `(l, r)` is the dry mix; the function
    /// returns the dry+wet sum.
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        let (dl, dr) = self.delay.process(l, r);
        let l_pre_rev = l + dl * self.delay_wet;
        let r_pre_rev = r + dr * self.delay_wet;
        let rl = self.reverb_l.process(l_pre_rev);
        let rr = self.reverb_r.process(r_pre_rev);
        (
            l_pre_rev + rl * self.reverb_wet,
            r_pre_rev + rr * self.reverb_wet,
        )
    }
}
