use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const VOLUME: f32 = 0.5;

/// Play start-recording beep: 880 Hz for 0.15s
pub fn play_start_sound() {
    std::thread::spawn(|| {
        play_tone_sequence(&[(880.0, 0.15)]);
    });
}

/// Play stop-recording beep: 440 Hz 0.1s + silence 0.05s + 440 Hz 0.1s
pub fn play_stop_sound() {
    std::thread::spawn(|| {
        play_tone_sequence(&[(440.0, 0.1), (0.0, 0.05), (440.0, 0.1)]);
    });
}

fn play_tone_sequence(segments: &[(f32, f32)]) {
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

    // Pre-generate all samples
    let mut samples = Vec::new();
    for &(freq, dur) in segments {
        let num = (sample_rate as f32 * dur) as usize;
        for i in 0..num {
            let t = i as f32 / sample_rate as f32;
            let s = if freq > 0.0 {
                VOLUME * (2.0 * PI * freq * t).sin()
            } else {
                0.0
            };
            samples.push(s);
        }
    }

    let total = samples.len();
    let position = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicBool::new(false));

    let pos_c = position.clone();
    let done_c = done.clone();

    let stream = device.build_output_stream(
        &stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let pos = pos_c.fetch_add(1, Ordering::Relaxed);
                let val = if pos < total { samples[pos] } else { 0.0 };
                for s in frame.iter_mut() {
                    *s = val;
                }
                if pos >= total {
                    done_c.store(true, Ordering::Relaxed);
                }
            }
        },
        |err| eprintln!("[sound] Error: {}", err),
        None,
    );

    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[sound] Failed to build stream: {}", e);
            return;
        }
    };

    if let Err(e) = stream.play() {
        eprintln!("[sound] Failed to play: {}", e);
        return;
    }

    while !done.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
    // Extra time for buffer to flush
    std::thread::sleep(Duration::from_millis(50));
}
