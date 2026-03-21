/**
 * Window management for Electron — mirrors Tauri's dual-window setup.
 *
 * Two windows:
 *   - main: 1024x768, standard app window
 *   - recorder: 200x220, transparent overlay, always-on-top, no frame
 */
import { BrowserWindow, screen } from "electron";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// ---------------------------------------------------------------------------
// Window references
// ---------------------------------------------------------------------------

let mainWindow: BrowserWindow | null = null;
let recorderWindow: BrowserWindow | null = null;

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

function distPath(): string {
  return join(__dirname, "../../dist");
}

// ---------------------------------------------------------------------------
// Main window
// ---------------------------------------------------------------------------

export function createMainWindow(): BrowserWindow {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.focus();
    return mainWindow;
  }

  mainWindow = new BrowserWindow({
    title: "语墨 YuMo",
    width: 1024,
    height: 768,
    minWidth: 1024,
    minHeight: 768,
    webPreferences: {
      preload: join(__dirname, "../preload/preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });

  if (process.env.ELECTRON_RENDERER_URL) {
    mainWindow.loadURL(process.env.ELECTRON_RENDERER_URL);
  } else {
    mainWindow.loadFile(join(distPath(), "index.html"));
  }

  mainWindow.on("closed", () => {
    mainWindow = null;
  });

  return mainWindow;
}

// ---------------------------------------------------------------------------
// Recorder overlay window
// ---------------------------------------------------------------------------

export function createRecorderWindow(): BrowserWindow {
  if (recorderWindow && !recorderWindow.isDestroyed()) {
    return recorderWindow;
  }

  // Center horizontally, just below menu bar
  const display = screen.getPrimaryDisplay();
  const sw = display.workAreaSize.width;
  const x = Math.round((sw - 200) / 2);
  const y = 30;

  recorderWindow = new BrowserWindow({
    title: "YuMo Recorder",
    width: 200,
    height: 220,
    x,
    y,
    frame: false,
    transparent: true,
    backgroundColor: "#00000000",  // fully transparent
    alwaysOnTop: true,
    resizable: false,
    show: false,          // hidden by default, shown during recording
    skipTaskbar: true,
    hasShadow: false,
    focusable: false,     // don't steal focus from the text input
    webPreferences: {
      preload: join(__dirname, "../preload/preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false,
    },
  });
  console.log(`[windows] recorder window created at (${x}, ${y})`);

  if (process.env.ELECTRON_RENDERER_URL) {
    recorderWindow.loadURL(process.env.ELECTRON_RENDERER_URL + "/recorder.html");
  } else {
    recorderWindow.loadFile(join(distPath(), "recorder.html"));
  }

  recorderWindow.on("closed", () => {
    recorderWindow = null;
  });

  return recorderWindow;
}

// ---------------------------------------------------------------------------
// Window control API (used by IPC handlers)
// ---------------------------------------------------------------------------

export function showRecorder(): void {
  if (!recorderWindow || recorderWindow.isDestroyed()) {
    createRecorderWindow();
  }
  recorderWindow?.showInactive(); // show without stealing focus
  console.log("[windows] recorder shown");
}

export function hideRecorder(): void {
  recorderWindow?.hide();
}

export function isRecorderVisible(): boolean {
  return recorderWindow?.isVisible() ?? false;
}

export function getMainWindow(): BrowserWindow | null {
  return mainWindow && !mainWindow.isDestroyed() ? mainWindow : null;
}

export function getRecorderWindow(): BrowserWindow | null {
  return recorderWindow && !recorderWindow.isDestroyed() ? recorderWindow : null;
}
