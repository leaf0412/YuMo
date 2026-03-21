import { app, BrowserWindow } from "electron";
import { registerAllHandlers } from "./ipc/index";
import { createMainWindow, createRecorderWindow } from "./windows";
import { createTray } from "./tray";
import { registerGlobalShortcut } from "./shortcuts";
import { getAddon } from "./addon";

// -------------------------------------------------------------------------
// App lifecycle
// -------------------------------------------------------------------------

app.whenReady().then(() => {
  registerAllHandlers();
  createMainWindow();
  createRecorderWindow(); // pre-create hidden, ready for recording
  createTray();

  // Restore saved hotkey from settings
  restoreSavedHotkey();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});

// -------------------------------------------------------------------------
// Hotkey restoration
// -------------------------------------------------------------------------

function restoreSavedHotkey(): void {
  try {
    const settingsJson = getAddon().getAllSettings();
    const settings = JSON.parse(settingsJson);
    const hotkey = settings.hotkey;
    if (typeof hotkey === "string" && hotkey.length > 0) {
      const success = registerGlobalShortcut(hotkey);
      if (success) {
        console.log(`[main] restored hotkey: ${hotkey}`);
      }
    }
  } catch (err) {
    console.error("[main] failed to restore hotkey:", err);
  }
}
