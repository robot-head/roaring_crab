//! Plays every patch with three seeds for sound design iteration.
//!
//! Run: `cargo run --release --example audition`

#[cfg(feature = "null-audio")]
fn main() {
    eprintln!("audition requires real audio output; run without --features null-audio");
}

#[cfg(not(feature = "null-audio"))]
fn main() {
    use roaring_crab::audio_sink::real::CpalSink;
    use roaring_crab::hook_event::HookEvent;
    use roaring_crab::mixer::Mixer;
    use roaring_crab::patches;
    use std::sync::Arc;
    use std::time::Duration;

    let mixer = Arc::new(Mixer::new(48000));
    mixer.set_master_volume(0.7);
    let mixer_for_cb = mixer.clone();
    let _sink =
        CpalSink::open(Box::new(move |buf| mixer_for_cb.render(buf))).expect("open audio output");

    for event in HookEvent::ALL {
        println!("---- {:?} ----", event);
        for seed in [1u64, 42, 9001] {
            println!("  seed {}", seed);
            mixer.push(patches::build(event, seed, mixer.sample_rate()));
            std::thread::sleep(Duration::from_millis(1100));
        }
    }
    std::thread::sleep(Duration::from_secs(1));
}
