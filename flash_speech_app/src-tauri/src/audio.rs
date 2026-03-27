use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::JoinHandle;
use std::time::Instant;

const MIN_DURATION: f64 = 0.3;
/// Ring buffer capacity: 30 seconds at 48kHz mono
const RING_BUFFER_CAPACITY: usize = 48000 * 30;

pub struct AudioRecorder {
    recording: Arc<AtomicBool>,
    consumer: Arc<Mutex<Option<rtrb::Consumer<f32>>>>,
    device_sample_rate: Arc<Mutex<u32>>,
    start_time: Arc<Mutex<Option<Instant>>>,
    thread_handle: Mutex<Option<JoinHandle<()>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            recording: Arc::new(AtomicBool::new(false)),
            consumer: Arc::new(Mutex::new(None)),
            device_sample_rate: Arc::new(Mutex::new(16000)),
            start_time: Arc::new(Mutex::new(None)),
            thread_handle: Mutex::new(None),
        }
    }

    pub fn start(&self) -> Result<(), String> {
        if self.recording.load(Ordering::Acquire) {
            return Ok(());
        }

        // Create a fresh ring buffer for this recording session
        let (producer, consumer) = rtrb::RingBuffer::new(RING_BUFFER_CAPACITY);
        *self.consumer.lock().unwrap() = Some(consumer);
        *self.start_time.lock().unwrap() = Some(Instant::now());
        self.recording.store(true, Ordering::Release);

        let recording = self.recording.clone();
        let device_sample_rate = self.device_sample_rate.clone();

        let handle = std::thread::spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    eprintln!("[audio] No input device found");
                    recording.store(false, Ordering::Release);
                    return;
                }
            };

            // Try to find a config that supports 16kHz to skip resampling
            let (stream_config, sample_rate, channels, sample_format) = match try_16khz_config(&device) {
                Some(cfg) => cfg,
                None => {
                    // Fall back to device default
                    let config = match device.default_input_config() {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("[audio] Failed to get input config: {}", e);
                            recording.store(false, Ordering::Release);
                            return;
                        }
                    };
                    let sr = config.sample_rate().0;
                    let ch = config.channels() as usize;
                    let sf = config.sample_format();
                    let sc: cpal::StreamConfig = config.into();
                    (sc, sr, ch, sf)
                }
            };

            *device_sample_rate.lock().unwrap() = sample_rate;
            eprintln!(
                "[audio] Device: rate={}Hz, channels={}, format={:?}",
                sample_rate, channels, sample_format
            );

            let err_fn = |err: cpal::StreamError| {
                eprintln!("[audio] Stream error: {}", err);
            };

            let stream = match sample_format {
                cpal::SampleFormat::F32 => {
                    let rec = recording.clone();
                    let mut prod = producer;
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if !rec.load(Ordering::Relaxed) {
                                return;
                            }
                            if channels > 1 {
                                for chunk in data.chunks(channels) {
                                    let _ = prod.push(chunk[0]);
                                }
                            } else {
                                // Write as many samples as the ring buffer can accept
                                if let Ok(mut chunk) = prod.write_chunk_uninit(data.len()) {
                                    let (first, second) = chunk.as_mut_slices();
                                    let first_len = first.len();
                                    for (slot, &sample) in first.iter_mut().zip(data.iter()) {
                                        slot.write(sample);
                                    }
                                    for (slot, &sample) in second.iter_mut().zip(data[first_len..].iter()) {
                                        slot.write(sample);
                                    }
                                    unsafe {
                                        chunk.commit_all();
                                    }
                                } else {
                                    // Fallback: push sample by sample (ring buffer nearly full)
                                    for &s in data {
                                        let _ = prod.push(s);
                                    }
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    let rec = recording.clone();
                    let mut prod = producer;
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if !rec.load(Ordering::Relaxed) {
                                return;
                            }
                            if channels > 1 {
                                for chunk in data.chunks(channels) {
                                    let _ = prod.push(chunk[0] as f32 / 32768.0);
                                }
                            } else {
                                for &s in data {
                                    let _ = prod.push(s as f32 / 32768.0);
                                }
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                fmt => {
                    eprintln!("[audio] Unsupported sample format: {:?}", fmt);
                    recording.store(false, Ordering::Release);
                    return;
                }
            };

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[audio] Failed to build stream: {}", e);
                    recording.store(false, Ordering::Release);
                    return;
                }
            };

            if let Err(e) = stream.play() {
                eprintln!("[audio] Failed to start stream: {}", e);
                recording.store(false, Ordering::Release);
                return;
            }

            // Park instead of polling with sleep(20ms)
            while recording.load(Ordering::Acquire) {
                std::thread::park();
            }
            // stream drops here, closing the audio device
        });

        *self.thread_handle.lock().unwrap() = Some(handle);
        Ok(())
    }

    /// Stop recording and return (samples, device_sample_rate) if valid audio was captured.
    pub fn stop(&self) -> Option<(Vec<f32>, u32)> {
        if !self.recording.load(Ordering::Acquire) {
            return None;
        }

        let duration = self
            .start_time
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        self.recording.store(false, Ordering::Release);
        *self.start_time.lock().unwrap() = None;

        // Unpark and join the recording thread (replaces sleep(50ms))
        if let Some(handle) = self.thread_handle.lock().unwrap().take() {
            handle.thread().unpark();
            let _ = handle.join();
        }

        if duration < MIN_DURATION {
            eprintln!(
                "[audio] Recording too short ({:.2}s < {:.2}s), discarded",
                duration, MIN_DURATION
            );
            // Drain and discard the consumer
            if let Some(mut consumer) = self.consumer.lock().unwrap().take() {
                while consumer.pop().is_ok() {}
            }
            return None;
        }

        // Drain all samples from the ring buffer
        let samples: Vec<f32> = if let Some(mut consumer) = self.consumer.lock().unwrap().take() {
            let available = consumer.slots();
            let mut buf = Vec::with_capacity(available);
            while let Ok(s) = consumer.pop() {
                buf.push(s);
            }
            buf
        } else {
            return None;
        };

        let sample_rate = *self.device_sample_rate.lock().unwrap();

        if samples.is_empty() {
            return None;
        }

        eprintln!(
            "[audio] Captured {} samples at {}Hz ({:.2}s)",
            samples.len(),
            sample_rate,
            duration
        );
        Some((samples, sample_rate))
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Acquire)
    }
}

/// Try to configure the device for 16kHz mono recording.
/// Returns (StreamConfig, sample_rate, channels, format) if successful.
fn try_16khz_config(device: &cpal::Device) -> Option<(cpal::StreamConfig, u32, usize, cpal::SampleFormat)> {
    let configs = device.supported_input_configs().ok()?;
    for cfg_range in configs {
        let min = cfg_range.min_sample_rate().0;
        let max = cfg_range.max_sample_rate().0;
        if min <= 16000 && max >= 16000 {
            let cfg = cfg_range.with_sample_rate(cpal::SampleRate(16000));
            let channels = cfg.channels() as usize;
            let format = cfg.sample_format();
            let stream_config: cpal::StreamConfig = cfg.into();
            eprintln!("[audio] Using direct 16kHz recording (skipping resample)");
            return Some((stream_config, 16000, channels, format));
        }
    }
    None
}
