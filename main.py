import time
import threading
import queue
import numpy as np
import sounddevice as sd
from pynput import keyboard
from audio_recorder import AudioRecorder
from engine_onnx import SpeechEngine
from text_injector import TextInjector
from ui_communicator import UICommunicator

# Configuration
HOTKEY = keyboard.Key.f4

# 预生成音频反馈数据，避免每次重新计算
_FS = 44100
_VOLUME = 0.5


def _generate_tone(freq, duration):
    t = np.arange(int(_FS * duration)) / _FS
    return (_VOLUME * np.sin(2 * np.pi * freq * t)).astype(np.float32)


# 开始录音：单声高音
_START_SOUND = _generate_tone(880.0, 0.15)
# 停止录音：双声低音
_stop_tone = _generate_tone(440.0, 0.1)
_stop_silence = np.zeros(int(_FS * 0.05), dtype=np.float32)
_STOP_SOUND = np.concatenate((_stop_tone, _stop_silence, _stop_tone))


def play_sound(sound_type="start"):
    """播放音频反馈（使用预缓存数据）"""
    try:
        samples = _START_SOUND if sound_type == "start" else _STOP_SOUND
        sd.play(samples, _FS)
        sd.wait()
    except Exception as e:
        print(f"Audio Feedback Error: {e}")


def main():
    print("Initializing FlashSpeech (SenseVoiceSmall ONNX) - V4.0")

    exit_event = threading.Event()

    def on_quit():
        print("\n[Graceful Exit Triggered]")
        if not exit_event.is_set():
            ui_comms.update("exiting")
            threading.Thread(target=play_sound, args=("stop",), daemon=True).start()
            time.sleep(1.5)
            exit_event.set()
        return False

    recorder = AudioRecorder()
    injector = TextInjector()
    ui_comms = UICommunicator(on_exit=on_quit)
    engine = SpeechEngine()

    print(f"\nFlashSpeech is ready!")
    print(f"Press and HOLD [{HOTKEY.name}] to record.")
    print(f"Release to transcribe and type.")
    print(f"Press <Cmd+Option+Q> to quit safely.")

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

                    # 根据文本长度动态调整结果显示时间
                    display_time = max(2.0, len(text) * 0.08)

                    def reset_ui():
                        if ui_comms.get_state() == "result":
                            ui_comms.update("idle")

                    threading.Timer(display_time, reset_ui).start()
                else:
                    print(f"No speech detected (Latency: {duration:.3f}s)")
                    ui_comms.update("idle")
            except Exception as e:
                print(f"Error processing audio: {e}")
                ui_comms.update("error", str(e))
                # 错误状态显示 3 秒后恢复
                def reset_error():
                    if ui_comms.get_state() == "error":
                        ui_comms.update("idle")
                threading.Timer(3.0, reset_error).start()
            finally:
                task_queue.task_done()

    t = threading.Thread(target=worker, daemon=True)
    t.start()

    is_pressed = False

    exit_hotkey = keyboard.GlobalHotKeys({
        '<cmd>+<alt>+q': on_quit
    })
    exit_hotkey.start()

    def on_press(key):
        nonlocal is_pressed
        if exit_event.is_set():
            return False

        if key == HOTKEY:
            if not is_pressed:
                is_pressed = True
                print("\n[Start Recording]")
                threading.Thread(target=play_sound, args=("start",), daemon=True).start()
                ui_comms.update("listening")
                recorder.start_recording()

    def on_release(key):
        nonlocal is_pressed
        if exit_event.is_set():
            return False

        if key == HOTKEY:
            if is_pressed:
                is_pressed = False
                print("[Stop Recording]")
                threading.Thread(target=play_sound, args=("stop",), daemon=True).start()
                audio_data = recorder.stop_recording()

                if audio_data and len(audio_data) > 0:
                    task_queue.put(audio_data)
                else:
                    print("No valid audio recorded (too short or empty).")
                    ui_comms.update("idle")

    try:
        with keyboard.Listener(
                on_press=on_press,
                on_release=on_release) as listener:
            while not exit_event.is_set() and listener.running:
                time.sleep(0.1)
    except KeyboardInterrupt:
        print("\nExiting...")
    finally:
        task_queue.put(None)
        recorder.terminate()
        exit_hotkey.stop()
        print("FlashSpeech shutdown complete.")


if __name__ == "__main__":
    main()
