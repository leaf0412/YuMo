/**
 * Native addon loader — singleton for the yumo-napi addon.
 *
 * Import `getAddon` in IPC modules to access the napi interface.
 */
import { app } from "electron";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const require = createRequire(import.meta.url);

// ---------------------------------------------------------------------------
// Type definition
// ---------------------------------------------------------------------------

export type NapiAddon = {
  // Initialization (sync — one-time startup)
  init(dataDir: string): void;

  // Audio devices
  listAudioDevices(): Promise<Array<{ id: number; name: string; isDefault: boolean }>>;

  // Settings
  getAllSettings(): Promise<string>;
  updateSetting(key: string, value: string): Promise<void>;

  // Transcriptions
  getTranscriptions(cursor: string | null, query: string | null, limit: number | null): Promise<string>;
  deleteTranscription(id: string): Promise<void>;
  deleteAllTranscriptions(): Promise<void>;

  // Models
  listAvailableModels(): Promise<string>;

  // Statistics
  getStatistics(days: number | null): Promise<string>;

  // Vocabulary
  getVocabulary(): Promise<string>;
  addVocabulary(word: string): Promise<string>;
  deleteVocabulary(id: string): Promise<void>;

  // Replacements
  getReplacements(): Promise<string>;
  setReplacement(original: string, replacement: string): Promise<string>;
  deleteReplacement(id: string): Promise<void>;

  // Prompts
  listPrompts(): Promise<string>;
  addPrompt(name: string, systemMsg: string, userMsg: string): Promise<string>;
  updatePrompt(id: string, name: string, systemMsg: string, userMsg: string): Promise<void>;
  deletePrompt(id: string): Promise<void>;

  // CSV Import/Export
  importDictionaryCsv(path: string, dictType: string): Promise<void>;
  exportDictionaryCsv(path: string, dictType: string): Promise<void>;

  // Keychain
  storeApiKey(provider: string, key: string): Promise<void>;
  getApiKey(provider: string): Promise<string | null>;
  deleteApiKey(provider: string): Promise<void>;

  // Daemon (sync reads, async operations)
  daemonStatus(): { running: boolean; loadedModel: string | null };
  daemonStart(): Promise<void>;
  daemonStop(): void;
  daemonLoadModel(modelRepo: string): Promise<void>;
  daemonUnloadModel(): Promise<void>;
  daemonCheckDeps(): Promise<boolean>;

  // Recording pipeline (startRecording stays sync — creates handle on napi thread)
  startRecording(deviceId: number | null): string;
  stopRecording(): Promise<string>;
  cancelRecording(): void;
  getPipelineState(): string;

  // Recording playback
  getRecording(recordingPath: string): Promise<string>;

  // Model download/delete
  downloadModel(modelId: string): Promise<void>;
  deleteModel(modelId: string): Promise<void>;

  // Sprites
  listSprites(): Promise<string>;
  getSpriteImage(dirId: string, fileName: string): Promise<string>;
  importSpriteFolder(path: string): Promise<string>;
  importSpriteZip(zipPath: string): Promise<string>;
  deleteSprite(dirId: string): Promise<void>;
  processSpriteBackground(dirId: string, threshold: number): Promise<void>;

  // Legacy import
  detectVoiceinkLegacyPath(): Promise<string | null>;
  importVoiceinkLegacy(storePath: string): Promise<string>;
};

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

function loadAddon(): NapiAddon {
  const platform = process.platform === "darwin" ? "darwin" : process.platform === "win32" ? "win32" : "linux";
  const arch = process.arch === "arm64" ? "arm64" : "x64";
  const addonName = `yumo-napi.${platform}-${arch}.node`;

  const paths = [
    join(process.resourcesPath ?? "", "napi", addonName),
    join(__dirname, "../../napi", addonName),
    // Fallback: generic name
    join(process.resourcesPath ?? "", "yumo-napi.node"),
    join(__dirname, "../../napi/yumo-napi.node"),
  ];

  for (const addonPath of paths) {
    try {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      return require(addonPath) as NapiAddon;
    } catch {
      // try next path
    }
  }
  throw new Error(
    `Failed to load yumo-napi addon. Searched: ${paths.join(", ")}`,
  );
}

// ---------------------------------------------------------------------------
// Singleton accessor
// ---------------------------------------------------------------------------

let addon: NapiAddon | null = null;

export function getAddon(): NapiAddon {
  if (!addon) {
    addon = loadAddon();
    // Initialize with default data directory (~/.voiceink)
    const dataDir = join(app.getPath("home"), ".voiceink");
    addon.init(dataDir);
  }
  return addon;
}
