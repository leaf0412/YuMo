/**
 * IPC handler registration — maps Electron IPC channels to napi addon calls.
 *
 * The napi addon is loaded at import time. In production it resolves from
 * the app resources; in development it resolves from the napi build output.
 */
import { ipcMain, app } from "electron";
import { join } from "node:path";

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
  const paths = [
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
  ipcMain.handle("list-audio-devices", () => {
    return getAddon().listAudioDevices();
  });

  ipcMain.handle("get-all-settings", () => {
    const json = getAddon().getAllSettings();
    return JSON.parse(json);
  });

  ipcMain.handle(
    "get-transcriptions",
    (
      _e,
      cursor: string | null,
      query: string | null,
      limit: number | null,
    ) => {
      const json = getAddon().getTranscriptions(cursor, query, limit);
      return JSON.parse(json);
    },
  );
}
