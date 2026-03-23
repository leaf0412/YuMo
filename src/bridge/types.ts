/**
 * Bridge interface types — platform-agnostic abstractions for Tauri/Electron IPC.
 *
 * Type shapes mirror the Rust serde structs serialized from yumo-core.
 * snake_case fields match Rust's default serde output (no rename_all on most structs).
 */

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

export interface AudioInputDevice {
  id: number;
  name: string;
  is_default: boolean;
}

export interface PasteToolsStatus {
  xdotool: boolean;
  wtype: boolean;
}

export interface PermissionStatus {
  microphone: boolean;
  accessibility: boolean;
  paste_tools?: PasteToolsStatus;
}

/** Provider discriminant — mirrors Rust's ModelProvider enum (serde camelCase). */
export type ModelProvider =
  | 'local'
  | 'mlxWhisper'
  | 'mlxFunASR'
  | 'groq'
  | 'deepgram'
  | 'elevenLabs'
  | 'mistral'
  | 'gemini'
  | 'soniox';

export interface ModelInfo {
  id: string;
  name: string;
  size_mb: number;
  /** Subset of keys from supported_languages for backward compat */
  languages: string[];
  supported_languages: Record<string, string>;
  download_url: string;
  is_downloaded: boolean;
  provider: ModelProvider;
  model_repo?: string;
  description?: string;
  /** Speed rating 1-10 */
  speed: number;
  /** Accuracy rating 1-10 */
  accuracy: number;
  is_recommended: boolean;
}

export interface TranscriptionRecord {
  id: string;
  text: string;
  enhanced_text: string | null;
  timestamp: string;
  duration: number;
  model_name: string;
  word_count: number;
  recording_path: string | null;
}

export interface DailyWpm {
  date: string;
  wpm: number;
  session_count: number;
}

export interface WpmStats {
  avg: number;
  max: number;
  min: number;
}

export interface Statistics {
  total_sessions: number;
  total_words: number;
  total_duration_seconds: number;
  total_keystrokes_saved: number;
  time_saved_minutes: number;
  avg_wpm: number;
  daily_wpm: DailyWpm[];
  wpm_stats: WpmStats;
}

export interface PaginatedResult {
  items: TranscriptionRecord[];
  next_cursor: string | null;
}

export interface VocabularyWord {
  id: string;
  word: string;
  created_at: string;
}

export interface Replacement {
  id: string;
  original: string;
  replacement: string;
  created_at: string;
}

export interface Prompt {
  id: string;
  name: string;
  system_message: string;
  user_message_template: string;
  is_predefined: boolean;
  created_at: string;
}

export interface ImportResult {
  transcriptions_imported: number;
  transcriptions_skipped: number;
  vocabulary_imported: number;
  replacements_imported: number;
  recordings_copied: number;
}

export interface DaemonStatus {
  running: boolean;
  loaded_model: string | null;
}

/** Sprite manifest with the dir ID injected by the backend */
export interface SpriteManifest {
  dirId: string;
  spriteFile: string;
  name?: string;
  [key: string]: unknown;
}

export interface AppSettings {
  [key: string]: unknown;
  language?: string;
  selected_model_id?: string;
  selected_prompt_id?: string;
  selected_sprite_id?: string;
  hotkey?: string;
  ui_locale?: string;
  onboarding_completed?: string;
}

export interface PipelineState {
  state: 'idle' | 'recording' | 'transcribing' | 'enhancing' | 'pasting';
}

// ---------------------------------------------------------------------------
// Event callback types
// ---------------------------------------------------------------------------

/** Returns a cleanup function to unsubscribe the listener */
export type Unsubscribe = () => void;

export type RecordingStateCallback = (payload: PipelineState) => void;
export type DownloadProgressCallback = (payload: { model_id?: string; model_repo?: string; progress: number }) => void;
export type DaemonSetupCallback = (payload: { stage: string; message?: string }) => void;
export type EscapePressedCallback = () => void;
export type ToggleRecordingCallback = () => void;
export type StatsUpdatedCallback = () => void;
export type LanguageChangedCallback = (lang: string) => void;

// ---------------------------------------------------------------------------
// Bridge interface
// ---------------------------------------------------------------------------

export interface Bridge {
  // --- Recording pipeline ---
  listAudioDevices(): Promise<AudioInputDevice[]>;
  startRecording(deviceId?: number): Promise<void>;
  stopRecording(): Promise<void>;
  cancelRecording(): Promise<void>;
  getPipelineState(): Promise<PipelineState>;

  // --- Permissions ---
  checkPermissions(): Promise<PermissionStatus>;
  requestPermission(permissionType: 'microphone' | 'accessibility'): Promise<void>;

  // --- History / Transcriptions ---
  getTranscriptions(opts?: { cursor?: string; query?: string; limit?: number }): Promise<PaginatedResult>;
  getRecording(recordingPath: string): Promise<string>;
  deleteTranscription(id: string): Promise<void>;
  deleteAllTranscriptions(): Promise<void>;
  getStatistics(days?: number): Promise<Statistics>;

  // --- Models ---
  listAvailableModels(): Promise<ModelInfo[]>;
  downloadModel(modelId: string): Promise<void>;
  deleteModel(modelId: string): Promise<void>;
  importModel(): Promise<boolean>;
  selectModel(modelId: string): Promise<void>;

  // --- Settings ---
  getSettings(): Promise<AppSettings>;
  updateSetting(key: string, value: unknown): Promise<void>;

  // --- Vocabulary ---
  getVocabulary(): Promise<VocabularyWord[]>;
  addVocabulary(word: string): Promise<string>;
  deleteVocabulary(id: string): Promise<void>;

  // --- Replacements ---
  getReplacements(): Promise<Replacement[]>;
  setReplacement(original: string, replacement: string): Promise<string>;
  deleteReplacement(id: string): Promise<void>;

  // --- Prompts ---
  listPrompts(): Promise<Prompt[]>;
  addPrompt(name: string, systemMsg: string, userMsg: string): Promise<string>;
  updatePrompt(id: string, name: string, systemMsg: string, userMsg: string): Promise<void>;
  deletePrompt(id: string): Promise<void>;
  selectPrompt(id: string): Promise<void>;

  // --- API Keys ---
  storeApiKey(provider: string, key: string): Promise<void>;
  getApiKey(provider: string): Promise<string | null>;
  deleteApiKey(provider: string): Promise<void>;

  // --- Hotkey ---
  registerHotkey(shortcut: string): Promise<void>;
  unregisterHotkey(): Promise<void>;

  // --- Dictionary CSV Import / Export ---
  importDictionaryCsv(path: string, dictType: 'vocabulary' | 'replacements'): Promise<void>;
  exportDictionaryCsv(path: string, dictType: 'vocabulary' | 'replacements'): Promise<void>;
  importDictionaryCsvDialog(dictType: 'vocabulary' | 'replacements'): Promise<void>;
  exportDictionaryCsvDialog(dictType: 'vocabulary' | 'replacements'): Promise<void>;

  // --- Legacy Import ---
  detectVoiceinkLegacyPath(): Promise<string | null>;
  importVoiceinkLegacy(storePath: string): Promise<ImportResult>;
  importVoiceinkFromDialog(): Promise<ImportResult>;

  // --- MLX Daemon ---
  daemonStart(): Promise<void>;
  daemonStop(): Promise<void>;
  daemonStatus(): Promise<DaemonStatus>;
  daemonCheckDeps(): Promise<unknown>;
  daemonLoadModel(modelRepo: string): Promise<void>;
  daemonUnloadModel(): Promise<void>;

  // --- Sprites ---
  listSprites(): Promise<SpriteManifest[]>;
  getSpriteImage(dirId: string, fileName: string): Promise<string>;
  importSpriteFolder(): Promise<SpriteManifest | null>;
  importSpriteZip(): Promise<SpriteManifest | null>;
  deleteSprite(dirId: string): Promise<void>;
  processSpriteBackground(dirId: string, threshold: number): Promise<void>;

  // --- System ---
  getSystemLocale(): Promise<string>;

  // --- Frontend Logging ---
  frontendLog(level: string, message: string): Promise<void>;

  // ---------------------------------------------------------------------------
  // Event subscriptions — return an unsubscribe cleanup function
  // ---------------------------------------------------------------------------

  onRecordingState(callback: RecordingStateCallback): Unsubscribe;
  onDownloadProgress(callback: DownloadProgressCallback): Unsubscribe;
  onDaemonSetupStatus(callback: DaemonSetupCallback): Unsubscribe;
  onEscapePressed(callback: EscapePressedCallback): Unsubscribe;
  onToggleRecording(callback: ToggleRecordingCallback): Unsubscribe;
  onStatsUpdated(callback: StatsUpdatedCallback): Unsubscribe;
  onLanguageChanged(callback: LanguageChangedCallback): Unsubscribe;
}
