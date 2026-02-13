fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path");
            println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
            // Copy onnxruntime dylib to Frameworks/ for Tauri macOS bundling
            copy_onnxruntime_dylib();
        }
        "linux" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
        }
        _ => {}
    }

    tauri_build::build()
}

fn copy_onnxruntime_dylib() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = std::path::Path::new(&out_dir);
    // OUT_DIR = target/{target}/release/build/{name}-{hash}/out
    // ancestors().nth(3) = target/{target}/release/
    let Some(release_dir) = out_path.ancestors().nth(3) else {
        return;
    };
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let fw_dir = std::path::Path::new(&manifest_dir).join("Frameworks");
    std::fs::create_dir_all(&fw_dir).ok();

    if let Ok(entries) = std::fs::read_dir(release_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".dylib")
                && (name.starts_with("libonnxruntime") || name.starts_with("libsherpa-onnx"))
            {
                let dst = fw_dir.join(name.as_ref());
                std::fs::copy(entry.path(), &dst).ok();
                println!("cargo:warning=Copied {} to Frameworks/", name);
            }
        }
    }
}
