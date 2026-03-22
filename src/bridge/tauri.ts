/**
 * Tauri implementation of the Bridge interface.
 *
 * Maps every Bridge method to the corresponding Tauri backend command
 * (snake_case) using `invoke`, and wraps Tauri events with `listen`.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, emit, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  Bridge,
  AudioInputDevice,
  PermissionStatus,
  ModelInfo,
  PaginatedResult,
  Statistics,
  VocabularyWord,
  Replacement,
  Prompt,
  ImportResult,
  DaemonStatus,
  SpriteManifest,
  AppSettings,
  PipelineState,
  RecordingStateCallback,
  DownloadProgressCallback,
  DaemonSetupCallback,
  EscapePressedCallback,
  ToggleRecordingCallback,
  StatsUpdatedCallback,
  LanguageChangedCallback,
  Unsubscribe,
} from './types';

// ---------------------------------------------------------------------------
// Helper: wrap a Tauri `listen` call into a synchronous Unsubscribe handle.
// The actual unlisten is async internally but the returned function is sync.
// ---------------------------------------------------------------------------
function makeListen<T>(
  event: string,
  handler: (payload: T) => void,
): Unsubscribe {
  let unlistenFn: UnlistenFn | null = null;
  const promise = listen<T>(event, (e) => handler(e.payload));
  promise.then((fn) => {
    unlistenFn = fn;
  });
  return () => {
    if (unlistenFn) {
      unlistenFn();
    } else {
      // If not resolved yet, wait and then unlisten
      promise.then((fn) => fn());
    }
  };
}

// ---------------------------------------------------------------------------
// Tauri Bridge implementation
// ---------------------------------------------------------------------------
export const tauriBridge: Bridge = {
  // --- Recording pipeline ---

  listAudioDevices(): Promise<AudioInputDevice[]> {
    return invoke('list_audio_devices');
  },

  startRecording(deviceId?: number): Promise<void> {
    return invoke('start_recording', deviceId !== undefined ? { deviceId } : {});
  },

  stopRecording(): Promise<void> {
    return invoke('stop_recording');
  },

  cancelRecording(): Promise<void> {
    return invoke('cancel_recording');
  },

  getPipelineState(): Promise<PipelineState> {
    return invoke('get_pipeline_state');
  },

  // --- Permissions ---

  checkPermissions(): Promise<PermissionStatus> {
    return invoke('check_permissions');
  },

  requestPermission(permissionType: 'microphone' | 'accessibility'): Promise<void> {
    return invoke('request_permission', { permissionType });
  },

  // --- History / Transcriptions ---

  getTranscriptions(opts = {}): Promise<PaginatedResult> {
    const { cursor, query, limit } = opts;
    return invoke('get_transcriptions', {
      cursor: cursor ?? null,
      query: query ?? null,
      limit: limit ?? 20,
    });
  },

  getRecording(recordingPath: string): Promise<string> {
    return invoke('get_recording', { recordingPath });
  },

  deleteTranscription(id: string): Promise<void> {
    return invoke('delete_transcription', { id });
  },

  deleteAllTranscriptions(): Promise<void> {
    return invoke('delete_all_transcriptions');
  },

  getStatistics(days?: number): Promise<Statistics> {
    return invoke('get_statistics', { days: days ?? null });
  },

  // --- Models ---

  listAvailableModels(): Promise<ModelInfo[]> {
    return invoke('list_available_models');
  },

  downloadModel(modelId: string): Promise<void> {
    return invoke('download_model', { modelId });
  },

  deleteModel(modelId: string): Promise<void> {
    return invoke('delete_model', { modelId });
  },

  importModel(): Promise<boolean> {
    return invoke('import_model');
  },

  selectModel(modelId: string): Promise<void> {
    return invoke('select_model', { modelId });
  },

  // --- Settings ---

  getSettings(): Promise<AppSettings> {
    return invoke('get_settings');
  },

  updateSetting(key: string, value: unknown): Promise<void> {
    return invoke('update_setting', { key, value });
  },

  // --- Vocabulary ---

  getVocabulary(): Promise<VocabularyWord[]> {
    return invoke('get_vocabulary');
  },

  addVocabulary(word: string): Promise<string> {
    return invoke('add_vocabulary', { word });
  },

  deleteVocabulary(id: string): Promise<void> {
    return invoke('delete_vocabulary', { id });
  },

  // --- Replacements ---

  getReplacements(): Promise<Replacement[]> {
    return invoke('get_replacements');
  },

  setReplacement(original: string, replacement: string): Promise<string> {
    return invoke('set_replacement', { original, replacement });
  },

  deleteReplacement(id: string): Promise<void> {
    return invoke('delete_replacement', { id });
  },

  // --- Prompts ---

  listPrompts(): Promise<Prompt[]> {
    return invoke('list_prompts');
  },

  addPrompt(name: string, systemMsg: string, userMsg: string): Promise<string> {
    return invoke('add_prompt', { name, systemMsg, userMsg });
  },

  updatePrompt(id: string, name: string, systemMsg: string, userMsg: string): Promise<void> {
    return invoke('update_prompt', { id, name, systemMsg, userMsg });
  },

  deletePrompt(id: string): Promise<void> {
    return invoke('delete_prompt', { id });
  },

  selectPrompt(id: string): Promise<void> {
    return invoke('select_prompt', { id });
  },

  // --- API Keys ---

  storeApiKey(provider: string, key: string): Promise<void> {
    return invoke('store_api_key', { provider, key });
  },

  getApiKey(provider: string): Promise<string | null> {
    return invoke('get_api_key', { provider });
  },

  deleteApiKey(provider: string): Promise<void> {
    return invoke('delete_api_key', { provider });
  },

  // --- Hotkey ---

  registerHotkey(shortcut: string): Promise<void> {
    return invoke('register_hotkey', { shortcut });
  },

  unregisterHotkey(): Promise<void> {
    return invoke('unregister_hotkey');
  },

  // --- Dictionary CSV Import / Export ---

  importDictionaryCsv(path: string, dictType: 'vocabulary' | 'replacements'): Promise<void> {
    return invoke('import_dictionary_csv', { path, dictType });
  },

  exportDictionaryCsv(path: string, dictType: 'vocabulary' | 'replacements'): Promise<void> {
    return invoke('export_dictionary_csv', { path, dictType });
  },

  importDictionaryCsvDialog(dictType: 'vocabulary' | 'replacements'): Promise<void> {
    return invoke('import_dictionary_csv_dialog', { dictType });
  },

  exportDictionaryCsvDialog(dictType: 'vocabulary' | 'replacements'): Promise<void> {
    return invoke('export_dictionary_csv_dialog', { dictType });
  },

  // --- Legacy Import ---

  detectVoiceinkLegacyPath(): Promise<string | null> {
    return invoke('detect_voiceink_legacy_path');
  },

  importVoiceinkLegacy(storePath: string): Promise<ImportResult> {
    return invoke('import_voiceink_legacy', { storePath });
  },

  importVoiceinkFromDialog(): Promise<ImportResult> {
    return invoke('import_voiceink_from_dialog');
  },

  // --- MLX Daemon ---

  daemonStart(): Promise<void> {
    return invoke('daemon_start');
  },

  daemonStop(): Promise<void> {
    return invoke('daemon_stop');
  },

  daemonStatus(): Promise<DaemonStatus> {
    return invoke('daemon_status');
  },

  daemonCheckDeps(): Promise<unknown> {
    return invoke('daemon_check_deps');
  },

  daemonLoadModel(modelRepo: string): Promise<void> {
    return invoke('daemon_load_model', { modelRepo });
  },

  daemonUnloadModel(): Promise<void> {
    return invoke('daemon_unload_model');
  },

  // --- Sprites ---

  listSprites(): Promise<SpriteManifest[]> {
    return invoke('list_sprites');
  },

  getSpriteImage(dirId: string, fileName: string): Promise<string> {
    return invoke('get_sprite_image', { dirId, fileName });
  },

  importSpriteFolder(): Promise<SpriteManifest | null> {
    return invoke<SpriteManifest | null>('import_sprite_folder').then(
      (v) => v ?? null,
    );
  },

  importSpriteZip(): Promise<SpriteManifest | null> {
    return invoke<SpriteManifest | null>('import_sprite_zip').then(
      (v) => v ?? null,
    );
  },

  deleteSprite(dirId: string): Promise<void> {
    return invoke('delete_sprite', { dirId });
  },

  processSpriteBackground(dirId: string, threshold: number): Promise<void> {
    return invoke('process_sprite_background', { dirId, threshold });
  },

  // --- System ---

  getSystemLocale(): Promise<string> {
    return invoke('get_system_locale');
  },

  // --- Frontend Logging ---

  frontendLog(level: string, message: string): Promise<void> {
    return invoke('frontend_log', { level, message });
  },

  // ---------------------------------------------------------------------------
  // Event subscriptions
  // ---------------------------------------------------------------------------

  onRecordingState(callback: RecordingStateCallback): Unsubscribe {
    return makeListen<PipelineState>('recording-state', callback);
  },

  onDownloadProgress(callback: DownloadProgressCallback): Unsubscribe {
    return makeListen('model-download-progress', callback);
  },

  onDaemonSetupStatus(callback: DaemonSetupCallback): Unsubscribe {
    return makeListen('daemon-setup-status', callback);
  },

  onEscapePressed(callback: EscapePressedCallback): Unsubscribe {
    return makeListen<void>('escape-pressed', () => callback());
  },

  onToggleRecording(callback: ToggleRecordingCallback): Unsubscribe {
    return makeListen<void>('toggle-recording', () => callback());
  },

  onStatsUpdated(callback: StatsUpdatedCallback): Unsubscribe {
    return makeListen<void>('stats-updated', () => callback());
  },

  onLanguageChanged(callback: LanguageChangedCallback): Unsubscribe {
    return makeListen<string>('language-changed', callback);
  },
};

// Re-export emit for the rare cases where frontend needs to fire events
// (e.g., History page emits 'stats-updated' after deleting a record).
export { emit };
