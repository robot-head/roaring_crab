use crate::mixer::Voice;
use crate::patches::{adsr, cents, chord_attack, chord_step, chords, phase, rng_for, ProgChord};
use rand::RngExt;

const DUR_MS: u32 = 900;

fn dur_samples(sample_rate: u32) -> u32 {
    (DUR_MS * sample_rate / 1000).max(1)
}

/// Notification: a 2-chord rising arpeggio. Each chord plays its notes in
/// quick succession (root → 3rd → 5th), then advances to the next chord.
/// First chord: I (major). Second chord: V (dominant 7), creating a tension
/// that begs for resolution — fitting for "needs your attention".
pub fn notification(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.random_range(-5.0..5.0);
    let chorus_cents: f32 = rng.random_range(5.0..10.0);
    let root = 440.0 * cents(detune);

    // I (major) → V (dom7), each chord arpeggiated over its segment.
    let prog: [ProgChord; 2] = [
        ProgChord {
            root: 1.0,
            shape: &chords::MAJOR,
        },
        ProgChord {
            root: 1.498_307,
            shape: &chords::DOM7,
        },
    ];

    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let env = adsr(t, total, 0.05, 0.15, 0.6, 0.45);
        let (chord_idx, chord_frac) = chord_step(progress, prog.len());
        let chord = prog[chord_idx];
        let chord_root = root * chord.root;

        // Arpeggiate within the chord: pick which interval based on chord_frac.
        let n_notes = chord.shape.len();
        let note_idx = (chord_frac * n_notes as f32)
            .floor()
            .min((n_notes - 1) as f32) as usize;
        let note_frac = (chord_frac * n_notes as f32).fract();
        let hz = chord_root * chord.shape[note_idx];

        // Per-note micro-attack so we hear distinct notes.
        let note_attack = (note_frac * 8.0).min(1.0);
        // Soft chord-boundary attack.
        let c_attack = chord_attack(chord_frac, 0.05);

        let a = phase(t, sample_rate, hz).sin();
        let b = phase(t, sample_rate, hz * cents(chorus_cents)).sin() * 0.85;
        let h = 0.28 * phase(t, sample_rate, hz * 2.0).sin();
        let sample = 0.16 * env * note_attack * c_attack * (a + b + h);
        let stereo_offset = 0.04 * phase(t, sample_rate, hz * cents(-chorus_cents)).sin();
        (sample - stereo_offset, sample + stereo_offset)
    })
}

/// Stop: a ii — V — I turnaround. Each chord rings with shimmer, the final I
/// gets the longest slice (40%) so the resolution lands with weight. Big
/// reverb-friendly tails.
pub fn stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.random_range(-4.0..4.0);
    let trem_hz: f32 = rng.random_range(4.5..7.0);
    let shimmer_amount: f32 = rng.random_range(0.05..0.12);
    let root = 330.0 * cents(detune);

    // ii (minor7) → V (dom7) → I (major7). Slices: 30% / 30% / 40%.
    let prog: [ProgChord; 3] = [
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
            shape: &chords::MAJOR7,
        }, // I7 (resolution)
    ];
    let slice_boundaries: [f32; 3] = [0.30, 0.60, 1.0];

    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let env = adsr(t, total, 0.06, 0.15, 0.78, 0.55);
        let trem = 0.92 + 0.08 * phase(t, sample_rate, trem_hz).sin();

        // Custom non-uniform chord selection.
        let (chord_idx, seg_start) = if progress < slice_boundaries[0] {
            (0usize, 0.0)
        } else if progress < slice_boundaries[1] {
            (1, slice_boundaries[0])
        } else {
            (2, slice_boundaries[1])
        };
        let seg_end = slice_boundaries[chord_idx];
        let seg_len = (seg_end - seg_start).max(1e-6);
        let chord_frac = ((progress - seg_start) / seg_len).clamp(0.0, 1.0);
        let attack = chord_attack(chord_frac, 0.08);

        let chord = prog[chord_idx];
        let chord_root = root * chord.root;

        // Sum all chord tones with light per-tone gain shaping.
        let mut sum = 0.0f32;
        for (i, &iv) in chord.shape.iter().enumerate() {
            let gain = match i {
                0 => 1.0,  // root
                1 => 0.75, // 3rd
                2 => 0.85, // 5th
                _ => 0.65, // 7th if present
            };
            sum += gain * phase(t, sample_rate, chord_root * iv).sin();
        }
        sum /= chord.shape.len() as f32;
        // Shimmer rides higher harmonics for the trippy octave-up halo.
        let shimmer = shimmer_amount * phase(t, sample_rate, chord_root * 4.0).sin();
        let sample = 0.18 * env * trem * attack * (sum + shimmer);
        let widen = 0.02 * phase(t, sample_rate, chord_root * cents(3.0)).sin();
        (sample - widen, sample + widen)
    })
}

/// SubagentStop: a quieter V → I resolution (just two chords, simpler than
/// Stop's full turnaround). Designed to be unobtrusive.
pub fn subagent_stop(seed: u64, sample_rate: u32) -> Voice {
    let total = dur_samples(sample_rate);
    let mut rng = rng_for(seed);
    let detune: f32 = rng.random_range(-5.0..5.0);
    let chorus_cents: f32 = rng.random_range(4.0..8.0);
    let root = 392.0 * cents(detune);

    let prog: [ProgChord; 2] = [
        ProgChord {
            root: 1.498_307,
            shape: &chords::DOM7,
        }, // V7
        ProgChord {
            root: 1.0,
            shape: &chords::MAJOR,
        }, // I
    ];

    Voice::from_fn(total, move |t| {
        let progress = t as f32 / total as f32;
        let env = adsr(t, total, 0.05, 0.22, 0.55, 0.55);
        let (idx, frac) = chord_step(progress, prog.len());
        let attack = chord_attack(frac, 0.08);
        let chord = prog[idx];
        let chord_root = root * chord.root;

        let mut sum = 0.0f32;
        for (i, &iv) in chord.shape.iter().enumerate() {
            let gain = if i == 0 { 1.0 } else { 0.7 };
            sum += gain * phase(t, sample_rate, chord_root * iv).sin();
        }
        sum /= chord.shape.len() as f32;
        let twin = 0.55 * phase(t, sample_rate, chord_root * cents(chorus_cents)).sin();
        let sample = 0.12 * env * attack * (sum + twin);
        let widen = 0.03 * phase(t, sample_rate, chord_root * cents(-chorus_cents)).sin();
        (sample - widen, sample + widen)
    })
}
