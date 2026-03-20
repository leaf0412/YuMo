# 语墨 YuMo

语音转文字桌面工具，支持本地模型和云端服务，说完即粘贴。

## 功能

- **一键录音转写** — 全局快捷键触发，转写完自动粘贴到光标位置
- **多模型支持** — 本地 Whisper、Apple Silicon GPU 加速（MLX）、云端 API
- **AI 增强** — 接入 OpenAI / Anthropic / Ollama，自定义 prompt 润色转写结果
- **语音活动检测（VAD）** — 自动识别语音片段，过滤静音
- **降噪** — 内置降噪处理
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

前往 [Releases](https://github.com/leaf0412/YuMo/releases) 下载最新 `.dmg`，打开后将 YuMo 拖入 Applications。

> 首次打开可能被 macOS Gatekeeper 拦截，右键点击 → 打开即可。

### 从源码构建

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布
npm run tauri build
```

需要：Node.js 20+、Rust 1.80+、macOS 11+

## 使用指南

### 首次使用

启动后会进入**引导向导**，按步骤完成设置：

1. **授予权限** — 授予麦克风和辅助功能权限
2. **选择模型** — 选择一个模型并下载
   - 推荐 Apple Silicon 用户选择 **MLX Fun-ASR Nano (8-bit)**（速度快、质量好）
   - 没有 Apple Silicon 可选 **Whisper Base**（体积小）或云端模型
3. **设置快捷键** — 录制全局快捷键
4. **开始使用** — 按快捷键录音，松开后自动转写并粘贴

> 引导向导会自动检测已完成的步骤，跳过已配置的项目。

### 录音流程

```
按下快捷键 → 录音中 → 松开 → 转写中 → [AI 增强] → 粘贴到光标
```

也可以在首页点击录音按钮手动操作。录音过程中连按两次 ESC 可取消录音。

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
| VAD 灵敏度 | 语音活动检测灵敏度（0-100） |
| 静音超时 | 连续静音多久后停止录音（100-5000ms） |
| 录音提示音 | 开始/结束录音时播放提示音 |
| 录音时静音系统 | 录音期间静音系统音频输出 |
| 自动大写 | 自动将句首字母大写 |
| 剪贴板恢复 | 粘贴后恢复原剪贴板内容 |
| 粘贴延迟 | 粘贴前等待时间（0-1000ms） |
| 精灵图 | 自定义录音浮窗动画，可调整大小（80-300px） |
| 菜单栏模式 | 隐藏 Dock 图标，仅在菜单栏显示 |
| 开机自启 | 登录时自动启动 |
| 数据导入 | 从 VoiceInk macOS 版导入历史记录 |

## 数据存储

所有数据存储在 `~/.voiceink/`：

```
~/.voiceink/
├── log.txt              # 应用日志
├── data.db              # 数据库（设置、转写记录）
├── models/              # 下载的模型文件
├── recordings/          # 录音 WAV 文件
├── venv/                # Python 虚拟环境（MLX 模型用）
└── sprites/             # 录音动画精灵图
```

## 技术栈

- **前端**：React 19 + TypeScript + Ant Design + Zustand
- **后端**：Rust + Tauri v2
- **转写**：whisper.cpp (CPU) + MLX (GPU) + 云端 API
- **数据库**：SQLite

## 许可

MIT
