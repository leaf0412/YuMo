import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerDaemonHandlers(): void {
  ipcMain.handle("daemon-status", () => {
    const s = getAddon().daemonStatus();
    return { running: s.running, loaded_model: s.loadedModel };
  });

  ipcMain.handle("daemon-start", () => {
    getAddon().daemonStart();
  });

  ipcMain.handle("daemon-stop", () => {
    getAddon().daemonStop();
  });

  ipcMain.handle("daemon-load-model", (_e, args?: { modelRepo?: string }) => {
    if (args?.modelRepo) getAddon().daemonLoadModel(args.modelRepo);
  });

  ipcMain.handle("daemon-unload-model", () => {
    getAddon().daemonUnloadModel();
  });

  ipcMain.handle("daemon-check-deps", () => {
    return getAddon().daemonCheckDeps();
  });
}
