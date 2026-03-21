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
  getTranscriptions(
    cursor: string | null,
    query: string | null,
    limit: number | null,
  ): string;
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

  // --- Stub handlers for commands not yet wired to napi ---
  // These return sensible defaults so the UI can render without crashing.

  ipcMain.handle("check-permissions", () => {
    return { microphone: true, accessibility: true };
  });

  ipcMain.handle("frontend-log", () => {
    // no-op: Electron logs go to stdout
  });

  ipcMain.handle("get-statistics", () => {
    return {
      total_sessions: 0,
      total_words: 0,
      total_duration_seconds: 0,
      total_keystrokes_saved: 0,
      time_saved_minutes: 0,
      avg_wpm: 0,
      daily_wpm: [],
      wpm_stats: { avg: 0, max: 0, min: 0 },
    };
  });

  ipcMain.handle("list-available-models", () => {
    return [];
  });

  ipcMain.handle("daemon-status", () => {
    return { running: false, loaded_model: null };
  });

  ipcMain.handle("detect-voiceink-legacy-path", () => {
    return null;
  });

  ipcMain.handle("list-sprites", () => {
    return [];
  });

  ipcMain.handle("get-system-locale", () => {
    return app.getLocale();
  });

  ipcMain.handle("update-setting", () => {
    // TODO: wire to napi
  });

  ipcMain.handle("request-permission", () => {
    // No-op on Electron
  });

  // Catch-all for unregistered commands — prevents crashes
  ipcMain.handle("__unhandled__", () => null);
}
