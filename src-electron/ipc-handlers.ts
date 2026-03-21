/**
 * IPC handler registration — maps Electron IPC channels to napi addon calls.
 *
 * The napi addon is loaded at import time. In production it resolves from
 * the app resources; in development it resolves from the napi build output.
 *
 * All 54 Tauri commands are covered:
 *   - Category 1: Already in napi — wired directly
 *   - Category 2: New napi functions added for DB ops — wired directly
 *   - Category 3: Tauri-specific (recording, hotkey, sprites, import) — stubbed
 */
import { ipcMain, app } from "electron";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const require = createRequire(import.meta.url);

// ---------------------------------------------------------------------------
// Load the native addon
// ---------------------------------------------------------------------------

type NapiAddon = {
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
  daemonStart(): void;
  daemonStop(): void;
  daemonLoadModel(modelRepo: string): void;
  daemonUnloadModel(): void;
  daemonCheckDeps(): boolean;
};

function loadAddon(): NapiAddon {
  // Platform-specific addon filenames
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

let addon: NapiAddon | null = null;

function getAddon(): NapiAddon {
  if (!addon) {
    addon = loadAddon();
    // Initialize with default data directory (~/.voiceink)
    const dataDir = join(app.getPath("home"), ".voiceink");
    addon.init(dataDir);
  }
  return addon;
}

// ---------------------------------------------------------------------------
// Register ALL IPC handlers (54 commands)
// ---------------------------------------------------------------------------

export function registerIpcHandlers(): void {
  // =======================================================================
  // Category 1 & 2: napi-backed handlers (data layer)
  // =======================================================================

  // --- Audio devices ---
  ipcMain.handle("list-audio-devices", () => {
    return getAddon().listAudioDevices();
  });

  // --- Settings ---
  ipcMain.handle("get-all-settings", () => {
    return JSON.parse(getAddon().getAllSettings());
  });

  ipcMain.handle("get-settings", () => {
    return JSON.parse(getAddon().getAllSettings());
  });

  ipcMain.handle("update-setting", (_e, args?: { key?: string; value?: unknown }) => {
    if (args?.key) {
      const val = typeof args.value === "string" ? args.value : JSON.stringify(args.value);
      getAddon().updateSetting(args.key, val);
    }
  });

  // --- Transcriptions ---
  ipcMain.handle(
    "get-transcriptions",
    (_e, args?: { cursor?: string; query?: string; limit?: number }) => {
      const json = getAddon().getTranscriptions(
        args?.cursor ?? null,
        args?.query ?? null,
        args?.limit ?? null,
      );
      return JSON.parse(json);
    },
  );

  ipcMain.handle("delete-transcription", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deleteTranscription(args.id);
  });

  ipcMain.handle("delete-all-transcriptions", () => {
    getAddon().deleteAllTranscriptions();
  });

  // --- Models ---
  ipcMain.handle("list-available-models", () => {
    return JSON.parse(getAddon().listAvailableModels());
  });

  ipcMain.handle("select-model", (_e, args?: { modelId?: string }) => {
    if (args?.modelId) {
      getAddon().updateSetting("selected_model_id", JSON.stringify(args.modelId));
    }
  });

  // --- Statistics ---
  ipcMain.handle("get-statistics", (_e, args?: { days?: number }) => {
    return JSON.parse(getAddon().getStatistics(args?.days ?? null));
  });

  // --- Vocabulary ---
  ipcMain.handle("get-vocabulary", () => {
    return JSON.parse(getAddon().getVocabulary());
  });

  ipcMain.handle("add-vocabulary", (_e, args?: { word?: string }) => {
    if (args?.word) return getAddon().addVocabulary(args.word);
  });

  ipcMain.handle("delete-vocabulary", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deleteVocabulary(args.id);
  });

  // --- Replacements ---
  ipcMain.handle("get-replacements", () => {
    return JSON.parse(getAddon().getReplacements());
  });

  ipcMain.handle("set-replacement", (_e, args?: { original?: string; replacement?: string }) => {
    if (args?.original && args?.replacement) {
      return getAddon().setReplacement(args.original, args.replacement);
    }
  });

  ipcMain.handle("delete-replacement", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deleteReplacement(args.id);
  });

  // --- Prompts ---
  ipcMain.handle("list-prompts", () => {
    return JSON.parse(getAddon().listPrompts());
  });

  ipcMain.handle(
    "add-prompt",
    (_e, args?: { name?: string; systemMsg?: string; userMsg?: string }) => {
      if (args?.name && args?.systemMsg !== undefined && args?.userMsg !== undefined) {
        return getAddon().addPrompt(args.name, args.systemMsg, args.userMsg);
      }
    },
  );

  ipcMain.handle(
    "update-prompt",
    (_e, args?: { id?: string; name?: string; systemMsg?: string; userMsg?: string }) => {
      if (args?.id && args?.name !== undefined && args?.systemMsg !== undefined && args?.userMsg !== undefined) {
        getAddon().updatePrompt(args.id, args.name, args.systemMsg, args.userMsg);
      }
    },
  );

  ipcMain.handle("delete-prompt", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deletePrompt(args.id);
  });

  ipcMain.handle("select-prompt", (_e, args?: { id?: string }) => {
    if (args?.id) {
      getAddon().updateSetting("selected_prompt_id", JSON.stringify(args.id));
    }
  });

  // --- CSV Import/Export ---
  ipcMain.handle(
    "import-dictionary-csv",
    (_e, args?: { path?: string; dictType?: string }) => {
      if (args?.path && args?.dictType) {
        getAddon().importDictionaryCsv(args.path, args.dictType);
      }
    },
  );

  ipcMain.handle(
    "export-dictionary-csv",
    (_e, args?: { path?: string; dictType?: string }) => {
      if (args?.path && args?.dictType) {
        getAddon().exportDictionaryCsv(args.path, args.dictType);
      }
    },
  );

  // --- Keychain ---
  ipcMain.handle("store-api-key", (_e, args?: { provider?: string; key?: string }) => {
    if (args?.provider && args?.key) {
      getAddon().storeApiKey(args.provider, args.key);
    }
  });

  ipcMain.handle("get-api-key", (_e, args?: { provider?: string }) => {
    return args?.provider ? getAddon().getApiKey(args.provider) : null;
  });

  ipcMain.handle("delete-api-key", (_e, args?: { provider?: string }) => {
    if (args?.provider) getAddon().deleteApiKey(args.provider);
  });

  // --- Daemon management ---
  ipcMain.handle("daemon-status", () => {
    const s = getAddon().daemonStatus();
    return { running: s.running, loaded_model: s.loadedModel };
  });

  ipcMain.handle("daemon-start", () => {
    getAddon().daemonStart();
  });

  ipcMain.handle("daemon-stop", () => {
    getAddon().daemonStop();
  });

  ipcMain.handle("daemon-load-model", (_e, args?: { modelRepo?: string }) => {
    if (args?.modelRepo) getAddon().daemonLoadModel(args.modelRepo);
  });

  ipcMain.handle("daemon-unload-model", () => {
    getAddon().daemonUnloadModel();
  });

  ipcMain.handle("daemon-check-deps", () => {
    return getAddon().daemonCheckDeps();
  });

  // --- Frontend logging ---
  ipcMain.handle("frontend-log", (_e, args?: { level?: string; message?: string }) => {
    if (args?.message) {
      const level = args.level === "error" ? "error" : "info";
      console[level](`[frontend] ${args.message}`);
    }
  });

  // =======================================================================
  // Category 3: Platform stubs (no napi needed / not yet available)
  // =======================================================================

  // --- Permissions ---
  ipcMain.handle("check-permissions", () => {
    return { microphone: true, accessibility: true };
  });

  ipcMain.handle("request-permission", () => {
    // no-op in Electron
  });

  // --- Recording pipeline (not available in Electron yet) ---
  ipcMain.handle("start-recording", () => {
    throw new Error("Recording not available in Electron yet");
  });

  ipcMain.handle("stop-recording", () => {
    throw new Error("Recording not available in Electron yet");
  });

  ipcMain.handle("cancel-recording", () => {
    throw new Error("Recording not available in Electron yet");
  });

  ipcMain.handle("get-pipeline-state", () => {
    return { state: "idle" };
  });

  ipcMain.handle("get-recording", () => {
    throw new Error("Recording playback not available in Electron yet");
  });

  // --- Hotkeys (no-op in Electron) ---
  ipcMain.handle("register-hotkey", () => {
    // no-op: hotkey registration not supported in Electron shell
  });

  ipcMain.handle("unregister-hotkey", () => {
    // no-op
  });

  // --- Model download/delete/import ---
  ipcMain.handle("download-model", () => {
    throw new Error("Model download not available in Electron yet");
  });

  ipcMain.handle("delete-model", () => {
    throw new Error("Model deletion not available in Electron yet");
  });

  ipcMain.handle("import-model", () => {
    throw new Error("Model import not available in Electron yet");
  });

  // --- Legacy import ---
  ipcMain.handle("detect-voiceink-legacy-path", () => null);

  ipcMain.handle("import-voiceink-legacy", () => {
    throw new Error("Legacy import not available in Electron");
  });

  ipcMain.handle("import-voiceink-from-dialog", () => {
    throw new Error("Legacy import not available in Electron");
  });

  // --- Sprites ---
  ipcMain.handle("list-sprites", () => []);

  ipcMain.handle("get-sprite-image", () => {
    throw new Error("Sprite images not available in Electron yet");
  });

  ipcMain.handle("import-sprite-folder", () => {
    throw new Error("Sprite import not available in Electron yet");
  });

  ipcMain.handle("import-sprite-zip", () => {
    throw new Error("Sprite import not available in Electron yet");
  });

  ipcMain.handle("delete-sprite", () => {
    throw new Error("Sprite deletion not available in Electron yet");
  });

  ipcMain.handle("process-sprite-background", () => {
    throw new Error("Sprite processing not available in Electron yet");
  });

  // --- Locale ---
  ipcMain.handle("get-system-locale", () => app.getLocale());
}
