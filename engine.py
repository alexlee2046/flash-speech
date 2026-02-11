import os
import re
import wave

# 仅设置模型缓存路径，不修改 HOME 环境变量
model_cache = os.path.join(os.getcwd(), "model_cache")
os.makedirs(model_cache, exist_ok=True)
os.environ["MODELSCOPE_CACHE"] = model_cache

from funasr import AutoModel


class SpeechEngine:
    def __init__(self):
        print("Loading FunASR model... (this may take a while on first run)")
        device = "cpu"

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
        try:
            temp_file = "temp_recording.wav"
            with wave.open(temp_file, "wb") as wf:
                wf.setnchannels(1)
                wf.setsampwidth(2)
                wf.setframerate(16000)
                wf.writeframes(audio_data)

            res = self.model.generate(
                input=temp_file,
                cache={},
                language="zh",
                use_itn=True,
                batch_size_s=60,
                merge_vad=True,
                merge_length_s=15,
            )

            if res and len(res) > 0:
                text = res[0]['text']
                clean_text = re.sub(r'<\|.*?\|>', '', text).strip()
                return clean_text
            return ""
        except Exception as e:
            print(f"Transcription error: {e}")
            return ""
