import { ipcMain, app, dialog } from "electron";
import log from "../logger";
import { getAddon } from "../addon";
import {
  registerGlobalShortcut,
  unregisterAllShortcuts,
} from "../shortcuts";

export function registerSystemHandlers(): void {
  // --- Permissions ---
  ipcMain.handle("check-permissions", async () => {
    return JSON.parse(await getAddon().checkPermissions());
  });

  ipcMain.handle(
    "request-permission",
    (_e, args?: { permissionType?: string }) => {
      if (args?.permissionType) {
        getAddon().requestPermission(args.permissionType);
      }
    },
  );

  // --- Pipeline state ---
  ipcMain.handle("get-pipeline-state", () => {
    return JSON.parse(getAddon().getPipelineState());
  });

  // --- Hotkeys ---
  ipcMain.handle("register-hotkey", async (_e, args?: { shortcut?: string }) => {
    if (args?.shortcut) {
      // Persist to settings
      await getAddon().updateSetting("hotkey", JSON.stringify(args.shortcut));
      // Register global shortcut
      unregisterAllShortcuts();
      registerGlobalShortcut(args.shortcut);
    }
  });

  ipcMain.handle("unregister-hotkey", () => {
    unregisterAllShortcuts();
  });

  // --- Frontend logging ---
  ipcMain.handle(
    "frontend-log",
    (_e, args?: { level?: string; message?: string }) => {
      if (args?.message) {
        if (args.level === "error") {
          log.error(`[frontend] ${args.message}`);
        } else {
          log.info(`[frontend] ${args.message}`);
        }
      }
    },
  );

  // --- Locale ---
  ipcMain.handle("get-system-locale", () => app.getLocale());

  // --- Legacy import ---
  ipcMain.handle("detect-voiceink-legacy-path", async () => {
    return await getAddon().detectVoiceinkLegacyPath();
  });

  ipcMain.handle(
    "import-voiceink-legacy",
    async (_e, args?: { storePath?: string }) => {
      if (args?.storePath) {
        const json = await getAddon().importVoiceinkLegacy(args.storePath);
        return JSON.parse(json);
      }
      return null;
    },
  );

  ipcMain.handle("import-voiceink-from-dialog", async () => {
    const result = await dialog.showOpenDialog({
      title: "Select VoiceInk Database",
      filters: [{ name: "VoiceInk Database", extensions: ["store"] }],
      properties: ["openFile"],
    });
    if (result.canceled || result.filePaths.length === 0) {
      return null;
    }
    const json = await getAddon().importVoiceinkLegacy(result.filePaths[0]);
    return JSON.parse(json);
  });
}
