import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerKeychainHandlers(): void {
  ipcMain.handle("store-api-key", (_e, args?: { provider?: string; key?: string }) => {
    if (args?.provider && args?.key) {
      getAddon().storeApiKey(args.provider, args.key);
    }
  });

  ipcMain.handle("get-api-key", (_e, args?: { provider?: string }) => {
    return args?.provider ? getAddon().getApiKey(args.provider) : null;
  });

  ipcMain.handle("delete-api-key", (_e, args?: { provider?: string }) => {
    if (args?.provider) getAddon().deleteApiKey(args.provider);
  });
}
