# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

FlashSpeech 是一个 macOS 语音转文字桌面应用（v2.0.0），采用纯 Rust 架构：Tauri 主进程内直接运行 SenseVoice 语音识别，零 Python 依赖。

## Architecture

```
Frontend (React) → Tauri Events → Rust Backend (in-process)
                                   ├─ sherpa-rs (SenseVoice ONNX)
                                   ├─ cpal (音频录制)
                                   └─ osascript (文本注入)
```

```
src-tauri/src/
├── main.rs          # Tauri 主入口: AppState, commands, 全局快捷键, 系统托盘
├── audio.rs         # cpal 麦克风录制 (多格式/多声道/自动重采样)
├── recognizer.rs    # sherpa-rs SenseVoice 语音识别
├── injector.rs      # 文本注入 (pbcopy + osascript Cmd+V)
├── model.rs         # 模型下载管理 (HuggingFace, 进度事件)
└── sound.rs         # 音频反馈蜂鸣声 (cpal 输出)
```

**通信机制**: 前端通过 Tauri events (`state-change`) 接收状态更新，通过 `invoke()` 调用 Rust commands。
**状态机**: `starting → idle → listening → processing → result → idle`

## Commands

### 开发
```bash
cd flash_speech/flash_speech_app
npm install                             # 安装前端依赖
npm run build                           # TypeScript + Vite 构建
npm run tauri dev                       # 开发模式
npm run tauri build                     # 打包 .app / .dmg
```

### Rust 检查
```bash
cd flash_speech/flash_speech_app/src-tauri
cargo check                             # 编译检查
cargo build                             # 完整构建
```

## Key Technical Details

- **语音模型**: SenseVoice (sherpa-onnx) — 中英日韩粤五语言，首次运行从 HuggingFace 下载 (~100MB)
- **模型路径**: `~/Library/Application Support/com.flashspeech.assistant/models/`
- **音频录制**: cpal (CoreAudio) → 原生采样率录制 → 线性插值重采样到 16kHz
- **文本注入**: pbcopy 写入剪贴板 → osascript 模拟 Cmd+V → 恢复旧剪贴板
- **音频反馈**: 开始录音 880Hz 0.15s / 停止录音 440Hz×2 (间隔 0.05s)
- **防抖**: 快捷键 600ms 防抖 + 最短录音 0.3s 防误触
- **macOS 特有**: `ActivationPolicy::Accessory`（无 Dock 图标）、NSWindow 透明背景
- **HUD 窗口**: always-on-top、无装饰、600x120、支持拖拽
- **Tauri v1** (非 v2)，`@tauri-apps/api` v1.6、`tauri` crate v1.8
- **前端**: React 19 + Vite 7 + Tailwind 3 + Framer Motion + Lucide Icons
- **Cargo registry**: 使用 rsproxy.cn 镜像
