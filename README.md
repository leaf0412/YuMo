# 语墨 YuMo

语音转文字桌面工具，支持本地模型和云端服务，说完即粘贴。

支持 **macOS**、**Windows**、**Linux**。

## 功能

- **一键录音转写** — 全局快捷键触发，转写完自动粘贴到光标位置
- **多模型支持** — 本地 Whisper、Apple Silicon GPU 加速（MLX）、云端 API
- **AI 增强** — 接入 OpenAI / Anthropic / Ollama，自定义 prompt 润色转写结果
- **语音活动检测（VAD）** — 自动识别语音片段，过滤静音
- **降噪** — 内置 DTLN 降噪处理
- **长音频分片** — 自动 VAD 静音点切分，支持长时间录音转写
- **多语言转写** — 中、英、日、韩、法、德、西、俄、葡、意等 10+ 语言
- **中英双语界面** — 支持中文和英文 UI，可跟随系统语言自动切换
- **幻觉过滤** — 自动检测并过滤 Whisper 无效输出（如重复 token、纯符号等）
- **统计面板** — 可视化展示转写次数、时长、节省击键数等统计数据
- **录音精灵图** — 自定义录音浮窗动画，支持导入文件夹或 ZIP 包
- **数据迁移** — 支持从 VoiceInk macOS 版导入历史转写记录

## 支持的模型

### 本地模型（离线可用）

| 模型 | 大小 | 说明 |
|------|------|------|
| Whisper Tiny / Base / Small / Medium / Large v3 | 75MB - 3GB | CPU 推理，英文或多语言 |
| MLX Whisper Large v3 / Distil Large v3 / Small | 500MB - 3GB | Apple Silicon GPU 加速 |
| MLX Fun-ASR Nano (8-bit / BF16) | 2 - 4GB | GPU 加速，多语言 |
| Qwen3-ASR 0.6B (8-bit / BF16) | 700MB - 1.2GB | GPU 加速，30+ 语言 |

### 云端模型（需 API Key）

Groq Whisper · Deepgram Nova-2 · ElevenLabs Scribe · Mistral ASR · Gemini ASR · Soniox ASR

## 安装

### 从 Release 下载

前往 [Releases](https://github.com/leaf0412/YuMo/releases) 下载适合你平台的安装包：

| 平台 | 版本 | 格式 | 说明 |
|------|------|------|------|
| macOS (ARM) | Tauri | `.dmg` | Apple Silicon Mac，推荐 |
| macOS (Intel) | Tauri | `.dmg` | Intel Mac |
| Windows | Tauri | `.exe` | Windows 10+ |
| Linux | Tauri | `.AppImage` / `.deb` | Ubuntu 22.04+，需要 WebKitGTK 4.1 |
| Linux (兼容) | Electron | `.AppImage` / `.deb` | Ubuntu 20.04+，旧系统兼容版 |

> macOS 首次打开可能被 Gatekeeper 拦截，右键点击 → 打开即可。

### 从源码构建

#### 环境要求

- Node.js 20+
- pnpm 9+
- Rust 1.80+

#### Tauri 版本（推荐）

```bash
pnpm install
pnpm tauri dev          # 开发模式
pnpm tauri build        # 构建发布
```

#### Electron 版本

```bash
pnpm install

# 构建 napi addon（Rust 核心库的 Node.js 绑定）
cargo build --release -p yumo-napi
cp target/release/libyumo_napi.dylib napi/yumo-napi.darwin-arm64.node  # macOS ARM
# Linux: cp target/release/libyumo_napi.so napi/yumo-napi.linux-x64-gnu.node

# 开发模式
pnpm electron:dev

# 构建发布
pnpm electron:build
pnpm electron-builder --mac    # macOS DMG
pnpm electron-builder --linux  # Linux AppImage + deb
```

## 使用指南

### 首次使用

启动后会进入**引导向导**，按步骤完成设置：

1. **授予权限** — 授予麦克风和辅助功能权限（macOS）
2. **选择模型** — 选择一个模型并下载
   - 推荐 Apple Silicon 用户选择 **MLX Fun-ASR Nano (8-bit)**（速度快、质量好）
   - 没有 Apple Silicon 可选 **Whisper Base**（体积小）或云端模型
3. **设置快捷键** — 录制全局快捷键
4. **开始使用** — 按快捷键录音，松开后自动转写并粘贴

### 录音流程

```
按下快捷键 → 录音中 → 再按快捷键 → 转写中 → [AI 增强] → 粘贴到光标
```

录音过程中连按两次 ESC 可取消录音。

### 模型管理

- **本地模型**：点击下载，等待完成后选中即可使用
- **MLX 模型**：首次使用会自动安装 Python 环境和依赖（约 1-2 分钟），之后自动下载模型
- **云端模型**：填入对应服务商的 API Key 后即可使用

### AI 增强（可选）

在「增强」页面：

1. 选择 LLM 服务商（OpenAI / Anthropic / Ollama）
2. 填入 API Key
3. 选择或创建 prompt（如"翻译为英文"、"修正语法"、"总结要点"）
4. 开启增强开关

转写结果会先经过 LLM 处理再粘贴。

### 设置说明

| 设置 | 说明 |
|------|------|
| 界面语言 | 中文 / English / 跟随系统 |
| 音频设备 | 选择录音使用的麦克风 |
| 转写语言 | 转写语言（auto 自动检测，或指定语言） |
| 降噪 | 开启录音降噪处理 |
| 静音超时 | 连续静音多久后停止录音（100-5000ms） |
| 录音时静音系统 | 录音期间静音系统音频输出 |
| 自动大写 | 自动将句首字母大写 |
| 精灵图 | 自定义录音浮窗动画，可调整大小 |
| 开机自启 | 登录时自动启动（macOS） |
| 数据导入 | 从 VoiceInk macOS 版导入历史记录 |

## 数据存储

所有数据存储在 `~/.voiceink/`（macOS/Linux）或 `%APPDATA%\YuMo`（Windows）：

```
~/.voiceink/
├── log.txt              # 应用日志
├── data.db              # 数据库（设置、转写记录）
├── models/              # 下载的模型文件
├── recordings/          # 录音 WAV 文件
├── denoiser/            # DTLN 降噪模型
├── venv/                # Python 虚拟环境（MLX 模型用）
└── sprites/             # 录音动画精灵图
```

## 项目架构

```
voiceink-tauri/
├── crates/yumo-core/       # Rust 核心库（零 Tauri 依赖）
│   └── src/platform/       # 平台抽象层
│       ├── traits.rs       # 5 个 trait 定义
│       ├── macos/          # macOS 实现（CoreAudio）
│       ├── windows/        # Windows 实现（cpal + WASAPI）
│       └── linux/          # Linux 实现（cpal + PulseAudio）
├── src/                    # 共享前端（React + TypeScript）
│   ├── bridge/             # Tauri/Electron 平台桥接
│   └── lib/events.ts       # 跨平台事件系统
├── src-tauri/              # Tauri 壳（薄封装层）
├── napi/                   # napi-rs 绑定（yumo-core → Node.js addon）
├── src-electron/           # Electron 壳
│   ├── main.ts             # 主进程
│   ├── windows.ts          # 窗口管理
│   ├── addon.ts            # napi addon 加载
│   └── ipc/                # 模块化 IPC handlers（10 个领域）
└── .github/workflows/      # CI/CD（多平台矩阵构建）
```

### 双壳架构

```
Tauri:    前端 → bridge/tauri.ts → invoke → src-tauri/commands.rs → yumo-core
Electron: 前端 → bridge/electron.ts → IPC → src-electron/ipc/*.ts → napi addon → yumo-core
```

两套壳共享同一份 `yumo-core` Rust 核心库和同一份 React 前端代码。

## 技术栈

- **前端**：React 19 + TypeScript + Ant Design + Zustand + i18next
- **核心库**：Rust (`yumo-core` crate)
- **Tauri 壳**：Tauri v2
- **Electron 壳**：Electron + napi-rs + esbuild
- **转写**：whisper.cpp (CPU) + MLX (GPU) + 云端 API
- **音频**：CoreAudio (macOS) / cpal (Windows/Linux)
- **凭据**：Keychain (macOS) / Credential Manager (Windows) / Secret Service (Linux)
- **数据库**：SQLite (rusqlite)
- **CI/CD**：GitHub Actions 矩阵构建

## 许可

MIT
