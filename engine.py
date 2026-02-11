import os

# Override HOME to avoid permission issues with ~/.cache, ~/.modelscope, etc.
fake_home = os.path.join(os.getcwd(), "fake_home")
os.makedirs(fake_home, exist_ok=True)
os.environ["HOME"] = fake_home
os.environ["MODELSCOPE_CACHE"] = os.path.join(fake_home, "model_cache")

from funasr import AutoModel
import torch

class SpeechEngine:
    def __init__(self):
        print("Loading FunASR model... (this may take a while on first run)")
        # Check for MPS (Apple Silicon) support
        device = "mps" if torch.backends.mps.is_available() else "cpu"
        # Fallback to cpu if mps causes issues with some ops, but give it a try for speed.
        # Note: Some FunASR ops might not fully support MPS yet, so we might need to fallback to cpu if it crashes.
        # For safety/stability on initial version, let's default to cpu or cuda if available, 
        # as MPS support in customized ops can be tricky. 
        # Actually, for standard inference, CPU on M1/M2 is extremely fast for this model size.
        device = "cpu" 
        
        # SenseVoiceSmall is a very fast and accurate model
        # It handles VAD and Punctuation implicitly for ASR tasks often, or we can use the pipeline.
        # For SenseVoiceSmall in FunASR:
        # model="iic/SenseVoiceSmall"
        
        self.model = AutoModel(
            model="iic/SenseVoiceSmall",
            trust_remote_code=True,
            device=device,
            disable_update=True,
            vad_model="fsmn-vad",
            vad_model_revision="v2.0.4",
        )
        print(f"Model loaded on {device}")

    def transcribe(self, audio_data):
        # FunASR expects a file path or numpy array. 
        # For simplicity/robustness, we can save to a temp file or pass bytes if supported.
        # AutoModel support input as: path, or audio bytes, or numpy array.
        
        # If passed raw bytes of 16kHz mono PCM
        try:
            # Note: generate() API is the main inference entry
            # input can be raw bytes if we treat it right, but easiest is to just write a temp file
            # or wrap it. 
            # Let's write to a temp file to be safe and debuggable.
            temp_file = "temp_recording.wav"
            import wave
            with wave.open(temp_file, "wb") as wf:
                wf.setnchannels(1)
                wf.setsampwidth(2) # 16 bit
                wf.setframerate(16000)
                wf.writeframes(audio_data)
            
            # SenseVoiceSmall inference
            # language="auto" or "zh", "en", etc.
            # use_itn=True for inverse text normalization (numbers to digits)
            res = self.model.generate(
                input=temp_file,
                cache={},
                language="zh",  # Force Chinese for input method context
                use_itn=True,
                batch_size_s=60, 
                merge_vad=True,
                merge_length_s=15,
            )
            
            # SenseVoice output format: [{'key': '...', 'text': '...'}]
            # It's similar to Paraformer.
            if res and len(res) > 0:
                text = res[0]['text']
                # SenseVoice sometimes outputs XML tags for events like <|zh|><|NEUTRAL|><|Speech|>, we should clean them.
                import re
                clean_text = re.sub(r'<\|.*?\|>', '', text).strip()
                return clean_text
            return ""
        except Exception as e:
            print(f"Transcription error: {e}")
            return ""
