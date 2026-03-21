import { ipcMain, dialog } from "electron";
import { getAddon } from "../addon";

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
    const fs = await import("node:fs");
    const path = await import("node:path");
    const { app } = await import("electron");
    const modelsDir = path.join(app.getPath("home"), ".voiceink", "models");
    fs.mkdirSync(modelsDir, { recursive: true });
    const dest = path.join(modelsDir, path.basename(src));
    fs.copyFileSync(src, dest);
    return true;
  });
}
