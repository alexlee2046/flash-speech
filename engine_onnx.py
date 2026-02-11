import re

try:
    from funasr_onnx import SenseVoiceSmall
except ImportError:
    SenseVoiceSmall = None


class SpeechEngine:
    def __init__(self):
        if SenseVoiceSmall is None:
            raise ImportError("Please install `funasr_onnx` via pip.")

        print("Initializing SenseVoiceSmall ONNX...")
        self.model = SenseVoiceSmall(model_dir="iic/SenseVoiceSmall-ONNX", quantize=True)
        print("Model loaded.")

    def transcribe(self, audio_data):
        try:
            res = self.model(audio_data, language="zh", use_itn=True)
            if isinstance(res, list) and len(res) > 0:
                text = res[0].get('text', '')
                # 清理模型输出中的特殊标签（如 <|zh|>, <|EMO|> 等）
                clean_text = re.sub(r'<\|.*?\|>', '', text).strip()
                return clean_text if clean_text else ""
            return str(res) if res else ""
        except Exception as e:
            print(f"Transcription error: {e}")
            return ""
