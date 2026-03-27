use sherpa_rs::sense_voice::{SenseVoiceConfig, SenseVoiceRecognizer};
use sherpa_rs::whisper::{WhisperConfig, WhisperRecognizer};
use sherpa_rs::paraformer::{ParaformerConfig, ParaformerRecognizer};

pub enum RecognizerType {
    SenseVoice(SenseVoiceRecognizer),
    Whisper(WhisperRecognizer),
    Paraformer(ParaformerRecognizer),
}

pub struct SpeechRecognizer {
    inner: RecognizerType,
}

/// Determine optimal thread count for ONNX inference.
/// Uses available parallelism capped at 8 (avoid E-cores on Apple Silicon).
fn optimal_thread_count() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4) as i32
}

impl SpeechRecognizer {
    pub fn new_sensevoice(model_path: &str, tokens_path: &str) -> Result<Self, String> {
        let num_threads = optimal_thread_count();
        eprintln!("[recognizer] SenseVoice using {} threads", num_threads);

        let config = SenseVoiceConfig {
            model: model_path.to_string(),
            tokens: tokens_path.to_string(),
            use_itn: true,
            language: "zh".to_string(),
            num_threads: Some(num_threads),
            ..Default::default()
        };

        let recognizer = SenseVoiceRecognizer::new(config)
            .map_err(|e| format!("Failed to create SenseVoice recognizer: {}", e))?;

        Ok(Self {
            inner: RecognizerType::SenseVoice(recognizer),
        })
    }

    pub fn new_whisper(encoder: &str, decoder: &str, tokens: &str) -> Result<Self, String> {
        let num_threads = optimal_thread_count();
        eprintln!("[recognizer] Whisper using {} threads", num_threads);

        let config = WhisperConfig {
            encoder: encoder.to_string(),
            decoder: decoder.to_string(),
            tokens: tokens.to_string(),
            language: "en".to_string(),
            num_threads: Some(num_threads),
            ..Default::default()
        };

        let recognizer = WhisperRecognizer::new(config)
            .map_err(|e| format!("Failed to create Whisper recognizer: {}", e))?;

        Ok(Self {
            inner: RecognizerType::Whisper(recognizer),
        })
    }

    pub fn new_paraformer(model: &str, tokens: &str) -> Result<Self, String> {
        let num_threads = optimal_thread_count();
        eprintln!("[recognizer] Paraformer using {} threads", num_threads);

        let config = ParaformerConfig {
            model: model.to_string(),
            tokens: tokens.to_string(),
            num_threads: Some(num_threads),
            ..Default::default()
        };

        let recognizer = ParaformerRecognizer::new(config)
            .map_err(|e| format!("Failed to create Paraformer recognizer: {}", e))?;

        Ok(Self {
            inner: RecognizerType::Paraformer(recognizer),
        })
    }

    pub fn transcribe(&mut self, sample_rate: u32, samples: &[f32]) -> Option<String> {
        let raw = match &mut self.inner {
            RecognizerType::SenseVoice(r) => r.transcribe(sample_rate, samples).text,
            RecognizerType::Whisper(r) => r.transcribe(sample_rate, samples).text,
            RecognizerType::Paraformer(r) => r.transcribe(sample_rate, samples).text,
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else if trimmed.len() == raw.len() {
            Some(raw)
        } else {
            Some(trimmed.to_string())
        }
    }
}
