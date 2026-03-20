# Changelog

所有重要变更都会记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [0.2.0] - 2026-03-21

### Added - 新增
- 中英双语界面（i18n），支持中文、英文、跟随系统语言自动切换
- 录音精灵图管理 — 支持从文件夹或 ZIP 导入自定义录音动画，可调整大小（80-300px）和背景移除
- 精灵图背景移除 — 自动采样角落颜色，按阈值去除背景
- Whisper 幻觉输出检测与过滤 — 自动识别括号包裹、符号过多、重复 token 等无效转写结果
- 统计面板 — 可视化展示转写次数、录音时长、节省击键数等数据
- 引导向导 — 首次启动引导用户完成权限授予、模型下载、快捷键设置
- 全局 ESC 双击取消录音
- 从 VoiceInk macOS 版导入历史转写记录，支持手动选择文件
- 词典 CSV 导入

### Changed - 变更
- 包名从 voiceink-tauri 重命名为 yumo
- 首页从简单仪表盘改为统计面板（含图表）
- CJK 字数统计和日期格式化优化

### Fixed - 修复
- 统计数据通过 stats-updated 事件实时刷新
- 录音启动延迟优化（优先启动 AudioUnit）
- 录音前静音系统音频（而非录音后）
- 引导向导自动检测已完成步骤
- 音频设备枚举与已保存设备校验修复
- 录音时长基于 PCM 采样数精确计算

### Removed - 移除
- 移除自动清理历史记录功能

## [0.1.0] - 2026-03-15

### Added
- 初始版本发布
- 一键录音转写，全局快捷键触发
- 本地 Whisper / MLX 模型支持
- 云端模型（Groq、Deepgram、ElevenLabs、Mistral、Gemini、Soniox）
- AI 增强（OpenAI / Anthropic / Ollama）
- VAD 语音活动检测、降噪
- 词典替换和自动大写
- 菜单栏模式、开机自启
