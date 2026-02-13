#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod audio;
mod injector;
mod model;
mod recognizer;
mod sound;

use audio::AudioRecorder;
use injector::TextInjector;
use recognizer::SpeechRecognizer;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{
    AppHandle, CustomMenuItem, GlobalShortcutManager, Manager, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem,
};

#[derive(Clone, serde::Serialize)]
struct StatePayload {
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
}

struct AppState {
    recognizer: Mutex<Option<SpeechRecognizer>>,
    recorder: AudioRecorder,
    injector: TextInjector,
    is_recording: AtomicBool,
    last_toggle: Mutex<std::time::Instant>,
}

fn emit_state(app: &AppHandle, state: &str, text: Option<String>) {
    let _ = app.emit_all(
        "state-change",
        StatePayload {
            state: state.to_string(),
            text,
        },
    );
}

/// Linear interpolation resampling (sufficient quality for speech recognition)
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    let ratio = to_rate as f64 / from_rate as f64;
    let output_len = (samples.len() as f64 * ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);
    for i in 0..output_len {
        let src_idx = i as f64 / ratio;
        let idx_floor = src_idx.floor() as usize;
        let frac = (src_idx - idx_floor as f64) as f32;
        let sample = if idx_floor + 1 < samples.len() {
            samples[idx_floor] * (1.0 - frac) + samples[idx_floor + 1] * frac
        } else if idx_floor < samples.len() {
            samples[idx_floor]
        } else {
            0.0
        };
        output.push(sample);
    }
    output
}

fn do_toggle_recording(app: &AppHandle) {
    let state = app.state::<AppState>();

    // Debounce: ignore if < 600ms since last toggle
    {
        let mut last = state.last_toggle.lock().unwrap();
        let now = std::time::Instant::now();
        if now.duration_since(*last).as_millis() < 600 {
            eprintln!(
                "[shortcut] debounced ({}ms)",
                now.duration_since(*last).as_millis()
            );
            return;
        }
        *last = now;
    }

    let was_recording = state.is_recording.load(Ordering::SeqCst);

    if was_recording {
        // Stop recording → process audio
        state.is_recording.store(false, Ordering::SeqCst);
        eprintln!("[shortcut] toggle → stop_record");
        sound::play_stop_sound();

        let app_handle = app.clone();
        std::thread::spawn(move || {
            let state = app_handle.state::<AppState>();
            let audio = state.recorder.stop();

            match audio {
                Some((samples, sample_rate)) => {
                    emit_state(&app_handle, "processing", None);

                    // Resample to 16kHz if needed
                    let samples_16k = resample(&samples, sample_rate, 16000);
                    eprintln!(
                        "[audio] Resampled: {} → {} samples ({}Hz → 16000Hz)",
                        samples.len(),
                        samples_16k.len(),
                        sample_rate
                    );

                    // Transcribe
                    let text = {
                        let mut rec = state.recognizer.lock().unwrap();
                        if let Some(ref mut recognizer) = *rec {
                            let start = std::time::Instant::now();
                            let result = recognizer.transcribe(16000, &samples_16k);
                            eprintln!("[recognizer] Transcription took {:?}", start.elapsed());
                            result
                        } else {
                            eprintln!("[recognizer] Not initialized yet");
                            None
                        }
                    };

                    match text {
                        Some(text) if !text.is_empty() => {
                            eprintln!("[result] {}", text);

                            // Inject text BEFORE updating HUD to avoid focus steal
                            state.injector.inject(&text);

                            emit_state(&app_handle, "result", Some(text.clone()));

                            // Auto-reset to idle after display time
                            let display_time = (text.len() as f64 * 0.08).max(2.0);
                            let app_for_timer = app_handle.clone();
                            std::thread::spawn(move || {
                                std::thread::sleep(std::time::Duration::from_secs_f64(
                                    display_time,
                                ));
                                emit_state(&app_for_timer, "idle", None);
                            });
                        }
                        _ => {
                            eprintln!("[recognizer] No speech detected");
                            emit_state(&app_handle, "idle", None);
                        }
                    }
                }
                None => {
                    eprintln!("[audio] No valid audio (too short or empty)");
                    emit_state(&app_handle, "idle", None);
                }
            }
        });
    } else {
        // Start recording
        state.is_recording.store(true, Ordering::SeqCst);
        eprintln!("[shortcut] toggle → start_record");
        sound::play_start_sound();
        emit_state(app, "listening", None);

        if let Err(e) = state.recorder.start() {
            eprintln!("[audio] Failed to start: {}", e);
            state.is_recording.store(false, Ordering::SeqCst);
            emit_state(app, "error", None);
            // Auto-reset error after 3s
            let app_c = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                emit_state(&app_c, "idle", None);
            });
        }
    }
}

#[tauri::command]
fn toggle_recording(app: AppHandle) {
    do_toggle_recording(&app);
}

fn do_quit(app: &AppHandle) {
    emit_state(app, "exiting", None);
    let app_clone = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        app_clone.exit(0);
    });
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    do_quit(&app);
}

/// Initialize the backend: download model if needed, then load the recognizer.
fn initialize_backend(app_handle: AppHandle) {
    std::thread::spawn(move || {
        emit_state(&app_handle, "starting", None);

        // Ensure model is downloaded
        if let Err(e) = model::ensure_model(&app_handle) {
            eprintln!("[init] Model download failed: {}", e);
            emit_state(
                &app_handle,
                "error",
                Some(format!("模型下载失败: {}", e)),
            );
            return;
        }

        // Get model paths
        let model_path = match model::model_path(&app_handle) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[init] Model path error: {}", e);
                emit_state(&app_handle, "error", Some(e));
                return;
            }
        };
        let tokens_path = match model::tokens_path(&app_handle) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[init] Tokens path error: {}", e);
                emit_state(&app_handle, "error", Some(e));
                return;
            }
        };

        // Load SenseVoice recognizer
        eprintln!("[init] Loading SenseVoice model...");
        match SpeechRecognizer::new(
            model_path.to_str().unwrap(),
            tokens_path.to_str().unwrap(),
        ) {
            Ok(rec) => {
                let state = app_handle.state::<AppState>();
                *state.recognizer.lock().unwrap() = Some(rec);
                eprintln!("[init] SenseVoice model loaded successfully");

                // Check accessibility permission for text injection
                #[cfg(target_os = "macos")]
                {
                    let trusted = state.injector.check_accessibility();
                    if !trusted {
                        eprintln!("[init] WARNING: Accessibility permission not granted. Text injection will use osascript fallback.");
                    }
                }

                emit_state(&app_handle, "idle", None);
            }
            Err(e) => {
                eprintln!("[init] Failed to load model: {}", e);
                emit_state(
                    &app_handle,
                    "error",
                    Some(format!("模型加载失败: {}", e)),
                );
            }
        }
    });
}

fn main() {
    let quit = CustomMenuItem::new("quit".to_string(), "Quit FlashSpeech");
    let toggle = CustomMenuItem::new("toggle".to_string(), "Show/Hide HUD");

    let tray_menu = SystemTrayMenu::new()
        .add_item(toggle)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    let system_tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .manage(AppState {
            recognizer: Mutex::new(None),
            recorder: AudioRecorder::new(),
            injector: TextInjector::new(),
            is_recording: AtomicBool::new(false),
            last_toggle: Mutex::new(std::time::Instant::now()),
        })
        .invoke_handler(tauri::generate_handler![toggle_recording, quit_app])
        .system_tray(system_tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::LeftClick { .. } => {
                let window = app.get_window("main").unwrap();
                if window.is_visible().unwrap() {
                    window.hide().unwrap();
                } else {
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    do_quit(app);
                }
                "toggle" => {
                    let window = app.get_window("main").unwrap();
                    if window.is_visible().unwrap() {
                        window.hide().unwrap();
                    } else {
                        window.show().unwrap();
                        window.set_focus().unwrap();
                    }
                }
                _ => {}
            },
            _ => {}
        })
        .on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event.event() {
                event.window().hide().unwrap();
                api.prevent_close();
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Initialize backend (model download + recognizer loading)
            initialize_backend(app.handle());

            // Register global shortcut: Alt+Space to toggle recording
            let handle = app.handle();
            app.global_shortcut_manager()
                .register("Alt+Space", move || {
                    do_toggle_recording(&handle);
                })
                .expect("failed to register global shortcut");

            // Configure transparent window
            let window = app.get_window("main").unwrap();
            #[cfg(target_os = "macos")]
            {
                use cocoa::appkit::{NSColor, NSWindow};
                use cocoa::base::{id, nil, NO};

                let ns_window = window.ns_window().unwrap() as id;
                unsafe {
                    ns_window.setHasShadow_(NO);
                    ns_window.setOpaque_(NO);
                    ns_window.setBackgroundColor_(NSColor::clearColor(nil));
                }

                window
                    .with_webview(|webview| {
                        use cocoa::base::{id, nil, NO};
                        use cocoa::foundation::NSString;
                        use objc::{class, msg_send, sel, sel_impl};
                        unsafe {
                            let wk: id = webview.inner();
                            let no: id = msg_send![class!(NSNumber), numberWithBool: NO];
                            let key = NSString::alloc(nil).init_str("drawsBackground");
                            let _: () = msg_send![wk, setValue: no forKey: key];
                        }
                    })
                    .expect("failed to configure webview transparency");
            }

            // Position window at bottom-center of screen
            if let Ok(Some(monitor)) = window.current_monitor() {
                let size = monitor.size();
                let scale = monitor.scale_factor();
                let sw = size.width as f64 / scale;
                let sh = size.height as f64 / scale;
                let x = (sw - 176.0) / 2.0;
                let y = sh - 64.0 - 80.0;
                let _ = window.set_position(tauri::Position::Logical(
                    tauri::LogicalPosition::new(x, y),
                ));
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
