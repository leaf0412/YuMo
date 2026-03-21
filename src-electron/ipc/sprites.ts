import { ipcMain } from "electron";

export function registerSpritesHandlers(): void {
  ipcMain.handle("list-sprites", () => []);
  ipcMain.handle("get-sprite-image", () => null);
  ipcMain.handle("import-sprite-folder", () => null);
  ipcMain.handle("import-sprite-zip", () => null);
  ipcMain.handle("delete-sprite", () => null);
  ipcMain.handle("process-sprite-background", () => null);
}
