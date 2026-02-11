import time
import sounddevice as sd
import numpy as np


class AudioRecorder:
    # 最短有效录音时长（秒），低于此值的录音将被丢弃
    MIN_DURATION = 0.3

    def __init__(self):
        self.fs = 16000
        self.channels = 1
        self.recording = False
        self.audio_data = []
        self.stream = None
        self._start_time = None

    def start_recording(self):
        if self.recording:
            return
        self.recording = True
        self.audio_data = []
        self._start_time = time.monotonic()

        def callback(indata, frames, time_info, status):
            if status:
                print(status)
            if self.recording:
                self.audio_data.append(indata.copy())

        self.stream = sd.InputStream(
            samplerate=self.fs,
            channels=self.channels,
            dtype='int16',
            callback=callback,
        )
        self.stream.start()

    def stop_recording(self):
        if not self.recording:
            return None
        self.recording = False
        duration = time.monotonic() - self._start_time if self._start_time else 0
        self._start_time = None

        if self.stream:
            self.stream.stop()
            self.stream.close()
            self.stream = None

        if not self.audio_data:
            return None

        # 录音时长过短，直接丢弃
        if duration < self.MIN_DURATION:
            print(f"录音过短 ({duration:.2f}s < {self.MIN_DURATION}s)，已丢弃")
            self.audio_data = []
            return None

        recording = np.concatenate(self.audio_data, axis=0)
        self.audio_data = []
        return recording.tobytes()

    @property
    def elapsed(self):
        """返回当前录音已持续的秒数，未录音时返回 0"""
        if self.recording and self._start_time:
            return time.monotonic() - self._start_time
        return 0.0

    def terminate(self):
        """安全关闭录音资源"""
        self.recording = False
        if self.stream:
            try:
                self.stream.stop()
                self.stream.close()
            except Exception:
                pass
            self.stream = None
        self.audio_data = []
