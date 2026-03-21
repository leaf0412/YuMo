import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerAudioHandlers(): void {
  ipcMain.handle("list-audio-devices", () => {
    return getAddon().listAudioDevices();
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
}
