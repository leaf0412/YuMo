import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerSettingsHandlers(): void {
  ipcMain.handle("get-all-settings", () => {
    return JSON.parse(getAddon().getAllSettings());
  });

  ipcMain.handle("get-settings", () => {
    return JSON.parse(getAddon().getAllSettings());
  });

  ipcMain.handle("update-setting", (_e, args?: { key?: string; value?: unknown }) => {
    if (args?.key) {
      const val = typeof args.value === "string" ? args.value : JSON.stringify(args.value);
      getAddon().updateSetting(args.key, val);
    }
  });
}
