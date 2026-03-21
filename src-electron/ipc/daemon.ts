import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerDaemonHandlers(): void {
  ipcMain.handle("daemon-status", () => {
    const s = getAddon().daemonStatus();
    return { running: s.running, loaded_model: s.loadedModel };
  });

  ipcMain.handle("daemon-start", async () => {
    await getAddon().daemonStart();
  });

  ipcMain.handle("daemon-stop", () => {
    getAddon().daemonStop();
  });

  ipcMain.handle("daemon-load-model", async (_e, args?: { modelRepo?: string }) => {
    if (args?.modelRepo) await getAddon().daemonLoadModel(args.modelRepo);
  });

  ipcMain.handle("daemon-unload-model", async () => {
    await getAddon().daemonUnloadModel();
  });

  ipcMain.handle("daemon-check-deps", () => {
    return getAddon().daemonCheckDeps();
  });
}
