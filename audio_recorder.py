import sounddevice as sd
import numpy as np
import threading

class AudioRecorder:
    def __init__(self):
        self.fs = 16000
        self.channels = 1
        self.recording = False
        self.audio_data = []
        self.stream = None

    def start_recording(self):
        if self.recording:
            return
        self.recording = True
        self.audio_data = []
        
        def callback(indata, frames, time, status):
            if status:
                print(status)
            if self.recording:
                self.audio_data.append(indata.copy())

        # dtype='int16' ensures we get raw 16-bit PCM samples which matches what engine.py expects
        self.stream = sd.InputStream(samplerate=self.fs, 
                                     channels=self.channels, 
                                     dtype='int16',
                                     callback=callback)
        self.stream.start()

    def stop_recording(self):
        if not self.recording:
            return None
        self.recording = False
        if self.stream:
            self.stream.stop()
            self.stream.close()
            self.stream = None
        
        if not self.audio_data:
            return None
            
        # Concatenate all chunks and convert to bytes
        recording = np.concatenate(self.audio_data, axis=0)
        return recording.tobytes()

    def terminate(self):
        pass
