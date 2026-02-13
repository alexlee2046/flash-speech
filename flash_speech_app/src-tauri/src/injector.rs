use std::io::Write;
use std::process::Command;
use std::time::Duration;

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

        // Simulate Cmd+V via osascript
        match Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "System Events" to keystroke "v" using command down"#)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!(
                        "[injector] osascript failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                } else {
                    eprintln!("[injector] Paste keystroke sent via osascript");
                }
            }
            Err(e) => eprintln!("[injector] osascript error: {}", e),
        }

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
