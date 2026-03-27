use std::io::Write;
use std::process::Command;
use std::time::Duration;

#[cfg(target_os = "macos")]
mod macos_paste {
    use std::ffi::c_void;
    use std::process::Command;

    type CGEventRef = *mut c_void;
    type CGEventSourceRef = *mut c_void;

    const K_CG_HID_EVENT_TAP: u32 = 0;
    const K_CG_EVENT_SOURCE_STATE_HID: u32 = 1;
    const K_CG_EVENT_FLAG_COMMAND: u64 = 1 << 20;
    const K_VK_V: u16 = 9;
    // Max characters per CGEvent Unicode string
    const UNICODE_CHUNK_SIZE: usize = 200;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceCreate(stateID: u32) -> CGEventSourceRef;
        fn CGEventCreateKeyboardEvent(
            source: CGEventSourceRef,
            keycode: u16,
            keydown: bool,
        ) -> CGEventRef;
        fn CGEventSetFlags(event: CGEventRef, flags: u64);
        fn CGEventPost(tap: u32, event: CGEventRef);
        fn CGEventKeyboardSetUnicodeString(
            event: CGEventRef,
            string_length: u32,
            unicode_string: *const u16,
        );
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *mut c_void);
    }

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }

    /// Check if the process has accessibility (AX) trust.
    pub fn is_accessibility_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    /// Inject text by typing it directly via CGEvent Unicode string.
    /// No clipboard needed. Sends text as keyboard input in chunks.
    pub fn cg_event_type_text(text: &str) -> bool {
        let utf16: Vec<u16> = text.encode_utf16().collect();
        if utf16.is_empty() {
            return true;
        }

        unsafe {
            let source = CGEventSourceCreate(K_CG_EVENT_SOURCE_STATE_HID);
            if source.is_null() {
                eprintln!("[injector] CGEventSourceCreate failed");
                return false;
            }

            for chunk in utf16.chunks(UNICODE_CHUNK_SIZE) {
                let key_down = CGEventCreateKeyboardEvent(source, 0, true);
                let key_up = CGEventCreateKeyboardEvent(source, 0, false);

                if key_down.is_null() || key_up.is_null() {
                    if !key_down.is_null() { CFRelease(key_down); }
                    if !key_up.is_null() { CFRelease(key_up); }
                    CFRelease(source);
                    eprintln!("[injector] CGEventCreateKeyboardEvent failed");
                    return false;
                }

                CGEventKeyboardSetUnicodeString(
                    key_down,
                    chunk.len() as u32,
                    chunk.as_ptr(),
                );

                CGEventPost(K_CG_HID_EVENT_TAP, key_down);
                CGEventPost(K_CG_HID_EVENT_TAP, key_up);

                CFRelease(key_down);
                CFRelease(key_up);

                // Small delay between chunks to avoid input buffer overflow
                if utf16.len() > UNICODE_CHUNK_SIZE {
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }

            CFRelease(source);
            eprintln!("[injector] Text typed via CGEventKeyboardSetUnicodeString ({} chars)", utf16.len());
            true
        }
    }

    /// Simulate Cmd+V via CGEventPost to HID event tap (broadcast).
    pub fn cg_event_paste_hid() -> bool {
        unsafe {
            let source = CGEventSourceCreate(K_CG_EVENT_SOURCE_STATE_HID);
            if source.is_null() {
                eprintln!("[injector] CGEventSourceCreate failed");
                return false;
            }

            let key_down = CGEventCreateKeyboardEvent(source, K_VK_V, true);
            let key_up = CGEventCreateKeyboardEvent(source, K_VK_V, false);

            if key_down.is_null() || key_up.is_null() {
                eprintln!("[injector] CGEventCreateKeyboardEvent failed");
                if !key_down.is_null() { CFRelease(key_down); }
                if !key_up.is_null() { CFRelease(key_up); }
                CFRelease(source);
                return false;
            }

            CGEventSetFlags(key_down, K_CG_EVENT_FLAG_COMMAND);
            CGEventSetFlags(key_up, K_CG_EVENT_FLAG_COMMAND);

            CGEventPost(K_CG_HID_EVENT_TAP, key_down);
            std::thread::sleep(std::time::Duration::from_millis(20));
            CGEventPost(K_CG_HID_EVENT_TAP, key_up);

            CFRelease(key_down);
            CFRelease(key_up);
            CFRelease(source);

            eprintln!("[injector] Paste sent via CGEventPost (HID broadcast)");
            true
        }
    }

    /// Simulate Cmd+V via System Events (osascript). Fallback method.
    pub fn osascript_paste() -> bool {
        let result = Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events"
    set frontApp to first application process whose frontmost is true
    tell frontApp to keystroke "v" using command down
end tell"#,
            ])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                eprintln!("[injector] Paste sent via osascript");
                true
            }
            Ok(output) => {
                eprintln!(
                    "[injector] osascript failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                false
            }
            Err(e) => {
                eprintln!("[injector] osascript error: {}", e);
                false
            }
        }
    }
}

pub struct TextInjector;

impl TextInjector {
    pub fn new() -> Self {
        Self
    }

    /// Check if accessibility permission is granted (macOS only).
    #[cfg(target_os = "macos")]
    pub fn check_accessibility(&self) -> bool {
        let trusted = macos_paste::is_accessibility_trusted();
        eprintln!(
            "[injector] Accessibility trusted: {}",
            if trusted { "YES" } else { "NO" }
        );
        trusted
    }

    #[cfg(target_os = "macos")]
    pub fn inject(&self, text: &str) {
        if text.is_empty() {
            return;
        }
        eprintln!("[injector] Injecting text: {}", text);

        // Method 1: Direct Unicode typing via CGEvent (no clipboard needed)
        if macos_paste::cg_event_type_text(text) {
            return;
        }

        // Method 2: Clipboard + Cmd+V (fallback)
        eprintln!("[injector] Unicode typing failed, falling back to clipboard+paste");

        let old_cb = Command::new("pbpaste")
            .output()
            .map(|o| o.stdout)
            .unwrap_or_default();

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
        std::thread::sleep(Duration::from_millis(80));

        let pasted = if macos_paste::is_accessibility_trusted() {
            macos_paste::cg_event_paste_hid()
        } else {
            false
        };
        if !pasted && !macos_paste::osascript_paste() {
            eprintln!("[injector] All paste methods failed");
        }

        std::thread::sleep(Duration::from_millis(300));

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
