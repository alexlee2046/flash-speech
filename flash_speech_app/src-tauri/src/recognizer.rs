use sherpa_rs::sense_voice::{SenseVoiceConfig, SenseVoiceRecognizer};

pub struct SpeechRecognizer {
    recognizer: SenseVoiceRecognizer,
}

impl SpeechRecognizer {
    pub fn new(model_path: &str, tokens_path: &str) -> Result<Self, String> {
        let config = SenseVoiceConfig {
            model: model_path.to_string(),
            tokens: tokens_path.to_string(),
            use_itn: true,
            ..Default::default()
        };

        let recognizer = SenseVoiceRecognizer::new(config)
            .map_err(|e| format!("Failed to create recognizer: {}", e))?;

        Ok(Self { recognizer })
    }

    pub fn transcribe(&mut self, sample_rate: u32, samples: &[f32]) -> Option<String> {
        let result = self.recognizer.transcribe(sample_rate, samples);
        let text = result.text.trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}
