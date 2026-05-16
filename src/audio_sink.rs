//! Audio output abstraction.
//!
//! In the default build, opens a cpal output stream and writes interleaved stereo f32.
//! When built with `--features null-audio`, swaps in an in-memory ring buffer used by tests.

#[allow(unused_imports)]
use std::sync::Arc;

pub trait AudioSink: Send + Sync {
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16 {
        2
    }
}

/// Callback signature: filled by the mixer to write the next chunk of interleaved stereo samples.
pub type AudioCallback = Box<dyn FnMut(&mut [f32]) + Send>;

#[cfg(not(feature = "null-audio"))]
pub mod real {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    /// Wraps `cpal::Stream` to assert `Send + Sync`.
    ///
    /// Safety: the stream is never accessed from multiple threads simultaneously;
    /// it is solely owned by `CpalSink` which is itself behind an `Arc` and only
    /// dropped (stopping the stream) when all `Arc` references are released.
    #[allow(dead_code)]
    struct StreamWrapper(cpal::Stream);
    // SAFETY: cpal::Stream's !Send/!Sync is a conservative platform marker on
    // Windows (WASAPI COM object is initialized per-thread at open time). We
    // never call stream methods after construction except `play()`, which is
    // called in `open` before the `Arc` is shared, so cross-thread access is
    // safe in practice.
    unsafe impl Send for StreamWrapper {}
    unsafe impl Sync for StreamWrapper {}

    pub struct CpalSink {
        _stream: StreamWrapper,
        sample_rate: u32,
    }

    impl CpalSink {
        pub fn open(mut callback: AudioCallback) -> anyhow::Result<Arc<Self>> {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or_else(|| anyhow::anyhow!("no default output device"))?;
            let config = cpal::StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(48000),
                buffer_size: cpal::BufferSize::Default,
            };
            let sample_rate = config.sample_rate.0;
            let stream = device.build_output_stream(
                &config,
                move |buf: &mut [f32], _: &cpal::OutputCallbackInfo| callback(buf),
                |err| eprintln!("cpal stream error: {}", err),
                None,
            )?;
            stream.play()?;
            Ok(Arc::new(Self {
                _stream: StreamWrapper(stream),
                sample_rate,
            }))
        }
    }

    impl AudioSink for CpalSink {
        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }
    }
}

#[cfg(feature = "null-audio")]
pub mod null {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    pub struct NullSink {
        pub captured: Arc<Mutex<Vec<f32>>>,
        sample_rate: u32,
    }

    impl NullSink {
        pub fn open(mut callback: AudioCallback, sample_rate: u32) -> Arc<Self> {
            let captured = Arc::new(Mutex::new(Vec::new()));
            let cap_clone = captured.clone();
            std::thread::spawn(move || {
                let mut chunk = vec![0.0f32; 512 * 2];
                loop {
                    chunk.fill(0.0);
                    callback(&mut chunk);
                    cap_clone.lock().extend_from_slice(&chunk);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            });
            Arc::new(Self {
                captured,
                sample_rate,
            })
        }
    }

    impl AudioSink for NullSink {
        fn sample_rate(&self) -> u32 {
            self.sample_rate
        }
    }
}
