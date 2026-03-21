import { ipcMain, globalShortcut } from "electron";
import log from "../logger";
import { getAddon } from "../addon";
import {
  showRecorder,
  hideRecorder,
  getMainWindow,
  getRecorderWindow,
} from "../windows";

let lastEscTime = 0;

export function registerAudioHandlers(): void {
  ipcMain.handle("list-audio-devices", async () => {
    return await getAddon().listAudioDevices();
  });

  ipcMain.handle("start-recording", (_e, args?: { deviceId?: number }) => {
    const result = getAddon().startRecording(args?.deviceId ?? null);
    showRecorder();

    // Register Escape: double-press within 500ms cancels recording
    lastEscTime = 0;
    globalShortcut.register("Escape", () => {
      const now = Date.now();
      if (now - lastEscTime < 500) {
        // Double-press: cancel recording
        log.info("[audio] Escape double-press, cancelling recording");
        globalShortcut.unregister("Escape");
        getAddon().cancelRecording();
        hideRecorder();
        emitToRenderers("recording-state", { state: "idle" });
        emitToRenderers("escape-hint", "cancelled");
        lastEscTime = 0;
      } else {
        // First press: show hint
        log.info("[audio] Escape pressed, waiting for double-press");
        emitToRenderers("escape-hint", "pressAgain");
        lastEscTime = now;
      }
    });

    const state = JSON.parse(result);
    emitToRenderers("recording-state", state);
    return state;
  });

  ipcMain.handle("stop-recording", async () => {
    globalShortcut.unregister("Escape");
    emitToRenderers("recording-state", { state: "processing" });
    try {
      const resultJson = await getAddon().stopRecording();
      const result = JSON.parse(resultJson);
      hideRecorder();
      emitToRenderers("recording-state", { state: "idle" });
      emitToRenderers("transcription-result", result);
      return result;
    } catch (err) {
      hideRecorder();
      emitToRenderers("recording-state", { state: "idle" });
      throw err;
    }
  });

  ipcMain.handle("cancel-recording", () => {
    globalShortcut.unregister("Escape");
    getAddon().cancelRecording();
    hideRecorder();
    emitToRenderers("recording-state", { state: "idle" });
  });
}

/** Send an event to all renderer windows (main + recorder). */
function emitToRenderers(channel: string, data: unknown): void {
  for (const win of [getMainWindow(), getRecorderWindow()]) {
    if (win && !win.isDestroyed()) {
      win.webContents.send(channel, data);
    }
  }
}
