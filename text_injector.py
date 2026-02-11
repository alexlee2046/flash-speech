from pynput.keyboard import Controller, Key
import time
import pyperclip
import platform


class TextInjector:
    def __init__(self):
        self.keyboard = Controller()
        self.os_name = platform.system()

    def type_text(self, text):
        if not text:
            return

        print(f"Injecting text: {text}")

        try:
            old_cb = pyperclip.paste()
        except Exception:
            old_cb = ""

        pyperclip.copy(text)
        # 等待剪贴板写入完成
        time.sleep(0.05)

        paste_key = Key.cmd if self.os_name == "Darwin" else Key.ctrl
        with self.keyboard.pressed(paste_key):
            self.keyboard.press('v')
            self.keyboard.release('v')

        # 等待粘贴操作完成后再恢复剪贴板
        time.sleep(0.25)

        try:
            pyperclip.copy(old_cb)
        except Exception:
            pass
