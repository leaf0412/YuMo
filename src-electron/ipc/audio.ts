import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerAudioHandlers(): void {
  ipcMain.handle("list-audio-devices", () => {
    return getAddon().listAudioDevices();
  });

  // --- Recording pipeline (not available in Electron yet) ---
  ipcMain.handle("start-recording", () => {
    return null;
  });

  ipcMain.handle("stop-recording", () => {
    return null;
  });

  ipcMain.handle("cancel-recording", () => {
    return null;
  });
}
