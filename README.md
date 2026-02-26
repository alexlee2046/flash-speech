# FlashSpeech

**本地语音转文字桌面应用** | 按下快捷键说话，松开即输入文字 | 完全离线，隐私安全

[English](README.md) | [中文](README.md)

<p align="center">
  <img src="flash_speech_app/src-tauri/icons/icon.png" width="128" alt="FlashSpeech Icon" />
</p>

<p align="center">

[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-blue?style=flat-square)](https://github.com/alexlee2046/flash-speech/releases)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.60+-orange?style=flat-square)](https://rustup.rs/)
[![Tauri](https://img.shields.io/badge/Tauri-v1.8-blue?style=flat-square)](https://tauri.app/)
[![Version](https://img.shields.io/badge/version-2.0.0-red?style=flat-square)](https://github.com/alexlee2046/flash-speech/releases)

</p>

> **🚀 快速访问**: [GitHub Releases](https://github.com/alexlee2046/flash-speech/releases) | [Gitee Releases](https://gitee.com/alex2046/flash-speech/releases) | [原理解析](docs/)

## 特性亮点

| 特性 | 说明 |
|------|------|
| ⚡ **极速识别** | SenseVoice 模型本地运行，转录延迟 ~70ms/10s 音频 |
| 🔒 **完全离线** | 模型首次下载后无需网络，隐私安全不泄露 |
| 🌍 **多语言** | 支持中文、英语、日语、韩语、粤语 |
| ⌨️ **一键输入** | 识别结果通过 CGEvent Unicode 直接键入光标位置 |
| 📦 **轻量级** | 应用 ~50MB，无 Python 依赖，启动 < 2 秒 |
| 🎨 **Liquid Glass UI** | Apple Liquid Glass 设计语言，半透明毛玻璃 HUD |
| 🍎 **macOS 原生** | 无 Dock 图标，透明悬浮窗口，支持 Intel / Apple Silicon |

## 适用场景

- 📝 **文字输入** — 说话代替打字，适合长文本输入
- 🧑‍💻 **开发者** — 编程时用语音写注释、文档
- ✍️ **写作** — 语音写作灵感捕捉
- 🌐 **翻译** — 多语言实时语音转写

## 快速开始

### 下载安装

| 平台 | 标准版（首次联网下载模型） | 完整版（含模型） |
|------|------|------|
| macOS (Apple Silicon) | `FlashSpeech_x.x.x_aarch64.dmg` | `FlashSpeech_x.x.x_aarch64_with-model.dmg` |
| macOS (Intel) | `FlashSpeech_x.x.x_x86_64.dmg` | `FlashSpeech_x.x.x_x86_64_with-model.dmg` |
| Windows | `FlashSpeech_x.x.x_x64-setup.exe` | — |
| Linux | `flash-speech_x.x.x_amd64.deb` | — |

> **下载渠道**：<br>
> - GitHub: https://github.com/alexlee2046/flash-speech/releases<br>
> - Gitee: https://gitee.com/alex2046/flash-speech/releases<br>
> - 标准版首次启动会自动从 HuggingFace 下载语音模型（~228MB），之后完全离线

### macOS 安装说明

由于应用未经 Apple 签名，macOS 会阻止打开。安装后需要在终端运行：

```bash
xattr -cr /Applications/FlashSpeech.app
```

然后正常双击打开即可。

### macOS 权限设置

1. **麦克风权限** — 系统会自动弹窗请求，点击"允许"
2. **辅助功能权限** — 用于自动粘贴文字到输入框
   - 打开 **系统设置 → 隐私与安全性 → 辅助功能**
   - 添加 FlashSpeech 并开启

## 使用方法

### 快捷键

| 操作 | macOS | Windows / Linux |
|------|-------|-----------------|
| 开始/停止录音 | `⌥ Option + Space` | `Alt + Space` |
| 打开菜单 | 双指轻点 HUD / 右键 | 右键点击 HUD |
| 移动窗口 | 左键拖拽 HUD | 左键拖拽 HUD |

### 工作流程

```
按下 ⌥Space → 听到"嘟"声 → 开始说话 → 再按 ⌥Space → 听到"嘟嘟"声 → 自动识别并输入文字
```

### HUD 状态说明

| 状态 | 显示 | 含义 |
|------|------|------|
| 灰色呼吸点 | 玻璃胶囊内小圆点缓慢闪烁 | 空闲，等待录音 |
| 红色边框 + 波形 | 玻璃边缘泛红，跳动的音频波形 | 正在录音 |
| 旋转圈 | 加载动画 | 正在识别 |
| 绿勾 + 文字 | 识别结果预览 | 识别完成，文字已输入 |
| 琥珀色警告 | 三角警告图标 | 识别出错 |

## 从源码构建

### 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) >= 1.60
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

**Linux 额外依赖：**

```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.0-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev cmake
```

### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/alexlee2046/flash-speech.git
# 或 Gitee
git clone https://gitee.com/alex2046/flash-speech.git

cd flash-speech/flash_speech_app

# 安装前端依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布版本
npm run tauri build
```

## 技术架构

```
┌──────────────────────────────────────────────────┐
│  Frontend (React + Tailwind + Framer Motion)     │
│  HUD 窗口 · 状态动画 · 拖拽 · 右键菜单            │
└──────────────────┬───────────────────────────────┘
                   │ Tauri Events / Commands
┌──────────────────┴───────────────────────────────┐
│  Backend (Rust, in-process)                      │
│                                                  │
│  ┌─────────┐ ┌────────────┐ ┌─────────────────┐ │
│  │  cpal   │ │ sherpa-rs  │ │   injector      │ │
│  │ 音频录制 │ │ SenseVoice │ │ 文本注入         │ │
│  │         │ │ ONNX 推理   │ │(CGEvent Unicode) │ │
│  └─────────┘ └────────────┘ └─────────────────┘ │
└──────────────────────────────────────────────────┘
```

### 核心技术栈

| 组件 | 技术 | 用途 |
|------|------|------|
| 框架 | [Tauri](https://tauri.app/) v1 | 桌面应用框架 |
| 语音识别 | [sherpa-rs](https://github.com/thewh1teagle/sherpa-rs) (sherpa-onnx) | SenseVoice 模型推理 |
| 音频录制 | [cpal](https://github.com/RustAudio/cpal) | 跨平台音频输入/输出 |
| 前端 | React 19 + Vite 7 + Tailwind 3 | UI 渲染 |
| 动画 | [Framer Motion](https://www.framer.com/motion/) | HUD 状态切换动画 |

## 常见问题

### Q: 首次启动时模型下载失败？

模型从 HuggingFace 下载，如果网络不稳定，可以手动下载：

1. 下载模型文件：[sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models)
2. 将 `model.int8.onnx` 和 `tokens.txt` 放到：
   - macOS: `~/Library/Application Support/com.flashspeech.assistant/models/`
   - Windows: `%APPDATA%/com.flashspeech.assistant/models/`
   - Linux: `~/.local/share/com.flashspeech.assistant/models/`

### Q: 录音后没有输出文字？

1. 确认麦克风权限已授予
2. 确认辅助功能权限已授予（macOS）
3. 录音时长需超过 0.3 秒（防误触设计）
4. 确保光标在可输入的文本框中

### Q: macOS 提示"已损坏，无法打开"？

```bash
xattr -cr /Applications/FlashSpeech.app
```

### Q: Linux 下文字无法自动输入？

```bash
sudo apt install xclip xdotool
```

## 致谢

- [SenseVoice](https://github.com/FunAudioLLM/SenseVoice) — 阿里达摩院多语言语音识别模型
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) — 高性能语音识别推理引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架

## 许可证

[MIT License](LICENSE)

---

<p align="center">
  如果这个项目对你有帮助，欢迎 ⭐ Star 支持！
</p>
