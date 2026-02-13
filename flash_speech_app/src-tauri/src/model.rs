use std::io::{Read, Write};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const MODEL_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/model.int8.onnx";
const TOKENS_URL: &str = "https://huggingface.co/csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/resolve/main/tokens.txt";

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

pub fn model_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("model.int8.onnx"))
}

pub fn tokens_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(model_dir(app)?.join("tokens.txt"))
}

pub fn is_model_ready(app: &AppHandle) -> bool {
    model_path(app).map(|p| p.exists()).unwrap_or(false)
        && tokens_path(app).map(|p| p.exists()).unwrap_or(false)
}

pub fn ensure_model(app: &AppHandle) -> Result<(), String> {
    if is_model_ready(app) {
        eprintln!("[model] Model already downloaded");
        return Ok(());
    }

    let dir = model_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create model dir: {}", e))?;

    download_file(MODEL_URL, &dir.join("model.int8.onnx"), "model.int8.onnx", app)?;
    download_file(TOKENS_URL, &dir.join("tokens.txt"), "tokens.txt", app)?;

    Ok(())
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
