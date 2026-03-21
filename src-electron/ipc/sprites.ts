import { ipcMain, dialog } from "electron";
import { getAddon } from "../addon";

export function registerSpritesHandlers(): void {
  ipcMain.handle("list-sprites", () => {
    return JSON.parse(getAddon().listSprites());
  });

  ipcMain.handle(
    "get-sprite-image",
    (_e, args?: { dirId?: string; fileName?: string }) => {
      if (args?.dirId && args?.fileName) {
        return getAddon().getSpriteImage(args.dirId, args.fileName);
      }
      return null;
    },
  );

  ipcMain.handle("import-sprite-folder", async () => {
    const result = await dialog.showOpenDialog({
      title: "Select Sprite Folder",
      properties: ["openDirectory"],
    });
    if (result.canceled || result.filePaths.length === 0) {
      return null;
    }
    const json = getAddon().importSpriteFolder(result.filePaths[0]);
    return JSON.parse(json);
  });

  ipcMain.handle("import-sprite-zip", async () => {
    const result = await dialog.showOpenDialog({
      title: "Select Sprite Archive",
      filters: [{ name: "Sprite Archive", extensions: ["zip"] }],
      properties: ["openFile"],
    });
    if (result.canceled || result.filePaths.length === 0) {
      return null;
    }
    const json = getAddon().importSpriteZip(result.filePaths[0]);
    return JSON.parse(json);
  });

  ipcMain.handle("delete-sprite", (_e, args?: { dirId?: string }) => {
    if (args?.dirId) {
      getAddon().deleteSprite(args.dirId);
    }
  });

  ipcMain.handle("process-sprite-background", () => {
    // Background processing not yet implemented in napi
    return null;
  });
}
