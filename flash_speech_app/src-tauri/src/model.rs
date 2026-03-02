use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const SENSEVOICE_MODEL_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/model.int8.onnx";
const SENSEVOICE_TOKENS_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt";

const WHISPER_ENCODER_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny.en/resolve/main/tiny.en-encoder.int8.onnx";
const WHISPER_DECODER_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny.en/resolve/main/tiny.en-decoder.int8.onnx";
const WHISPER_TOKENS_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny.en/resolve/main/tiny.en-tokens.txt";

const PARAFORMER_MODEL_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2023-09-14/resolve/main/model.int8.onnx";
const PARAFORMER_TOKENS_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-paraformer-zh-2023-09-14/resolve/main/tokens.txt";

#[derive(Clone, serde::Serialize)]
struct DownloadProgress {
    file: String,
    downloaded: u64,
    total: u64,
}

pub fn model_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path_resolver()
        .app_data_dir()
        .ok_or("Cannot resolve app data directory")?;
    Ok(data_dir.join("models"))
}

// ── SenseVoice Paths ──
pub fn sensevoice_model_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("model.int8.onnx"))
}

pub fn sensevoice_tokens_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("tokens.txt"))
}

pub fn is_sensevoice_ready(app: &AppHandle) -> bool {
    sensevoice_model_path(app).map(|p| p.exists()).unwrap_or(false)
        && sensevoice_tokens_path(app).map(|p| p.exists()).unwrap_or(false)
}

pub fn ensure_sensevoice(app: &AppHandle) -> Result<(), String> {
    if is_sensevoice_ready(app) {
        eprintln!("[model] SenseVoice already downloaded");
        return Ok(());
    }

    if try_copy_bundled_models(app, "sensevoice") {
        eprintln!("[model] Copied bundled SenseVoice models");
        return Ok(());
    }

    let dir = model_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create model dir: {}", e))?;

    download_file(SENSEVOICE_MODEL_URL, &dir.join("model.int8.onnx"), "model.int8.onnx", app)?;
    download_file(SENSEVOICE_TOKENS_URL, &dir.join("tokens.txt"), "tokens.txt", app)?;

    Ok(())
}

// ── Whisper Paths ──
pub fn whisper_encoder_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("whisper").join("tiny.en-encoder.int8.onnx"))
}

pub fn whisper_decoder_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("whisper").join("tiny.en-decoder.int8.onnx"))
}

pub fn whisper_tokens_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("whisper").join("tiny.en-tokens.txt"))
}

pub fn is_whisper_ready(app: &AppHandle) -> bool {
    whisper_encoder_path(app).map(|p| p.exists()).unwrap_or(false)
        && whisper_decoder_path(app).map(|p| p.exists()).unwrap_or(false)
        && whisper_tokens_path(app).map(|p| p.exists()).unwrap_or(false)
}

pub fn ensure_whisper(app: &AppHandle) -> Result<(), String> {
    if is_whisper_ready(app) {
        return Ok(());
    }

    if try_copy_bundled_models(app, "whisper") {
        return Ok(());
    }

    let dir = model_dir(app)?.join("whisper");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create dir: {}", e))?;

    download_file(WHISPER_ENCODER_URL, &dir.join("tiny.en-encoder.int8.onnx"), "tiny.en-encoder.int8.onnx", app)?;
    download_file(WHISPER_DECODER_URL, &dir.join("tiny.en-decoder.int8.onnx"), "tiny.en-decoder.int8.onnx", app)?;
    download_file(WHISPER_TOKENS_URL, &dir.join("tiny.en-tokens.txt"), "tiny.en-tokens.txt", app)?;

    Ok(())
}

// ── Paraformer Paths ──
pub fn paraformer_model_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("paraformer").join("model.int8.onnx"))
}

pub fn paraformer_tokens_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("paraformer").join("tokens.txt"))
}

pub fn is_paraformer_ready(app: &AppHandle) -> bool {
    paraformer_model_path(app).map(|p| p.exists()).unwrap_or(false)
        && paraformer_tokens_path(app).map(|p| p.exists()).unwrap_or(false)
}

pub fn ensure_paraformer(app: &AppHandle) -> Result<(), String> {
    if is_paraformer_ready(app) {
        return Ok(());
    }

    if try_copy_bundled_models(app, "paraformer") {
        return Ok(());
    }

    let dir = model_dir(app)?.join("paraformer");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create dir: {}", e))?;

    download_file(PARAFORMER_MODEL_URL, &dir.join("model.int8.onnx"), "paraformer-model.int8.onnx", app)?;
    download_file(PARAFORMER_TOKENS_URL, &dir.join("tokens.txt"), "paraformer-tokens.txt", app)?;

    Ok(())
}

fn try_copy_bundled_models(app: &AppHandle, model_type: &str) -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let resources = exe
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("Resources/models"));
    let resources = match resources {
        Some(p) if p.exists() => p,
        _ => return false,
    };

    let dir = match model_dir(app) {
        Ok(d) => d,
        Err(_) => return false,
    };

    if model_type == "sensevoice" {
        let model_src = resources.join("model.int8.onnx");
        let tokens_src = resources.join("tokens.txt");
        if !model_src.exists() || !tokens_src.exists() {
            return false;
        }
        if std::fs::create_dir_all(&dir).is_err() {
            return false;
        }
        std::fs::copy(&model_src, dir.join("model.int8.onnx")).is_ok()
            && std::fs::copy(&tokens_src, dir.join("tokens.txt")).is_ok()
    } else if model_type == "whisper" {
        // whisper bundled checking
        let w_dir = dir.join("whisper");
        let e_src = resources.join("whisper").join("tiny.en-encoder.int8.onnx");
        let d_src = resources.join("whisper").join("tiny.en-decoder.int8.onnx");
        let t_src = resources.join("whisper").join("tiny.en-tokens.txt");
        if !e_src.exists() || !d_src.exists() || !t_src.exists() {
            return false;
        }
        if std::fs::create_dir_all(&w_dir).is_err() {
            return false;
        }
        std::fs::copy(&e_src, w_dir.join("tiny.en-encoder.int8.onnx")).is_ok()
            && std::fs::copy(&d_src, w_dir.join("tiny.en-decoder.int8.onnx")).is_ok()
            && std::fs::copy(&t_src, w_dir.join("tiny.en-tokens.txt")).is_ok()
    } else if model_type == "paraformer" {
        // paraformer bundled checking
        let p_dir = dir.join("paraformer");
        let m_src = resources.join("paraformer").join("model.int8.onnx");
        let t_src = resources.join("paraformer").join("tokens.txt");
        if !m_src.exists() || !t_src.exists() {
            return false;
        }
        if std::fs::create_dir_all(&p_dir).is_err() {
            return false;
        }
        std::fs::copy(&m_src, p_dir.join("model.int8.onnx")).is_ok()
            && std::fs::copy(&t_src, p_dir.join("tokens.txt")).is_ok()
    } else {
        false
    }
}

fn download_file(url: &str, path: &PathBuf, name: &str, app: &AppHandle) -> Result<(), String> {
    eprintln!("[model] Downloading {} from {} ...", name, url);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let mut resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Download failed for {}: {}", name, e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Download failed for {}: HTTP {}",
            name,
            resp.status()
        ));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut file =
        std::fs::File::create(path).map_err(|e| format!("Cannot create {}: {}", name, e))?;

    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 65536];
    let mut last_progress: u64 = 0;

    loop {
        let n = resp
            .read(&mut buf)
            .map_err(|e| format!("Read error for {}: {}", name, e))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("Write error for {}: {}", name, e))?;
        downloaded += n as u64;

        // Emit progress every 1MB
        if downloaded - last_progress > 1_000_000 || (total > 0 && downloaded >= total) {
            let _ = app.emit_all(
                "download-progress",
                DownloadProgress {
                    file: name.to_string(),
                    downloaded,
                    total,
                },
            );
            last_progress = downloaded;
        }
    }

    eprintln!("[model] Downloaded {} ({} bytes)", name, downloaded);
    Ok(())
}
