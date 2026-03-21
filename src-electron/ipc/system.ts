import { ipcMain, app, dialog } from "electron";
import { getAddon } from "../addon";
import {
  registerGlobalShortcut,
  unregisterAllShortcuts,
} from "../shortcuts";

export function registerSystemHandlers(): void {
  // --- Permissions ---
  ipcMain.handle("check-permissions", () => {
    return { microphone: true, accessibility: true };
  });

  ipcMain.handle("request-permission", () => {
    // no-op in Electron (OS-level permissions)
  });

  // --- Pipeline state ---
  ipcMain.handle("get-pipeline-state", () => {
    return JSON.parse(getAddon().getPipelineState());
  });

  // --- Hotkeys ---
  ipcMain.handle("register-hotkey", (_e, args?: { shortcut?: string }) => {
    if (args?.shortcut) {
      // Persist to settings
      getAddon().updateSetting("hotkey", JSON.stringify(args.shortcut));
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
        const level = args.level === "error" ? "error" : "info";
        console[level](`[frontend] ${args.message}`);
      }
    },
  );

  // --- Locale ---
  ipcMain.handle("get-system-locale", () => app.getLocale());

  // --- Legacy import ---
  ipcMain.handle("detect-voiceink-legacy-path", () => {
    return getAddon().detectVoiceinkLegacyPath();
  });

  ipcMain.handle(
    "import-voiceink-legacy",
    (_e, args?: { storePath?: string }) => {
      if (args?.storePath) {
        const json = getAddon().importVoiceinkLegacy(args.storePath);
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
    const json = getAddon().importVoiceinkLegacy(result.filePaths[0]);
    return JSON.parse(json);
  });
}
