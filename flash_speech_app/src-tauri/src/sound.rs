use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, OnceLock};

const VOLUME: f32 = 0.5;

enum SoundCommand {
    PlayStart,
    PlayStop,
}

static SOUND_TX: OnceLock<mpsc::Sender<SoundCommand>> = OnceLock::new();

/// Initialize the sound system. Call once at app startup.
/// Spawns a background thread that owns the cpal output stream.
pub fn init() {
    let (tx, rx) = mpsc::channel::<SoundCommand>();
    SOUND_TX.get_or_init(|| tx);

    std::thread::spawn(move || {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("[sound] No output device");
                return;
            }
        };

        let supported = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[sound] No output config: {}", e);
                return;
            }
        };

        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let stream_config: cpal::StreamConfig = supported.into();

        // Pre-compute waveforms
        let start_beep = generate_samples(sample_rate, &[(880.0, 0.15)]);
        let stop_beep = generate_samples(sample_rate, &[(440.0, 0.1), (0.0, 0.05), (440.0, 0.1)]);

        // Shared state for the audio callback
        let current_samples: Arc<std::sync::Mutex<Option<Arc<[f32]>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let position = Arc::new(AtomicUsize::new(0));
        let playing = Arc::new(AtomicBool::new(false));

        let cs = current_samples.clone();
        let pos = position.clone();
        let pl = playing.clone();

        let stream = device.build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                if !pl.load(Ordering::Relaxed) {
                    // Fill with silence when not playing
                    for s in data.iter_mut() {
                        *s = 0.0;
                    }
                    return;
                }

                let samples_guard = cs.lock().unwrap();
                let samples = match samples_guard.as_ref() {
                    Some(s) => s.clone(),
                    None => {
                        for s in data.iter_mut() {
                            *s = 0.0;
                        }
                        return;
                    }
                };
                drop(samples_guard);

                let total = samples.len();
                for frame in data.chunks_mut(channels) {
                    let p = pos.fetch_add(1, Ordering::Relaxed);
                    let val = if p < total { samples[p] } else { 0.0 };
                    for s in frame.iter_mut() {
                        *s = val;
                    }
                    if p >= total {
                        pl.store(false, Ordering::Relaxed);
                    }
                }
            },
            |err| eprintln!("[sound] Error: {}", err),
            None,
        );

        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[sound] Failed to build output stream: {}", e);
                return;
            }
        };

        if let Err(e) = stream.play() {
            eprintln!("[sound] Failed to start output stream: {}", e);
            return;
        }

        eprintln!("[sound] Persistent output stream initialized at {}Hz", sample_rate);

        // Event loop: receive commands and trigger playback
        for cmd in rx {
            let samples = match cmd {
                SoundCommand::PlayStart => start_beep.clone(),
                SoundCommand::PlayStop => stop_beep.clone(),
            };

            *current_samples.lock().unwrap() = Some(samples);
            position.store(0, Ordering::Relaxed);
            playing.store(true, Ordering::Relaxed);
        }

        // rx dropped = app shutting down, stream drops here
    });
}

/// Play start-recording beep (non-blocking).
pub fn play_start_sound() {
    if let Some(tx) = SOUND_TX.get() {
        let _ = tx.send(SoundCommand::PlayStart);
    }
}

/// Play stop-recording beep (non-blocking).
pub fn play_stop_sound() {
    if let Some(tx) = SOUND_TX.get() {
        let _ = tx.send(SoundCommand::PlayStop);
    }
}

/// Pre-generate waveform samples for a tone sequence.
fn generate_samples(sample_rate: u32, segments: &[(f32, f32)]) -> Arc<[f32]> {
    let total_samples: usize = segments
        .iter()
        .map(|&(_, dur)| (sample_rate as f32 * dur) as usize)
        .sum();

    let mut samples = Vec::with_capacity(total_samples);
    for &(freq, dur) in segments {
        let num = (sample_rate as f32 * dur) as usize;
        if freq > 0.0 {
            let omega = 2.0 * PI * freq / sample_rate as f32;
            for i in 0..num {
                samples.push(VOLUME * (omega * i as f32).sin());
            }
        } else {
            samples.resize(samples.len() + num, 0.0);
        }
    }
    samples.into()
}
