# Changelog

所有重要变更都会记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [0.8.3] - 2026-05-20

### Fixed - 修复
- **「末尾追加句号」OFF 不生效** — ASR 模型（含 `custom-mimo-v2.5-asr-int4`）自带末尾句号，原 toggle 仅做"缺则补"加法、不做减法，用户关闭后输出依然带「。」看上去全无效果。OFF 现在主动剥末尾连续的 `。/.`，`?!？！…⋯` 一律保留；`...`（≥3 ASCII 点）视作省略号也保留
- **`paste_delay` / `system_mute` UI 键名与后端对不上** — UI 写 `paste_delay`，后端却读 `clipboard_restore_delay`；UI 写 `system_mute`，后端读 `system_mute_enabled`。两个 slider / toggle 拨拉一辈子都不会进 pipeline。新 `yumo_core::settings::{resolve_paste_restore_delay_ms, resolve_system_mute}` 集中 UI ↔ 后端 key 映射，10 单测覆盖含死键忽略
- **`clipboard_restore` bool UI 早就有但后端从来不读** —— 关掉也照样恢复剪贴板。现在 false 时 `restore_delay=0`，走 paster 的 `if restore_delay_ms > 0` 旁路跳过 restore
- **`paste_delay` UI 默认 100ms 与后端 1500ms 不一致** —— 历史用户实际行为是 1500ms，UI slider 默认改成 1500ms 对齐；100ms 在某些 app 下 paste 还没完就 restore，会把用户原剪贴板内容也粘出去

### Added - 新增（启用沉睡的 UI 设置）
- **`autostart` 真的能开机自启** — `tauri_plugin_autostart` 之前已 init 但 toggle 只往 DB 写 bool 从不调 plugin 的 enable/disable。现 toggle 真的去改 OS 的 LaunchAgent；启动时 `isEnabled()` 拿 OS 真实状态覆盖 DB，用户从系统设置 / launchctl 外部改过的也能即时反映；capability 补 `autostart:allow-{enable,disable,is-enabled}` 三条
- **`sound_enabled` + `custom_sound_file` 录音提示音** — 之前两个 UI 在 Settings 外零引用，开关失效。新 `yumo_core::audio_cue` 模块基于 rodio：`sound_enabled=false` 直接禁用；`custom_sound_file` 非空走 rodio 解码播放（支持 WAV/MP3/FLAC/OGG）；否则生成默认正弦音（起 880Hz / 止 660Hz，120ms，振幅 0.18）。`play_async` fire-and-forget，播放失败 warn 日志静默吞，不拖崩录音管线。7 单测覆盖路径
- **`menu_bar_mode` 真的隐藏 Dock 图标** — 启动 + 运行时切换都生效，调 `app.set_activation_policy(Accessory/Regular)`；Linux / Windows 无对应概念，`cfg(target_os=macos)` 门闸
- **`auto_cleanup` + `auto_cleanup_days` 定时清理** — 之前 UI 摆设。`db::prune_older_than_days` 按 cutoff 删 SQLite 行 + 同名 WAV/.txt 旁路文件，`ErrorKind::NotFound` 不算失败（孤儿记录常见）；启动时若 `auto_cleanup=true` 在独立线程跑 prune，不阻塞 setup。5 单测含边界 cutoff / wav+txt 双删 / 缺文件容忍 / days=0 全删

### Removed - 移除（清理装饰画 UI / 未实现 feature）
- **AI 增强整条线** — `Enhancement` 页 + 6 个 UI 设置（`ai_enhancement_enabled` / `llm_provider` / `llm_model` / `ollama_url` / `cloud_provider` / `cloud_api_key`）+ commands.rs pipeline 中 `enhanced_text` 整段 TODO 桩（注释明写"enhancement not implemented yet, skipping"）+ `crates/yumo-core/src/enhancer.rs` 整个模块（`build_prompt` / `EnhancerConfig` 无人调用）+ 3 个 keychain API key 命令 + Prompts 子系统（Prompt 表 + 4 个 CRUD + 5 个 tauri 命令）+ `PipelineState::Enhancing` 状态变体。**-2272 行**，老 DB 列 + prompts 表保留无害
- **7 个云端 ASR Provider** — Models 页云端 Tab + `cloud.rs` 整个模块（`CloudProvider` / `CloudConfig` / `build_request_info` / `parse_response` 无人调用）+ `ModelProvider` 枚举的 Groq / Deepgram / ElevenLabs / Mistral / Gemini / Soniox / OpenAI 变体 + `is_cloud()` + `ModelFilter::Cloud` + 7 个预定义模型条目。pipeline 只有 `needs_daemon()` 和 local Whisper 两个分支，选了云端模型等于什么都不发生
- **`keychain` 模块 + `keyring` 依赖** — 三平台 `platform/*/keychain.rs` + `PlatformKeychain` trait 全删，唯一用户（API key 命令）已删
- **VAD 三个 UI 控件**（`vad_enabled` / `vad_sensitivity` / `vad_silence_timeout`） — 后端从来没接，录音管线不调用任何 VAD 逻辑。拨开关 / 拖 slider 全是装饰。`vad.rs::ChunkManager` 单测覆盖完好，留着备用；仅删 UI 入口与 i18n 翻译，DB 老 key 保留无害

## [0.8.2] - 2026-05-16

### Added - 新增
- **文本后处理 3 个开关** — 设置页新建「文本后处理」面板，独立控制转录后文本加工链路
  - `append_period`（默认关）— 末尾自动追加句号，CJK 末尾→「。」/ ASCII 末尾→「.」，已有终止标点 `.?!。？！…⋯` 跳过
  - `convert_cn_numerals`（默认关）— 中文数字转阿拉伯数字总开关，可托底「百度→100度」「万象→10000象」之类量词扫描结构性误伤
  - `use_builtin_dictionary`（默认开）— 内置错别字 / 同音误识词典开关
- **内置错别字词典** — `crates/yumo-core/src/text_processor/builtin_dictionary.json` 首批 22 条种子（百渡→百度、腾迅→腾讯、Github→GitHub、复务器→服务器…），JSON 随发版迭代
  - CJK 词条用子串替换（regex `\b` 在 CJK 内部不触发，嵌入式才能命中）
  - ASCII 词条仍走 word-boundary case-insensitive 匹配
  - 用户自定义 Dictionary 先跑、内置后跑，用户规则可覆盖内置

### Changed - 变更
- **`process_text` 签名收敛**（仅 yumo-core crate 内部破坏性）—— 从 3 位置参数改为 `&ProcessOptions` struct，未来加 toggle 不再破坏 ABI

### Removed - 移除
- **Linux `.AppImage` 产物** — 项目仅承诺兼容 Ubuntu 22.04+，默认 apt 源直接有 `libwebkit2gtk-4.1`，`.deb` 5.7MB 已完美覆盖；AppImage 80MB「跨发行版兜底」对单一目标 distro 是冗余成本

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
