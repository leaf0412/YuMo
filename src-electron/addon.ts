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
  // Initialization
  init(dataDir: string): void;

  // Audio devices
  listAudioDevices(): Array<{ id: number; name: string; isDefault: boolean }>;

  // Settings
  getAllSettings(): string;
  updateSetting(key: string, value: string): void;

  // Transcriptions
  getTranscriptions(cursor: string | null, query: string | null, limit: number | null): string;
  deleteTranscription(id: string): void;
  deleteAllTranscriptions(): void;

  // Models
  listAvailableModels(): string;

  // Statistics
  getStatistics(days: number | null): string;

  // Vocabulary
  getVocabulary(): string;
  addVocabulary(word: string): string;
  deleteVocabulary(id: string): void;

  // Replacements
  getReplacements(): string;
  setReplacement(original: string, replacement: string): string;
  deleteReplacement(id: string): void;

  // Prompts
  listPrompts(): string;
  addPrompt(name: string, systemMsg: string, userMsg: string): string;
  updatePrompt(id: string, name: string, systemMsg: string, userMsg: string): void;
  deletePrompt(id: string): void;

  // CSV Import/Export
  importDictionaryCsv(path: string, dictType: string): void;
  exportDictionaryCsv(path: string, dictType: string): void;

  // Keychain
  storeApiKey(provider: string, key: string): void;
  getApiKey(provider: string): string | null;
  deleteApiKey(provider: string): void;

  // Daemon
  daemonStatus(): { running: boolean; loadedModel: string | null };
  daemonStart(): Promise<void>;
  daemonStop(): void;
  daemonLoadModel(modelRepo: string): Promise<void>;
  daemonUnloadModel(): Promise<void>;
  daemonCheckDeps(): boolean;

  // Recording pipeline
  startRecording(deviceId: number | null): string;
  stopRecording(): Promise<string>;
  cancelRecording(): void;
  getPipelineState(): string;

  // Recording playback
  getRecording(recordingPath: string): string;

  // Model download/delete
  downloadModel(modelId: string): Promise<void>;
  deleteModel(modelId: string): void;

  // Sprites
  listSprites(): string;
  getSpriteImage(dirId: string, fileName: string): string;
  importSpriteFolder(path: string): string;
  importSpriteZip(zipPath: string): string;
  deleteSprite(dirId: string): void;

  // Legacy import
  detectVoiceinkLegacyPath(): string | null;
  importVoiceinkLegacy(storePath: string): string;
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
