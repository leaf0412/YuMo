import { app, BrowserWindow } from "electron";
import { registerAllHandlers } from "./ipc/index";
import { createMainWindow, createRecorderWindow } from "./windows";
import { createTray } from "./tray";
import { registerGlobalShortcut } from "./shortcuts";
import { getAddon } from "./addon";

// -------------------------------------------------------------------------
// App lifecycle
// -------------------------------------------------------------------------

app.whenReady().then(async () => {
  registerAllHandlers();
  createMainWindow();
  await createRecorderWindow();
  createTray();

  await restoreSavedHotkey();
  warmupDaemon(); // fire-and-forget, don't block startup

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

async function restoreSavedHotkey(): Promise<void> {
  try {
    const settingsJson = await getAddon().getAllSettings();
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

// -------------------------------------------------------------------------
// Daemon warmup — mirrors Tauri lib.rs lines 165-212
// -------------------------------------------------------------------------

async function warmupDaemon(): Promise<void> {
  try {
    const settingsJson = await getAddon().getAllSettings();
    const settings = JSON.parse(settingsJson);
    const modelId = settings.selected_model_id;
    if (!modelId) {
      console.log("[warmup] no selected model, skipping");
      return;
    }

    const modelsJson = await getAddon().listAvailableModels();
    const models = JSON.parse(modelsJson) as Array<{
      id: string;
      provider: string;
      model_repo?: string;
      downloaded?: boolean;
    }>;
    const model = models.find((m) => m.id === modelId);
    if (!model?.model_repo) {
      console.log(`[warmup] model ${modelId} not found or no repo, skipping`);
      return;
    }

    // Only MLX models need daemon — match regardless of case
    if (!model.provider?.toLowerCase().includes("mlx")) {
      console.log(`[warmup] model ${modelId} provider=${model.provider}, no daemon needed`);
      return;
    }

    if (model.downloaded === false) {
      console.log(`[warmup] model ${modelId} not downloaded, skipping`);
      return;
    }

    console.log(`[warmup] starting daemon for: ${model.model_repo}`);
    await getAddon().daemonStart();
    console.log("[warmup] daemon started, loading model...");
    await getAddon().daemonLoadModel(model.model_repo);
    console.log(`[warmup] model loaded: ${model.model_repo}`);
  } catch (err) {
    console.error("[warmup] failed (non-fatal):", err);
  }
}
