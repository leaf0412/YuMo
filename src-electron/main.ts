import log from "./logger";
import { app, BrowserWindow } from "electron";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
import { registerAllHandlers } from "./ipc/index";
import { createMainWindow, createRecorderWindow } from "./windows";
import { createTray } from "./tray";
import { registerGlobalShortcut } from "./shortcuts";
import { getAddon } from "./addon";

// -------------------------------------------------------------------------
// App lifecycle
// -------------------------------------------------------------------------

app.whenReady().then(async () => {
  syncResources();
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
        log.info(`[main] restored hotkey: ${hotkey}`);
      }
    }
  } catch (err) {
    log.error("[main] failed to restore hotkey:", err);
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
      log.info("[warmup] no selected model, skipping");
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
      log.info(`[warmup] model ${modelId} not found or no repo, skipping`);
      return;
    }

    // Only MLX models need daemon — match regardless of case
    if (!model.provider?.toLowerCase().includes("mlx")) {
      log.info(`[warmup] model ${modelId} provider=${model.provider}, no daemon needed`);
      return;
    }

    if (model.downloaded === false) {
      log.info(`[warmup] model ${modelId} not downloaded, skipping`);
      return;
    }

    log.info(`[warmup] starting daemon for: ${model.model_repo}`);
    await getAddon().daemonStart();
    log.info("[warmup] daemon started, loading model...");
    await getAddon().daemonLoadModel(model.model_repo);
    log.info(`[warmup] model loaded: ${model.model_repo}`);
  } catch (err) {
    log.error("[warmup] failed (non-fatal):", err);
  }
}

// -------------------------------------------------------------------------
// Resource syncing — mirrors Tauri lib.rs lines 81-128
// -------------------------------------------------------------------------

function syncResources(): void {
  const dataDir = path.join(app.getPath("home"), ".voiceink");
  fs.mkdirSync(dataDir, { recursive: true });

  // Resource locations: production (resourcesPath) or dev (src-tauri/resources/)
  const prodDir = path.join(process.resourcesPath ?? "", "resources");
  const devDir = path.join(__dirname, "../../src-tauri/resources");
  const resDir = fs.existsSync(prodDir) ? prodDir : devDir;

  const filesToSync = [
    { name: "mlx_funasr_daemon.py", executable: false },
  ];

  // Sync denoiser models to ~/.voiceink/denoiser/
  const denoiserDir = path.join(dataDir, "denoiser");
  fs.mkdirSync(denoiserDir, { recursive: true });
  const denoiserFiles = ["dtln_1.onnx", "dtln_2.onnx"];

  for (const { name, executable } of filesToSync) {
    syncFile(resDir, dataDir, name, executable);
  }
  for (const name of denoiserFiles) {
    syncFile(path.join(resDir, "denoiser"), denoiserDir, name, false);
  }
}

function syncFile(
  srcDir: string,
  destDir: string,
  name: string,
  executable: boolean,
): void {
  const src = path.join(srcDir, name);
  const dest = path.join(destDir, name);
  if (!fs.existsSync(src)) {
    log.info(`[sync] ${name} not found at ${src}`);
    return;
  }
  const srcSize = fs.statSync(src).size;
  const destSize = fs.existsSync(dest) ? fs.statSync(dest).size : 0;
  if (!fs.existsSync(dest) || srcSize !== destSize) {
    fs.copyFileSync(src, dest);
    if (executable && process.platform !== "win32") {
      fs.chmodSync(dest, 0o755);
    }
    log.info(`[sync] ${name} synced (${srcSize} bytes)`);
  }
}
