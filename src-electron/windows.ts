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
import { getAddon } from "./addon";

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

  // Restore saved position from DB, or center below menu bar
  const saved = loadWindowPosition("recorder");
  const display = screen.getPrimaryDisplay();
  const sw = display.workAreaSize.width;
  const x = saved ? Math.round(saved.x) : Math.round((sw - 200) / 2);
  const y = saved ? Math.round(saved.y) : 30;
  const width = saved ? Math.round(saved.width) : 200;
  const height = saved ? Math.round(saved.height) : 220;

  recorderWindow = new BrowserWindow({
    title: "YuMo Recorder",
    width,
    height,
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
    // Draggable via -webkit-app-region: drag in the HTML/CSS
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

  // Save position when window is moved (dragged)
  recorderWindow.on("moved", () => {
    if (recorderWindow && !recorderWindow.isDestroyed()) {
      const [rx, ry] = recorderWindow.getPosition();
      const [rw, rh] = recorderWindow.getSize();
      saveWindowPosition("recorder", { x: rx, y: ry, width: rw, height: rh });
    }
  });

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

// ---------------------------------------------------------------------------
// Window position persistence (mirrors Tauri WindowManager)
// ---------------------------------------------------------------------------

type WindowPos = { x: number; y: number; width: number; height: number };

function loadWindowPosition(label: string): WindowPos | null {
  try {
    const settingsJson = getAddon().getAllSettings();
    const settings = JSON.parse(settingsJson);
    const layoutStr = settings.window_layout;
    if (typeof layoutStr !== "string") return null;
    const layout = JSON.parse(layoutStr);
    const pos = layout?.positions?.[label];
    if (pos && typeof pos.x === "number") {
      console.log(`[windows] restored position for '${label}': (${pos.x}, ${pos.y})`);
      return pos;
    }
  } catch {
    // ignore parse errors
  }
  return null;
}

function saveWindowPosition(label: string, pos: WindowPos): void {
  try {
    const settingsJson = getAddon().getAllSettings();
    const settings = JSON.parse(settingsJson);
    const layoutStr = settings.window_layout;
    const layout = typeof layoutStr === "string" ? JSON.parse(layoutStr) : { positions: {} };
    if (!layout.positions) layout.positions = {};
    layout.positions[label] = pos;
    getAddon().updateSetting("window_layout", JSON.stringify(JSON.stringify(layout)));
    console.log(`[windows] saved position for '${label}': (${pos.x}, ${pos.y})`);
  } catch (err) {
    console.error(`[windows] failed to save position for '${label}':`, err);
  }
}
