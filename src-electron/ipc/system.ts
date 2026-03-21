import { ipcMain, app } from "electron";

export function registerSystemHandlers(): void {
  // --- Permissions ---
  ipcMain.handle("check-permissions", () => {
    return { microphone: true, accessibility: true };
  });

  ipcMain.handle("request-permission", () => {
    // no-op in Electron
  });

  // --- Pipeline state ---
  ipcMain.handle("get-pipeline-state", () => {
    return { state: "idle" };
  });

  // --- Hotkeys (no-op in Electron) ---
  ipcMain.handle("register-hotkey", () => {
    // no-op: hotkey registration not supported in Electron shell
  });

  ipcMain.handle("unregister-hotkey", () => {
    // no-op
  });

  // --- Frontend logging ---
  ipcMain.handle("frontend-log", (_e, args?: { level?: string; message?: string }) => {
    if (args?.message) {
      const level = args.level === "error" ? "error" : "info";
      console[level](`[frontend] ${args.message}`);
    }
  });

  // --- Locale ---
  ipcMain.handle("get-system-locale", () => app.getLocale());

  // --- Legacy import ---
  ipcMain.handle("detect-voiceink-legacy-path", () => null);

  ipcMain.handle("import-voiceink-legacy", () => {
    throw new Error("Legacy import not available in Electron");
  });

  ipcMain.handle("import-voiceink-from-dialog", () => {
    throw new Error("Legacy import not available in Electron");
  });
}
