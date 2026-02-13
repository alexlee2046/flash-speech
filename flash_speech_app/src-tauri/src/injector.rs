use std::io::Write;
use std::process::Command;
use std::time::Duration;

#[cfg(target_os = "macos")]
mod cg_paste {
    use std::ffi::c_void;

    type CGEventRef = *mut c_void;
    type CGEventSourceRef = *mut c_void;

    const K_CG_HID_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_SOURCE_STATE_HID: u32 = 1;
    const K_CG_EVENT_FLAG_COMMAND: u64 = 1 << 20;
    const K_VK_V: u16 = 9;

    extern "C" {
        fn CGEventSourceCreate(stateID: u32) -> CGEventSourceRef;
        fn CGEventCreateKeyboardEvent(
            source: CGEventSourceRef,
            keycode: u16,
            keydown: bool,
        ) -> CGEventRef;
        fn CGEventSetFlags(event: CGEventRef, flags: u64);
        fn CGEventPost(tap: u32, event: CGEventRef);
        fn CFRelease(cf: *mut c_void);
    }

    pub fn simulate_cmd_v() {
        unsafe {
            let source = CGEventSourceCreate(K_CG_EVENT_SOURCE_STATE_HID);
            if source.is_null() {
                eprintln!("[injector] CGEventSourceCreate failed");
                return;
            }

            let key_down = CGEventCreateKeyboardEvent(source, K_VK_V, true);
            let key_up = CGEventCreateKeyboardEvent(source, K_VK_V, false);

            CGEventSetFlags(key_down, K_CG_EVENT_FLAG_COMMAND);
            CGEventSetFlags(key_up, K_CG_EVENT_FLAG_COMMAND);

            CGEventPost(K_CG_HID_EVENT_TAP, key_down);
            CGEventPost(K_CG_HID_EVENT_TAP, key_up);

            CFRelease(key_down);
            CFRelease(key_up);
            CFRelease(source);

            eprintln!("[injector] Paste keystroke sent via CGEvent (HID)");
        }
    }
}

#[cfg(target_os = "macos")]
use cg_paste::simulate_cmd_v;

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        Self
    }

    #[cfg(target_os = "macos")]
    pub fn inject(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        eprintln!("[injector] Injecting text: {}", text);

        // Save old clipboard
        let old_cb = Command::new("pbpaste")
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default();

        // Write text to clipboard
        let mut child = match Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[injector] pbcopy failed: {}", e);
                return;
            }
        };
        if let Some(ref mut stdin) = child.stdin {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();

        std::thread::sleep(Duration::from_millis(50));

        // Simulate Cmd+V via CoreGraphics CGEvent (HID level)
        simulate_cmd_v();

        std::thread::sleep(Duration::from_millis(250));

        // Restore old clipboard
        if let Ok(mut child) = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(&old_cb);
            }
            let _ = child.wait();
        }
    }

    #[cfg(target_os = "windows")]
    pub fn inject(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        eprintln!("[injector] Injecting text: {}", text);

        // Set clipboard via PowerShell
        let escaped = text.replace("'", "''");
        if let Err(e) = Command::new("powershell")
            .args(["-NoProfile", "-Command"])
            .arg(format!("Set-Clipboard -Value '{}'", escaped))
            .output()
        {
            eprintln!("[injector] Set-Clipboard failed: {}", e);
            return;
        }

        std::thread::sleep(Duration::from_millis(50));

        // Simulate Ctrl+V via PowerShell SendKeys
        match Command::new("powershell")
            .args(["-NoProfile", "-Command"])
            .arg("Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait('^v')")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!(
                        "[injector] SendKeys failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                } else {
                    eprintln!("[injector] Paste keystroke sent via SendKeys");
                }
            }
            Err(e) => eprintln!("[injector] PowerShell error: {}", e),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn inject(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        eprintln!("[injector] Injecting text: {}", text);

        // Set clipboard via xclip (or xsel as fallback)
        let clipboard_set = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .or_else(|_| {
                Command::new("xsel")
                    .args(["--clipboard", "--input"])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
            });

        match clipboard_set {
            Ok(mut child) => {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
            Err(e) => {
                eprintln!("[injector] Clipboard tool not found (xclip/xsel): {}", e);
                return;
            }
        }

        std::thread::sleep(Duration::from_millis(50));

        // Simulate Ctrl+V via xdotool
        match Command::new("xdotool")
            .args(["key", "ctrl+v"])
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!(
                        "[injector] xdotool failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                } else {
                    eprintln!("[injector] Paste keystroke sent via xdotool");
                }
            }
            Err(e) => eprintln!("[injector] xdotool error: {}", e),
        }
    }
}
