use crate::mixer::Voice;
use crate::patches::{
    adsr, cents, chord_attack, chord_step, chords, lp_alpha, phase, rng_for, ProgChord,
};
use rand::Rng;

const DUR_MS: u32 = 2200;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Wide detuned-stack pad with a chord progression playing underneath.
/// Each chord in `progression` is given an equal time slice of the voice.
/// `direction = +1` swells in, `-1` decays out.
fn pad(
    seed: u64,
    sample_rate: u32,
    root_hz: f32,
    direction: i32,
    progression: &'static [ProgChord],
) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let coarse_detune: f32 = rng.gen_range(-4.0..4.0);
    let spread: f32 = rng.gen_range(6.0..14.0); // cents spread between voices
    let trem_hz: f32 = rng.gen_range(2.3..4.7);
    let filter_lfo_hz: f32 = rng.gen_range(0.18..0.35);
    let cutoff_low: f32 = rng.gen_range(700.0..1100.0);
    let cutoff_high: f32 = rng.gen_range(2200.0..3400.0);

    let root = root_hz * cents(coarse_detune);

    let mut lp_l = 0.0f32;
    let mut lp_r = 0.0f32;

    let n = progression.len();

    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let mut env = adsr(t, total, 0.18, 0.18, 0.78, 0.30);
        if direction < 0 {
            env *= 1.0 - 0.6 * progress;
        }
        let trem = 0.84 + 0.16 * phase(t, sample_rate, trem_hz).sin();

        // Select active chord and apply a soft per-chord attack so transitions
        // are smooth, not abrupt.
        let (idx, frac) = chord_step(progress, n);
        let attack = chord_attack(frac, 0.08);
        let chord = progression[idx];
        let chord_root = root * chord.root;

        // Sum chord tones (per-channel detune for width).
        let mut l = 0.35 * phase(t, sample_rate, chord_root * 0.5).sin(); // sub octave
        let mut r = l;
        for &iv in chord.shape {
            let f_l = chord_root * iv * cents(spread);
            let f_r = chord_root * iv * cents(-spread);
            l += phase(t, sample_rate, f_l).sin();
            r += phase(t, sample_rate, f_r).sin();
        }
        let shimmer = 0.16 * phase(t, sample_rate, chord_root * 4.0).sin();
        l += shimmer;
        r += shimmer * 0.9;

        let denom = (chord.shape.len() as f32) + 1.0;
        let mix_l = 0.13 * env * trem * attack * l / denom;
        let mix_r = 0.13 * env * trem * attack * r / denom;

        let lfo = 0.5 + 0.5 * phase(t, sample_rate, filter_lfo_hz).sin();
        let cutoff = cutoff_low + (cutoff_high - cutoff_low) * lfo;
        let alpha = lp_alpha(sample_rate, cutoff);
        lp_l += alpha * (mix_l - lp_l);
        lp_r += alpha * (mix_r - lp_r);
        (lp_l, lp_r)
    })
}

// SessionStart: rising minor progression — i, III, VII, v.
const SESSION_START_PROG: &[ProgChord] = &[
    ProgChord {
        root: 1.0,
        shape: &chords::MINOR,
    }, // i (Am)
    ProgChord {
        root: 1.189_207,
        shape: &chords::MAJOR,
    }, // III (C)
    ProgChord {
        root: 0.890_899,
        shape: &chords::MAJOR,
    }, // VII (G, below)
    ProgChord {
        root: 1.498_307,
        shape: &chords::MINOR,
    }, // v (Em)
];

// SessionEnd: descending resolution — i, VI, iv, i.
const SESSION_END_PROG: &[ProgChord] = &[
    ProgChord {
        root: 1.0,
        shape: &chords::MINOR,
    }, // i (Am)
    ProgChord {
        root: 1.587_401,
        shape: &chords::MAJOR,
    }, // VI (F above i? actually m6 down)
    ProgChord {
        root: 1.334_84,
        shape: &chords::MINOR,
    }, // iv (Dm)
    ProgChord {
        root: 1.0,
        shape: &chords::MINOR,
    }, // i
];

pub fn session_start(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 220.0, 1, SESSION_START_PROG)
}

pub fn session_end(seed: u64, sample_rate: u32) -> Voice {
    pad(seed, sample_rate, 165.0, -1, SESSION_END_PROG)
}

// PreCompact: ii — V — i turnaround with the two-note rising step on top.
const PRE_COMPACT_PROG: &[ProgChord] = &[
    ProgChord {
        root: 1.122_462,
        shape: &chords::MINOR7,
    }, // ii7
    ProgChord {
        root: 1.498_307,
        shape: &chords::DOM7,
    }, // V7
    ProgChord {
        root: 1.0,
        shape: &chords::MINOR7,
    }, // i7
];

pub fn pre_compact(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let coarse_detune: f32 = rng.gen_range(-6.0..6.0);
    let wobble_hz: f32 = rng.gen_range(4.5..7.0);
    let wobble_depth: f32 = rng.gen_range(2.5..5.5);
    let root = 196.0 * cents(coarse_detune);

    let mut lp = 0.0f32;
    let alpha = lp_alpha(sample_rate, 2200.0);
    let prog = PRE_COMPACT_PROG;

    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let env = adsr(t, total, 0.18, 0.18, 0.7, 0.36);
        let (idx, frac) = chord_step(progress, prog.len());
        let attack = chord_attack(frac, 0.10);
        let chord = prog[idx];
        let chord_root = root * chord.root;

        let wob = cents(wobble_depth * phase(t, sample_rate, wobble_hz).sin());
        // The melodic step continues to alternate between root*3 and root*4
        // (relative to the patch's fundamental), regardless of chord — gives
        // a sense of motion riding over the chord changes.
        let step_hz = if progress < 0.5 {
            root * 3.0
        } else {
            root * 4.0
        } * wob;

        // Underlying chord pad.
        let mut chord_sum = 0.0f32;
        for &iv in chord.shape {
            chord_sum += phase(t, sample_rate, chord_root * iv).sin();
        }
        chord_sum /= chord.shape.len() as f32;
        let sub = 0.4 * phase(t, sample_rate, chord_root * 0.5).sin();
        let step = 0.30 * phase(t, sample_rate, step_hz).sin();

        let raw = 0.14 * env * attack * (chord_sum + sub + step);
        lp += alpha * (raw - lp);
        (lp * 0.96, lp * 1.04)
    })
}
