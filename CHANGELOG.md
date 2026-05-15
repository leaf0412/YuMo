# Changelog

所有重要变更都会记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [0.8.1] - 2026-05-15

### Added - 新增
- **ModelCard 卡片抽象** — 新增共享组件 `src/components/ModelCard/`，CustomModels / Models 页统一卡片渲染
- **custom worker 解耦** — Rust 端 `crates/yumo-core/src/custom_worker.rs` + Python 端 `custom_model_shared.py` / `custom_model_worker.py`，custom 模型 worker 从 `mlx_funasr_daemon` 剥离

### Changed - 变更
- **模型主键 model_repo → model_id**（破坏性）—— daemon 主键改用 modelId，避免不同模型类型共用 HF repo 名时冲突
  - bridge: `daemonLoadModel(modelRepo)` → `daemonLoadModel(modelId)`
  - store 新增 `selectModel` action，统一走 `select_model` 命令
  - 老安装升级后 daemon 已加载状态会重置（首次启动需重新选模型）
- **mlx_funasr_daemon.py 瘦身 236 行** — custom 模型逻辑剥离到独立 worker
- **recorder 日志降级** — 三平台 `list_devices` 路径 `info` → `debug`，避免 UI 启动时刷屏 log.txt

### Fixed - 修复
- **Models 页滑块设置无效** — `update_setting` value 双重 JSON 编码，后端 `Value::as_f64()` 拒绝字符串字面量、落回默认值
- **`platform_integration_test`** 补 `PermissionStatus.paste_tools: None` 字段，跟上之前 struct 变更

### Removed - 移除（破坏性）
- **Electron 兼容壳全量移除** — 仓库瘦身为单运行时（Tauri）项目
  - 删除 `src-electron/`、`napi/` crate、`electron-builder.yml`、`dist-electron/`
  - `package.json` 清掉 `electron` / `electron-builder` / `electron-log` / `esbuild` 依赖和 `electron:*` scripts
  - Cargo workspace 移除 `napi` 成员
  - CI release 工作流删 `electron` job，仅保留 Tauri 多平台矩阵
  - 前端 bridge / events / logger 简化为 Tauri-only，删除 runtime 双分支
- **放弃 Ubuntu 20.04 兼容版** — 仅保留 Tauri 路径，最低支持 Ubuntu 22.04 + WebKitGTK 4.1
- **删除 i18n `customModels.title` key** — 标题改由 ModelCard 内部统一处理

## [0.8.0] - 2026-05-07

### Added - 新增
- **自定义模型** — 设置-模型 新增"自定义模型"区域，YAML 插件机制
  - 内置 MiMo INT4 示例
  - 支持 hf_repos / function 两种下载变体
  - 卡片四态渲染（未安装/下载中/已安装/已激活）
  - 首次激活信任对话框
  - daemon 支持 custom 模型 load / transcribe / install / check 链路
  - napi 桥 + Electron IPC + Tauri commands 全栈接入
- **中文数字识别** — 重写为「场景模板 + 量词锚点」双层架构
  - 7 个场景模板：版本号、百分比、千分比、分数、负数、小数、序数
  - 量词锚点扫描 + 12 条「'一'前置限定词」过滤（同一个 / 唯一一个 等保留原文）
  - 决策日志可观测（template_match / quantifier_match / skip / parse_failed）
  - 71 条语料库回归测试 + CI 自动跑

### Changed - 变更
- **中文数字行为变化**（强证据原则）
  - 单字数字 + 量词现在转：`三个 → 3个`、`九点 → 9点`、`第三 → 第3`
  - 独立无量词数字串保留原文：`二十 → 二十`（旧版会转）、`一万两千三百 → 一万两千三百`
  - 小数右侧按位读取保前导零：`零点零五 → 0.05`（不再丢精度）
  - 新增类型支持：负数、百分比、千分比、分数

### Fixed - 修复
- 量词扫描 '两' 字冲突触发 usize 下溢 panic
- 小数右侧 parse_cn_numeral 整数路径丢前导零
- "同一个" / "唯一一个" 等限定词 + '一' 搭配被误转

### Removed - 移除
- `chinese_numerals_to_arabic` / `chinese_version_numbers_to_arabic` 公开 API
  （内部 Rust 模块，公开接口 `process_text` 不变，napi/src-tauri 调用方零影响）

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
