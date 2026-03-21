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

  // Pre-warm daemon + load selected model in background (prevents first-recording cold start)
  warmupDaemon();

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

// -------------------------------------------------------------------------
// Daemon warmup — start daemon + load selected model in background
// -------------------------------------------------------------------------

async function warmupDaemon(): Promise<void> {
  try {
    const settingsJson = getAddon().getAllSettings();
    const settings = JSON.parse(settingsJson);
    const modelId = settings.selected_model_id;
    if (!modelId) {
      console.log("[main] no selected model, skipping daemon warmup");
      return;
    }

    // Find model info to get repo name
    const modelsJson = getAddon().listAvailableModels();
    const models = JSON.parse(modelsJson);
    const model = models.find((m: { id: string }) => m.id === modelId);
    if (!model?.model_repo) {
      console.log(`[main] model ${modelId} has no repo, skipping daemon warmup`);
      return;
    }

    // Check if daemon needs python (MLX models only)
    if (!model.needs_daemon) {
      console.log(`[main] model ${modelId} doesn't need daemon, skipping warmup`);
      return;
    }

    console.log(`[main] warming up daemon + loading model: ${model.model_repo}`);
    await getAddon().daemonStart();
    console.log("[main] daemon started");
    await getAddon().daemonLoadModel(model.model_repo);
    console.log(`[main] model loaded: ${model.model_repo}`);
  } catch (err) {
    console.error("[main] daemon warmup failed (non-fatal):", err);
  }
}

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
