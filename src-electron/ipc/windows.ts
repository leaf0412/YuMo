/**
 * Window management IPC handlers.
 */
import { ipcMain } from "electron";
import {
  showRecorder,
  hideRecorder,
  isRecorderVisible,
  getMainWindow,
} from "../windows";

export function registerWindowHandlers(): void {
  ipcMain.handle("show-recorder", () => {
    showRecorder();
  });

  ipcMain.handle("hide-recorder", () => {
    hideRecorder();
  });

  ipcMain.handle("toggle-recorder", () => {
    if (isRecorderVisible()) {
      hideRecorder();
    } else {
      showRecorder();
    }
  });

  ipcMain.handle("show-main-window", () => {
    getMainWindow()?.show();
    getMainWindow()?.focus();
  });

  ipcMain.handle("hide-main-window", () => {
    getMainWindow()?.hide();
  });
}
