import { app, BrowserWindow } from "electron";
import { registerAllHandlers } from "./ipc/index";
import { createMainWindow, createRecorderWindow } from "./windows";

// -------------------------------------------------------------------------
// App lifecycle
// -------------------------------------------------------------------------

app.whenReady().then(() => {
  registerAllHandlers();
  createMainWindow();
  createRecorderWindow(); // pre-create hidden, ready for recording

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
