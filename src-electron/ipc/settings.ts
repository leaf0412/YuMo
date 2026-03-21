import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerSettingsHandlers(): void {
  ipcMain.handle("get-all-settings", async () => {
    return JSON.parse(await getAddon().getAllSettings());
  });

  ipcMain.handle("get-settings", async () => {
    return JSON.parse(await getAddon().getAllSettings());
  });

  ipcMain.handle("update-setting", async (_e, args?: { key?: string; value?: unknown }) => {
    if (args?.key) {
      const val = typeof args.value === "string" ? args.value : JSON.stringify(args.value);
      await getAddon().updateSetting(args.key, val);
    }
  });
}
