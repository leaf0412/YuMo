import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerKeychainHandlers(): void {
  ipcMain.handle("store-api-key", async (_e, args?: { provider?: string; key?: string }) => {
    if (args?.provider && args?.key) {
      await getAddon().storeApiKey(args.provider, args.key);
    }
  });

  ipcMain.handle("get-api-key", async (_e, args?: { provider?: string }) => {
    return args?.provider ? await getAddon().getApiKey(args.provider) : null;
  });

  ipcMain.handle("delete-api-key", async (_e, args?: { provider?: string }) => {
    if (args?.provider) await getAddon().deleteApiKey(args.provider);
  });
}
