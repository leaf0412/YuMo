import { ipcMain, dialog, shell, app } from "electron";
import { promises as fsp } from "node:fs";
import * as fs from "node:fs";
import * as path from "node:path";
import { getAddon } from "../addon";

// ---------------------------------------------------------------------------
// Custom-model paths (~/.voiceink subtree)
// ---------------------------------------------------------------------------

function homeVoiceinkDir(): string {
  return path.join(app.getPath("home"), ".voiceink");
}

function customModelsDir(): string {
  return path.join(homeVoiceinkDir(), "custom_models");
}

function voiceinkModelsDir(): string {
  return path.join(homeVoiceinkDir(), "models");
}

function customCacheDir(): string {
  return path.join(customModelsDir(), ".cache");
}

function trustedFilePath(): string {
  return path.join(customModelsDir(), ".trusted");
}

export function registerModelsHandlers(): void {
  ipcMain.handle("list-available-models", async () => {
    return JSON.parse(await getAddon().listAvailableModels());
  });

  ipcMain.handle("select-model", async (_e, args?: { modelId?: string }) => {
    if (args?.modelId) {
      await getAddon().updateSetting("selected_model_id", JSON.stringify(args.modelId));
    }
  });

  ipcMain.handle("download-model", async (_e, args?: { modelId?: string }) => {
    if (!args?.modelId) return;
    await getAddon().downloadModel(args.modelId);
  });

  ipcMain.handle("delete-model", async (_e, args?: { modelId?: string }) => {
    if (args?.modelId) {
      await getAddon().deleteModel(args.modelId);
    }
  });

  ipcMain.handle("import-model", async () => {
    const result = await dialog.showOpenDialog({
      title: "Select Whisper Model",
      filters: [{ name: "Whisper Model", extensions: ["bin"] }],
      properties: ["openFile"],
    });
    if (result.canceled || result.filePaths.length === 0) {
      return false;
    }
    // Copy file to models directory
    const src = result.filePaths[0];
    const modelsDir = voiceinkModelsDir();
    fs.mkdirSync(modelsDir, { recursive: true });
    const dest = path.join(modelsDir, path.basename(src));
    fs.copyFileSync(src, dest);
    return true;
  });

  // -------------------------------------------------------------------------
  // Custom-model YAML plugin handlers
  // -------------------------------------------------------------------------

  // 1. Scan ~/.voiceink/custom_models/*.yaml and return { ok, errors }.
  // TODO(T17): wire to Rust scan_custom_models via napi addon. For now this
  // returns an empty scaffold so the IPC channel is registered and the
  // renderer can be built before T17 lands.
  ipcMain.handle("list-custom-models", async () => {
    return { ok: [], errors: [] };
  });

  // 2. Check whether a custom spec's pip dependencies are satisfied.
  // TODO(T17): wire to napi addon (e.g. customCheckDeps(specPath)) which
  // forwards to the Python daemon `check_custom_dependencies` action.
  ipcMain.handle("custom-check-deps", async (_e, _specPath: string) => {
    throw new Error(
      "custom-check-deps not yet wired — pending T17 napi bridge",
    );
  });

  // 3. Install pip dependencies for a custom spec.
  // TODO(T17): wire to napi addon (e.g. customInstallDeps(specPath)) which
  // forwards to the Python daemon `install_custom_dependencies` action.
  ipcMain.handle("custom-install-deps", async (_e, _specPath: string) => {
    throw new Error(
      "custom-install-deps not yet wired — pending T17 napi bridge",
    );
  });

  // 4. Run the spec's download step (function or hf_repos variant).
  // TODO(T17): wire to napi addon (e.g. customDownload(specPath, voiceinkModelsDir, customModelsDir))
  // which forwards to the Python daemon `download_custom_model` action.
  ipcMain.handle("custom-download", async (_e, _specPath: string) => {
    throw new Error(
      "custom-download not yet wired — pending T17 napi bridge",
    );
  });

  // 5. Open ~/.voiceink/custom_models/ in the OS file manager.
  ipcMain.handle("custom-open-dir", async () => {
    const dir = customModelsDir();
    await fsp.mkdir(dir, { recursive: true });
    const err = await shell.openPath(dir);
    if (err) {
      throw new Error(`Failed to open custom models dir: ${err}`);
    }
  });

  // 6. Copy a built-in example YAML from the app bundle to the custom dir.
  ipcMain.handle(
    "custom-import-example",
    async (_e, fileName: string) => {
      // Defensive: prevent path traversal — fileName must be a bare basename.
      const safeName = path.basename(fileName);
      if (safeName !== fileName || safeName.startsWith(".")) {
        throw new Error(`Invalid example file name: ${fileName}`);
      }
      const src = path.join(
        app.getAppPath(),
        "_docs",
        "custom_model_examples",
        safeName,
      );
      const dest = path.join(customModelsDir(), safeName);
      await fsp.mkdir(customModelsDir(), { recursive: true });
      await fsp.copyFile(src, dest);
      return { destPath: dest };
    },
  );

  // 7. Remove a custom spec: YAML + sidecar + downloaded dirs (~/.voiceink only).
  ipcMain.handle("custom-remove", async (_e, specPath: string) => {
    // Sidecar lookup uses the YAML basename. The Rust scan resolves spec id
    // from the YAML body, but we don't parse YAML in node — accepting basename
    // is consistent with how Rust writes the sidecar in T11/T12.
    // TODO(T18): if the renderer has the parsed spec, prefer (specPath, specId).
    const baseName = path.basename(specPath, path.extname(specPath));
    const sidecarPath = path.join(customCacheDir(), `${baseName}.paths.json`);

    let downloadedDirs: string[] = [];
    try {
      const raw = await fsp.readFile(sidecarPath, "utf-8");
      const sidecar = JSON.parse(raw) as Record<string, string>;
      downloadedDirs = Object.values(sidecar);
    } catch {
      // Missing or unreadable sidecar is acceptable — nothing to clean up.
    }

    await fsp.rm(specPath, { force: true });
    await fsp.rm(sidecarPath, { force: true });

    const safeRoot = homeVoiceinkDir();
    for (const d of downloadedDirs) {
      if (typeof d !== "string" || !d) continue;
      const resolved = path.resolve(d);
      // Only delete dirs that live inside ~/.voiceink/.
      if (
        resolved === safeRoot ||
        resolved.startsWith(safeRoot + path.sep)
      ) {
        await fsp.rm(resolved, { recursive: true, force: true });
      }
    }
    return { ok: true };
  });

  // 8. is-downloaded check — sidecar paths.json existence.
  ipcMain.handle("custom-is-downloaded", async (_e, id: string) => {
    const sidecar = path.join(customCacheDir(), `${id}.paths.json`);
    try {
      await fsp.access(sidecar);
      return true;
    } catch {
      return false;
    }
  });

  // 9. Trust state — backed by ~/.voiceink/custom_models/.trusted line list.
  ipcMain.handle("custom-is-trusted", async (_e, id: string) => {
    try {
      const raw = await fsp.readFile(trustedFilePath(), "utf-8");
      const list = raw
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
      return list.includes(id);
    } catch {
      return false;
    }
  });

  ipcMain.handle("custom-set-trusted", async (_e, id: string) => {
    await fsp.mkdir(customModelsDir(), { recursive: true });
    let list: string[] = [];
    try {
      const raw = await fsp.readFile(trustedFilePath(), "utf-8");
      list = raw
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
    } catch {
      // First-time write — empty list is fine.
    }
    if (!list.includes(id)) {
      list.push(id);
    }
    await fsp.writeFile(trustedFilePath(), list.join("\n") + "\n");
  });
}
