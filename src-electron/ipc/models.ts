import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerModelsHandlers(): void {
  ipcMain.handle("list-available-models", () => {
    return JSON.parse(getAddon().listAvailableModels());
  });

  ipcMain.handle("select-model", (_e, args?: { modelId?: string }) => {
    if (args?.modelId) {
      getAddon().updateSetting("selected_model_id", JSON.stringify(args.modelId));
    }
  });

  // --- Model download/delete/import (not available in Electron yet) ---
  ipcMain.handle("download-model", () => {
    throw new Error("Model download not available in Electron yet");
  });

  ipcMain.handle("delete-model", () => {
    throw new Error("Model deletion not available in Electron yet");
  });

  ipcMain.handle("import-model", () => {
    throw new Error("Model import not available in Electron yet");
  });
}
