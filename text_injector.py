from pynput.keyboard import Controller, Key
import time
import pyperclip
import platform
import random

class TextInjector:
    def __init__(self):
        self.keyboard = Controller()
        self.os_name = platform.system()

    def type_text(self, text):
        if not text:
            return
        
        print(f"Injecting text (Typewriter Mode): {text}")
        
        # Tech Mode: Instant Paste (Quantum Snap)
        # 1. Save old clipboard
        old_cb = pyperclip.paste()
        # 2. Copy new text
        pyperclip.copy(text)
        # 3. Trigger Paste (Cmd+V)
        # Use OS-specific keys
        if self.os_name == "Darwin":
            with self.keyboard.pressed(Key.cmd):
                self.keyboard.press('v')
                self.keyboard.release('v')
        else:
            with self.keyboard.pressed(Key.ctrl):
                self.keyboard.press('v')
                self.keyboard.release('v')
        
        # 4. Restore clipboard (async or delayed slightly to ensure paste happens)
        time.sleep(0.1) 
        pyperclip.copy(old_cb)
