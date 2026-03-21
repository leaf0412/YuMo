/**
 * IPC handler registration — maps Electron IPC channels to napi addon calls.
 *
 * The napi addon is loaded at import time. In production it resolves from
 * the app resources; in development it resolves from the napi build output.
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
  init(dataDir: string): void;
  listAudioDevices(): Array<{ id: number; name: string; isDefault: boolean }>;
  getAllSettings(): string;
  getTranscriptions(cursor: string | null, query: string | null, limit: number | null): string;
  listAvailableModels(): string;
  updateSetting(key: string, value: string): void;
  getStatistics(days: number | null): string;
  getVocabulary(): string;
  getReplacements(): string;
  storeApiKey(provider: string, key: string): void;
  getApiKey(provider: string): string | null;
  deleteApiKey(provider: string): void;
};

function loadAddon(): NapiAddon {
  // Try production path first, then development path
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
    // Initialize with default data directory
    const dataDir = join(app.getPath("userData"), "yumo-data");
    addon.init(dataDir);
  }
  return addon;
}

// ---------------------------------------------------------------------------
// Register handlers
// ---------------------------------------------------------------------------

export function registerIpcHandlers(): void {
  // --- napi addon handlers ---
  ipcMain.handle("list-audio-devices", () => {
    return getAddon().listAudioDevices();
  });

  ipcMain.handle("get-all-settings", () => {
    const json = getAddon().getAllSettings();
    return JSON.parse(json);
  });

  ipcMain.handle("get-settings", () => {
    const json = getAddon().getAllSettings();
    return JSON.parse(json);
  });

  ipcMain.handle(
    "get-transcriptions",
    (
      _e,
      args?: { cursor?: string; query?: string; limit?: number },
    ) => {
      const json = getAddon().getTranscriptions(
        args?.cursor ?? null,
        args?.query ?? null,
        args?.limit ?? null,
      );
      return JSON.parse(json);
    },
  );

  // --- Wired to napi addon ---

  ipcMain.handle("list-available-models", () => {
    return JSON.parse(getAddon().listAvailableModels());
  });

  ipcMain.handle("update-setting", (_e, args?: { key?: string; value?: unknown }) => {
    if (args?.key) {
      const val = typeof args.value === 'string' ? args.value : JSON.stringify(args.value);
      getAddon().updateSetting(args.key, val);
    }
  });

  ipcMain.handle("get-statistics", (_e, args?: { days?: number }) => {
    return JSON.parse(getAddon().getStatistics(args?.days ?? null));
  });

  ipcMain.handle("get-vocabulary", () => {
    return JSON.parse(getAddon().getVocabulary());
  });

  ipcMain.handle("get-replacements", () => {
    return JSON.parse(getAddon().getReplacements());
  });

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

  // --- Platform stubs (no napi needed) ---

  ipcMain.handle("check-permissions", () => {
    return { microphone: true, accessibility: true };
  });

  ipcMain.handle("frontend-log", (_e, args?: { level?: string; message?: string }) => {
    if (args?.message) {
      const level = args.level === 'error' ? 'error' : 'info';
      console[level](`[frontend] ${args.message}`);
    }
  });

  ipcMain.handle("daemon-status", () => {
    return { running: false, loaded_model: null };
  });

  ipcMain.handle("detect-voiceink-legacy-path", () => null);

  ipcMain.handle("list-sprites", () => []);

  ipcMain.handle("get-system-locale", () => app.getLocale());

  ipcMain.handle("request-permission", () => {});
}
