# FlashSpeech

**本地语音转文字桌面应用** — 按下快捷键说话，松开即输入文字。完全离线，无需联网。

<p align="center">
  <img src="flash_speech_app/src-tauri/icons/icon.png" width="128" alt="FlashSpeech Icon" />
</p>

## 特性

- **极速识别** — SenseVoice 模型本地运行，转录延迟 ~70ms/10s 音频
- **完全离线** — 模型首次下载后无需网络，隐私安全
- **多语言** — 支持中文、英文、日语、韩语、粤语
- **一键输入** — 识别结果通过 CGEvent Unicode 直接键入光标位置，不占用剪贴板
- **轻量级** — 应用 ~50MB，无 Python 依赖，启动 < 2 秒
- **Liquid Glass UI** — 借鉴 Apple Liquid Glass 设计语言，亮色半透明毛玻璃 HUD，支持状态形变动画
- **macOS 原生体验** — 支持 macOS (Intel / Apple Silicon)，Windows / Linux 实验性支持

## 快速开始

### 下载安装

从 [Releases](https://github.com/alexlee2046/flash-speech/releases) 页面下载对应平台的安装包：

| 平台 | 标准版（首次联网下载模型） | 完整版（含模型，开箱即用） |
|------|------|------|
| macOS (Apple Silicon) | `FlashSpeech_x.x.x_aarch64.dmg` | `FlashSpeech_x.x.x_aarch64_with-model.dmg` |
| macOS (Intel) | `FlashSpeech_x.x.x_x86_64.dmg` | `FlashSpeech_x.x.x_x86_64_with-model.dmg` |
| Windows | `FlashSpeech_x.x.x_x64-setup.exe` | — |
| Linux (Debian/Ubuntu) | `flash-speech_x.x.x_amd64.deb` | — |

> 标准版首次启动会自动从 HuggingFace 下载语音模型（~228MB），之后完全离线。网络不便的用户建议下载完整版。

### macOS 安装说明

由于应用未经 Apple 签名，macOS 会阻止打开。安装后需要在终端运行一次以下命令：

```bash
xattr -cr /Applications/FlashSpeech.app
```

然后正常双击打开即可。

### macOS 权限设置

首次使用需要授权：

1. **麦克风权限** — 系统会自动弹窗请求，点击"允许"
2. **辅助功能权限** — 用于自动粘贴文字到输入框
   - 打开 **系统设置 → 隐私与安全性 → 辅助功能**
   - 添加 FlashSpeech 并开启

## 使用方法

### 基本操作

| 操作 | macOS | Windows / Linux | 说明 |
|------|-------|-----------------|------|
| 开始/停止录音 | `⌥ Option + Space` | `Alt + Space` | 切换式，按一次开始，再按一次停止 |
| 打开菜单 | 双指轻点 HUD / 右键 | 右键点击 HUD | 显示退出选项 |
| 移动窗口 | 左键拖拽 HUD | 左键拖拽 HUD | 拖拽到屏幕任意位置 |
| 显示/隐藏 HUD | 点击菜单栏托盘图标 | 点击系统托盘图标 | — |

> **macOS 用户注意**：快捷键中的 `Alt` 对应 Mac 键盘上的 `⌥ Option` 键。如果你的系统设置中 `Option + Space` 被 Spotlight 或输入法切换占用，需要先到 **系统设置 → 键盘 → 键盘快捷键** 中关闭冲突项。

### 工作流程

```
按下 ⌥Space → 听到"嘟"声 → 开始说话 → 再按 ⌥Space → 听到"嘟嘟"声 → 自动识别并输入文字
```

### HUD 状态

| 状态 | 显示 | 含义 |
|------|------|------|
| 灰色呼吸点 | 玻璃胶囊内小圆点缓慢闪烁 | 空闲，等待录音 |
| 红色边框 + 波形 | 玻璃边缘泛红，跳动的音频波形 | 正在录音 |
| 旋转圈 | 加载动画 | 正在识别 |
| 绿勾 + 文字 | 识别结果预览（深色文字） | 识别完成，文字已输入 |
| 琥珀色警告 | 三角警告图标 | 识别出错 |
| 灰色圆点（静止） | 暗灰色圆点 | 后端未连接 |

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
cd flash-speech/flash_speech_app

# 安装前端依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布版本
npm run tauri build
```

构建产物位于 `flash_speech_app/src-tauri/target/release/bundle/`。

### macOS 签名说明

构建后安装到 `/Applications` 时，使用以下签名命令保持 TCC 权限稳定（避免每次重新构建后需要重新授权辅助功能）：

```bash
# 分层签名：dylib → binary → bundle
codesign --force --sign - /Applications/FlashSpeech.app/Contents/Frameworks/*.dylib
codesign --force --sign - --entitlements src-tauri/entitlements.plist \
  --requirements '=designated => identifier "com.flashspeech.assistant"' \
  /Applications/FlashSpeech.app/Contents/MacOS/FlashSpeech
codesign --force --sign - --entitlements src-tauri/entitlements.plist \
  --requirements '=designated => identifier "com.flashspeech.assistant"' \
  /Applications/FlashSpeech.app
```

> 关键：`--requirements` 参数让 macOS TCC 通过 bundle identifier（而非 CDHash）识别应用，这样重新构建后无需重新授权辅助功能权限。

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

### 核心依赖

| 组件 | 技术 | 用途 |
|------|------|------|
| 框架 | [Tauri](https://tauri.app/) v1 | 桌面应用框架 |
| 语音识别 | [sherpa-rs](https://github.com/thewh1teagle/sherpa-rs) (sherpa-onnx) | SenseVoice 模型推理 |
| 音频录制 | [cpal](https://github.com/RustAudio/cpal) | 跨平台音频输入/输出 |
| 前端 | React 19 + Vite 7 + Tailwind 3 | UI 渲染 |
| 动画 | [Framer Motion](https://www.framer.com/motion/) | HUD 状态切换动画 |

### 项目结构

```
flash_speech/
├── flash_speech_app/
│   ├── src/                    # 前端源码
│   │   ├── App.tsx             # 主组件，Tauri 事件监听
│   │   ├── components/
│   │   │   └── HUD.tsx         # HUD 组件（状态显示、拖拽、菜单）
│   │   └── index.css           # 样式（Liquid Glass 材质）
│   └── src-tauri/
│       ├── src/
│       │   ├── main.rs         # Tauri 入口、状态机、快捷键
│       │   ├── audio.rs        # 麦克风录制（cpal）
│       │   ├── recognizer.rs   # 语音识别（sherpa-rs）
│       │   ├── injector.rs     # 文本注入（跨平台）
│       │   ├── model.rs        # 模型下载管理
│       │   └── sound.rs        # 音频反馈蜂鸣声
│       ├── Cargo.toml
│       └── tauri.conf.json
├── .github/workflows/
│   └── release.yml             # 多平台自动构建
├── LICENSE
└── README.md
```

## 常见问题

### 首次启动时模型下载失败？

模型从 HuggingFace 下载，如果网络不稳定，可以手动下载：

1. 下载模型文件：[sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models)
2. 将 `model.int8.onnx` 和 `tokens.txt` 放到：
   - macOS: `~/Library/Application Support/com.flashspeech.assistant/models/`
   - Windows: `%APPDATA%/com.flashspeech.assistant/models/`
   - Linux: `~/.local/share/com.flashspeech.assistant/models/`

### 录音后没有输出文字？

1. 确认麦克风权限已授予
2. 确认辅助功能权限已授予（macOS）— 文本注入通过 `CGEventKeyboardSetUnicodeString` 直接键入，需要辅助功能权限
3. 录音时长需超过 0.3 秒（防误触设计）
4. 确保光标在可输入的文本框中
5. 如果从源码重新构建，签名时需指定 `--requirements` 参数保持 TCC 权限稳定（详见构建说明）

### macOS 提示"已损坏，无法打开"？

这是因为应用未经 Apple 签名。打开终端运行：

```bash
xattr -cr /Applications/FlashSpeech.app
```

然后重新打开应用即可。如果提示"无法验证开发者"，右键点击应用 → 打开，或在 **系统设置 → 隐私与安全性** 中点击"仍要打开"。

### Linux 下文字无法自动输入？

需要安装 `xclip`（或 `xsel`）和 `xdotool`：

```bash
sudo apt install xclip xdotool
```

### 如何更换快捷键？

当前版本快捷键为 `Alt + Space`，暂不支持自定义。后续版本会加入设置界面。

## 致谢

- [SenseVoice](https://github.com/FunAudioLLM/SenseVoice) — 阿里达摩院多语言语音识别模型
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) — 高性能语音识别推理引擎
- [Tauri](https://tauri.app/) — 轻量级桌面应用框架

## 许可证

[MIT License](LICENSE)
