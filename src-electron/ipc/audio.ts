import { ipcMain, globalShortcut, clipboard } from "electron";
import { exec } from "child_process";
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

  // Register device change listener (macOS: CoreAudio callback)
  try {
    getAddon().registerDeviceChangeCallback((devicesJson: string) => {
      try {
        const devices = JSON.parse(devicesJson);
        log.info("[audio] devices changed, notifying renderers");
        emitToRenderers("devices-changed", devices);
      } catch (e) {
        log.warn("[audio] failed to parse device change payload:", e);
      }
    });
  } catch (e) {
    log.info("[audio] device change listener not available:", e);
  }

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
      // Linux: write clipboard via Electron API, then try xdotool auto-paste.
      // Clipboard always works (Chromium maintains X11 selection ownership).
      // xdotool is best-effort — if not installed, user Ctrl+V manually.
      if (process.platform === "linux" && result.text) {
        clipboard.writeText(result.text);
        log.info("[audio] Linux: wrote transcription to clipboard via Electron");
        // Small delay for clipboard to settle, then simulate Ctrl+V
        setTimeout(() => {
          exec("xdotool key --clearmodifiers ctrl+v", (err) => {
            if (err) {
              log.info("[audio] Linux: xdotool auto-paste unavailable, user can Ctrl+V");
            } else {
              log.info("[audio] Linux: xdotool auto-paste succeeded");
            }
          });
        }, 100);
      }
      hideRecorder();
      emitToRenderers("recording-state", { state: "idle" });
      emitToRenderers("transcription-result", result);
      if (result.paste_error) {
        log.warn("[audio] paste failed:", result.paste_error);
        emitToRenderers("paste-failed", { error: result.paste_error });
      }
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
