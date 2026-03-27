#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod audio;
mod injector;
mod model;
mod recognizer;
mod resample;
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

fn delayed_hide(app: AppHandle, delay_ms: u64) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        if let Some(win) = app.get_window("main") {
            let state = app.state::<AppState>();
            if !state.is_recording.load(Ordering::Acquire) {
                let _ = win.hide();
            }
        }
    });
}

fn do_toggle_recording(app: &AppHandle) {
    let state = app.state::<AppState>();

    if let Some(win) = app.get_window("main") {
        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::NSEvent;
            use cocoa::foundation::NSPoint;
            use cocoa::base::nil;
            let loc: NSPoint = unsafe { NSEvent::mouseLocation(nil) };
            if let Ok(Some(monitor)) = win.primary_monitor() {
                let scale = monitor.scale_factor();
                let sh = monitor.size().height as f64 / scale;
                let wx = loc.x - 100.0;
                let wy = sh - loc.y + 20.0; 
                let _ = win.set_position(tauri::Position::Logical(
                    tauri::LogicalPosition::new(wx, wy)
                ));
            }
        }
        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::NSWindow;
            use cocoa::base::id;
            let ns_window = win.ns_window().unwrap() as id;
            unsafe {
                ns_window.orderFrontRegardless();
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = win.show();
        }
    }

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

    let was_recording = state.is_recording.load(Ordering::Acquire);

    if was_recording {
        // Stop recording → process audio
        state.is_recording.store(false, Ordering::Release);
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
                    let original_len = samples.len();
                    let samples_16k = resample::resample(&samples, sample_rate, 16000);
                    eprintln!(
                        "[audio] Resampled: {} → {} samples ({}Hz → 16000Hz)",
                        original_len,
                        samples_16k.len(),
                        sample_rate
                    );

                    // Transcribe (take/put to minimize mutex hold time)
                    let text = {
                        let mut recognizer = {
                            let mut guard = state.recognizer.lock().unwrap();
                            match guard.take() {
                                Some(r) => r,
                                None => {
                                    eprintln!("[recognizer] Not initialized yet");
                                    emit_state(&app_handle, "idle", None);
                                    delayed_hide(app_handle.clone(), 200);
                                    return;
                                }
                            }
                        }; // mutex released here

                        let start = std::time::Instant::now();
                        let result = recognizer.transcribe(16000, &samples_16k);
                        eprintln!("[recognizer] Transcription took {:?}", start.elapsed());

                        // Put the recognizer back
                        *state.recognizer.lock().unwrap() = Some(recognizer);
                        result
                    };

                    match text {
                        Some(text) if !text.is_empty() => {
                            eprintln!("[result] {}", text);

                            // Inject text BEFORE updating HUD to avoid focus steal
                            state.injector.inject(&text);

                            let display_time = (text.len() as f64 * 0.08).max(2.0);
                            emit_state(&app_handle, "result", Some(text));

                            // Auto-reset to idle after display time
                            let app_for_timer = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_secs_f64(
                                    display_time,
                                )).await;
                                emit_state(&app_for_timer, "idle", None);
                                delayed_hide(app_for_timer, 200);
                            });
                        }
                        _ => {
                            eprintln!("[recognizer] No speech detected");
                            emit_state(&app_handle, "idle", None);
                            delayed_hide(app_handle, 200);
                        }
                    }
                }
                None => {
                    eprintln!("[audio] No valid audio (too short or empty)");
                    emit_state(&app_handle, "idle", None);
                    delayed_hide(app_handle, 200);
                }
            }
        });
    } else {
        // Start recording
        state.is_recording.store(true, Ordering::Release);
        eprintln!("[shortcut] toggle → start_record");
        sound::play_start_sound();
        emit_state(app, "listening", None);

        if let Err(e) = state.recorder.start() {
            eprintln!("[audio] Failed to start: {}", e);
            state.is_recording.store(false, Ordering::Release);
            emit_state(app, "error", None);
            // Auto-reset error after 3s
            let app_c = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                emit_state(&app_c, "idle", None);
                delayed_hide(app_c, 200);
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
        std::thread::sleep(std::time::Duration::from_millis(300));
        app_clone.exit(0);
    });
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    do_quit(&app);
}

/// Initialize the backend: download model if needed, then load the recognizer.
fn initialize_backend(app_handle: AppHandle, model_name: String) {
    std::thread::spawn(move || {
        emit_state(&app_handle, "starting", None);

        // Disconnect old recognizer first to free memory
        {
            let state = app_handle.state::<AppState>();
            *state.recognizer.lock().unwrap() = None;
        }

        if model_name == "whisper" {
            if let Err(e) = model::ensure_whisper(&app_handle) {
                eprintln!("[init] Whisper download failed: {}", e);
                emit_state(&app_handle, "error", Some(format!("模型下载失败: {}", e)));
                return;
            }

            let enc = model::whisper_encoder_path(&app_handle).unwrap();
            let dec = model::whisper_decoder_path(&app_handle).unwrap();
            let tok = model::whisper_tokens_path(&app_handle).unwrap();

            eprintln!("[init] Loading Whisper model...");
            match SpeechRecognizer::new_whisper(
                enc.to_str().unwrap(),
                dec.to_str().unwrap(),
                tok.to_str().unwrap()
            ) {
                Ok(rec) => {
                    let state = app_handle.state::<AppState>();
                    *state.recognizer.lock().unwrap() = Some(rec);
                    eprintln!("[init] Whisper loaded successfully");
                    emit_state(&app_handle, "idle", None);
                    delayed_hide(app_handle.clone(), 2000);
                }
                Err(e) => {
                    eprintln!("[init] Failed to load Whisper: {}", e);
                    emit_state(&app_handle, "error", Some(format!("模型加载失败: {}", e)));
                }
            }
        } else if model_name == "paraformer" {
            if let Err(e) = model::ensure_paraformer(&app_handle) {
                eprintln!("[init] Paraformer download failed: {}", e);
                emit_state(&app_handle, "error", Some(format!("模型下载失败: {}", e)));
                return;
            }

            let p_model = model::paraformer_model_path(&app_handle).unwrap();
            let p_tokens = model::paraformer_tokens_path(&app_handle).unwrap();

            eprintln!("[init] Loading Paraformer model...");
            match SpeechRecognizer::new_paraformer(
                p_model.to_str().unwrap(),
                p_tokens.to_str().unwrap(),
            ) {
                Ok(rec) => {
                    let state = app_handle.state::<AppState>();
                    *state.recognizer.lock().unwrap() = Some(rec);
                    eprintln!("[init] Paraformer loaded successfully");
                    emit_state(&app_handle, "idle", None);
                    delayed_hide(app_handle.clone(), 2000);
                }
                Err(e) => {
                    eprintln!("[init] Failed to load Paraformer: {}", e);
                    emit_state(&app_handle, "error", Some(format!("模型加载失败: {}", e)));
                }
            }
        } else {
            // Default to SenseVoice
            if let Err(e) = model::ensure_sensevoice(&app_handle) {
                eprintln!("[init] SenseVoice download failed: {}", e);
                emit_state(&app_handle, "error", Some(format!("模型下载失败: {}", e)));
                return;
            }

            let model_path = model::sensevoice_model_path(&app_handle).unwrap();
            let tokens_path = model::sensevoice_tokens_path(&app_handle).unwrap();

            eprintln!("[init] Loading SenseVoice model...");
            let load_start = std::time::Instant::now();
            match SpeechRecognizer::new_sensevoice(
                model_path.to_str().unwrap(),
                tokens_path.to_str().unwrap(),
            ) {
                Ok(rec) => {
                    let state = app_handle.state::<AppState>();
                    *state.recognizer.lock().unwrap() = Some(rec);
                    eprintln!("[init] SenseVoice loaded successfully in {:?}", load_start.elapsed());
                    emit_state(&app_handle, "idle", None);
                    delayed_hide(app_handle.clone(), 2000);
                }
                Err(e) => {
                    eprintln!("[init] Failed to load SenseVoice: {}", e);
                    emit_state(&app_handle, "error", Some(format!("模型加载失败: {}", e)));
                }
            }
        }

        // Display macOS accessibility warning
        #[cfg(target_os = "macos")]
        {
            let state = app_handle.state::<AppState>();
            let trusted = state.injector.check_accessibility();
            if !trusted {
                eprintln!("[init] WARNING: Accessibility permission not granted. Text injection will use osascript fallback.");
            }
        }
    });
}

#[tauri::command]
fn switch_model(name: String, app: AppHandle) -> Result<(), String> {
    eprintln!("[command] switch_model payload: {}", name);
    let state = app.state::<AppState>();
    
    // Stop recording before switching models to free resources
    let was_recording = state.is_recording.load(Ordering::Acquire);
    if was_recording {
        state.is_recording.store(false, Ordering::Release);
        let _ = state.recorder.stop();
        sound::play_stop_sound();
    }
    
    initialize_backend(app, name);
    Ok(())
}

fn main() {
    // Redirect stderr to log file for GUI launch debugging
    {
        use std::fs::OpenOptions;
        use std::os::unix::io::IntoRawFd;
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/flashspeech.log")
        {
            let fd = file.into_raw_fd();
            unsafe { libc::dup2(fd, 2); }
        }
    }
    eprintln!("\n=== FlashSpeech started at {:?} ===", std::time::SystemTime::now());

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
        .invoke_handler(tauri::generate_handler![toggle_recording, quit_app, switch_model])
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

            // Initialize persistent sound system
            sound::init();

            // Initialize backend (model download + recognizer loading)
            initialize_backend(app.handle(), "sensevoice".to_string());

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
                    use objc::{msg_send, sel, sel_impl};
                    ns_window.setHasShadow_(NO);
                    ns_window.setOpaque_(NO);
                    ns_window.setBackgroundColor_(NSColor::clearColor(nil));
                    // Floating panel level (above normal windows)
                    let _: () = msg_send![ns_window, setLevel: 3i64];
                    // Join all spaces, stationary, fullscreen auxiliary
                    let collection_behavior: u64 = (1 << 0) | (1 << 4) | (1 << 6);
                    let _: () = msg_send![ns_window, setCollectionBehavior: collection_behavior];
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
