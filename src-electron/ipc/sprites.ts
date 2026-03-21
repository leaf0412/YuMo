import { ipcMain } from "electron";

export function registerSpritesHandlers(): void {
  ipcMain.handle("list-sprites", () => []);

  ipcMain.handle("get-sprite-image", () => {
    throw new Error("Sprite images not available in Electron yet");
  });

  ipcMain.handle("import-sprite-folder", () => {
    throw new Error("Sprite import not available in Electron yet");
  });

  ipcMain.handle("import-sprite-zip", () => {
    throw new Error("Sprite import not available in Electron yet");
  });

  ipcMain.handle("delete-sprite", () => {
    throw new Error("Sprite deletion not available in Electron yet");
  });

  ipcMain.handle("process-sprite-background", () => {
    throw new Error("Sprite processing not available in Electron yet");
  });
}
