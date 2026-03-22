import { ipcMain } from "electron";
import { getAddon } from "../addon";
import { emitDaemonStatusChanged } from "../windows";

export function registerDaemonHandlers(): void {
  ipcMain.handle("daemon-status", () => {
    const s = getAddon().daemonStatus();
    return { running: s.running, loaded_model: s.loadedModel };
  });

  ipcMain.handle("daemon-start", async () => {
    await getAddon().daemonStart();
    emitDaemonStatusChanged();
  });

  ipcMain.handle("daemon-stop", () => {
    getAddon().daemonStop();
    emitDaemonStatusChanged();
  });

  ipcMain.handle("daemon-load-model", async (_e, args?: { modelRepo?: string }) => {
    if (args?.modelRepo) await getAddon().daemonLoadModel(args.modelRepo);
    emitDaemonStatusChanged();
  });

  ipcMain.handle("daemon-unload-model", async () => {
    await getAddon().daemonUnloadModel();
    emitDaemonStatusChanged();
  });

  ipcMain.handle("daemon-check-deps", async () => {
    return await getAddon().daemonCheckDeps();
  });
}
