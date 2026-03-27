use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

const MIN_DURATION: f64 = 0.3;

pub struct AudioRecorder {
    recording: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    device_sample_rate: Arc<Mutex<u32>>,
    start_time: Arc<Mutex<Option<Instant>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            recording: Arc::new(AtomicBool::new(false)),
            buffer: Arc::new(Mutex::new(Vec::new())),
            device_sample_rate: Arc::new(Mutex::new(16000)),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    pub fn start(&self) -> Result<(), String> {
        if self.recording.load(Ordering::SeqCst) {
            return Ok(());
        }

        let rate = *self.device_sample_rate.lock().unwrap();
        let mut buf = self.buffer.lock().unwrap();
        buf.clear();
        buf.reserve((rate as usize) * 10);
        drop(buf);
        *self.start_time.lock().unwrap() = Some(Instant::now());
        self.recording.store(true, Ordering::SeqCst);

        let recording = self.recording.clone();
        let buffer = self.buffer.clone();
        let device_sample_rate = self.device_sample_rate.clone();

        std::thread::spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_input_device() {
                Some(d) => d,
                None => {
                    eprintln!("[audio] No input device found");
                    recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let config = match device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[audio] Failed to get input config: {}", e);
                    recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let sample_rate = config.sample_rate().0;
            let channels = config.channels() as usize;
            let sample_format = config.sample_format();
            let stream_config: cpal::StreamConfig = config.into();

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
                    let buf = buffer.clone();
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[f32], _: &cpal::InputCallbackInfo| {
                            if !rec.load(Ordering::Relaxed) {
                                return;
                            }
                            let mut b = buf.lock().unwrap();
                            if channels > 1 {
                                b.extend(data.chunks(channels).map(|c| c[0]));
                            } else {
                                b.extend_from_slice(data);
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                cpal::SampleFormat::I16 => {
                    let rec = recording.clone();
                    let buf = buffer.clone();
                    device.build_input_stream(
                        &stream_config,
                        move |data: &[i16], _: &cpal::InputCallbackInfo| {
                            if !rec.load(Ordering::Relaxed) {
                                return;
                            }
                            let mut b = buf.lock().unwrap();
                            if channels > 1 {
                                b.extend(
                                    data.chunks(channels)
                                        .map(|c| c[0] as f32 / 32768.0),
                                );
                            } else {
                                b.extend(data.iter().map(|&s| s as f32 / 32768.0));
                            }
                        },
                        err_fn,
                        None,
                    )
                }
                fmt => {
                    eprintln!("[audio] Unsupported sample format: {:?}", fmt);
                    recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let stream = match stream {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[audio] Failed to build stream: {}", e);
                    recording.store(false, Ordering::SeqCst);
                    return;
                }
            };

            if let Err(e) = stream.play() {
                eprintln!("[audio] Failed to start stream: {}", e);
                recording.store(false, Ordering::SeqCst);
                return;
            }

            while recording.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(20));
            }
            // stream drops here, closing the audio device
        });

        Ok(())
    }

    /// Stop recording and return (samples, device_sample_rate) if valid audio was captured.
    pub fn stop(&self) -> Option<(Vec<f32>, u32)> {
        if !self.recording.load(Ordering::SeqCst) {
            return None;
        }

        let duration = self
            .start_time
            .lock()
            .unwrap()
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);

        self.recording.store(false, Ordering::SeqCst);
        *self.start_time.lock().unwrap() = None;

        // Wait for recording thread to finish and release the stream
        std::thread::sleep(Duration::from_millis(50));

        if duration < MIN_DURATION {
            eprintln!(
                "[audio] Recording too short ({:.2}s < {:.2}s), discarded",
                duration, MIN_DURATION
            );
            self.buffer.lock().unwrap().clear();
            return None;
        }

        let samples = std::mem::take(&mut *self.buffer.lock().unwrap());
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
        self.recording.load(Ordering::SeqCst)
    }
}
