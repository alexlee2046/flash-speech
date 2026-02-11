import time
import sys
import threading
import queue
import subprocess
import os
import math
import numpy as np
import sounddevice as sd
from pynput import keyboard
from audio_recorder import AudioRecorder
# from engine import SpeechEngine # PyTorch version (Heavy)
from engine_onnx import SpeechEngine # ONNX version (Lightweight)
from text_injector import TextInjector
from ui_communicator import UICommunicator

# Configuration
HOTKEY = keyboard.Key.f4 

def play_sound(sound_type="start"):
    """
    Play feedback sound using sounddevice (numpy array).
    Non-blocking execution (fire and forget via sounddevice async or threaded).
    """
    try:
        volume = 0.5
        fs = 44100
        duration = 0.15 if sound_type == "start" else 0.1
        f = 880.0 if sound_type == "start" else 440.0

        # Generate samples
        t = np.arange(int(fs * duration)) / fs
        samples = (volume * np.sin(2 * np.pi * f * t)).astype(np.float32)

        if sound_type == "stop":
            # Quick pause then second beep for "stop" (double beep)
            silence = np.zeros(int(fs * 0.05), dtype=np.float32)
            samples = np.concatenate((samples, silence, samples))
            
        # sd.play is asynchronous by default, so we can just call it.
        # But if we call it rapidly, we might want to wait? 
        # For UI feedback, overlap is fine or we can wait.
        sd.play(samples, fs)
        sd.wait() # Wait for sound to finish so thread dies cleanly
    except Exception as e:
        print(f"Audio Feedback Error: {e}")

def main():
    print("Initializing FlashSpeech (SenseVoiceSmall ONNX) - V3.0 REBOOT")
    print("DEBUG: Typewriter Mode should be ACTIVE.")
    
    recorder = AudioRecorder()
    injector = TextInjector()
    ui_comms = UICommunicator() # Starts HTTP server on port 56789
    
    # Initialize engine last as it takes longest
    engine = SpeechEngine()
    
    print(f"\nFlashSpeech is ready!")
    print(f"Press and HOLD [{HOTKEY.name}] to record.")
    print(f"Release to transcribe and type.")
    print("Press Ctrl+C to exit.")

    # Queue for processing audio tasks
    task_queue = queue.Queue()
    
    def worker():
        while True:
            audio_data = task_queue.get()
            if audio_data is None:
                break
            
            try:
                print("Transcribing...")
                ui_comms.update("processing")
                start_time = time.time()
                text = engine.transcribe(audio_data)
                duration = time.time() - start_time
                
                if text:
                    print(f"Result: {text} (Latency: {duration:.3f}s)")
                    ui_comms.update("result", text)
                    injector.type_text(text)
                    # Reset UI after short delay
                    time.sleep(2)
                    ui_comms.update("idle")
                else:
                    print(f"No speech detected (Latency: {duration:.3f}s)")
                    ui_comms.update("idle")
            except Exception as e:
                print(f"Error processing audio: {e}")
                ui_comms.update("idle")
            finally:
                task_queue.task_done()

    # Start worker thread
    t = threading.Thread(target=worker, daemon=True)
    t.start()

    is_pressed = False

    def on_press(key):
        nonlocal is_pressed
        if key == HOTKEY:
            if not is_pressed:
                is_pressed = True
                print("\n[Start Recording]")
                # Play sound in a thread to non-block the listener
                threading.Thread(target=play_sound, args=("start",)).start()
                ui_comms.update("listening")
                recorder.start_recording()

    def on_release(key):
        nonlocal is_pressed
        if key == HOTKEY:
            if is_pressed:
                is_pressed = False
                print("[Stop Recording]")
                threading.Thread(target=play_sound, args=("stop",)).start()
                audio_data = recorder.stop_recording()
                
                if audio_data and len(audio_data) > 0:
                    # Put task in queue
                    task_queue.put(audio_data) 
                else:
                    print("No audio recorded.")
                    ui_comms.update("idle")

    # Collect events until released
    try:
        with keyboard.Listener(
                on_press=on_press,
                on_release=on_release) as listener:
            listener.join()
    except KeyboardInterrupt:
        print("\nExiting...")
        task_queue.put(None) # Signal worker to stop if needed
    finally:
        recorder.terminate()

if __name__ == "__main__":
    main()
