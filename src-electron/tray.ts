/**
 * System tray for Electron — provides quick access to show/hide and quit.
 */
import { Tray, Menu, nativeImage, app } from "electron";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createMainWindow, getMainWindow } from "./windows";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

let tray: Tray | null = null;

/**
 * Create the system tray icon with a context menu.
 *
 * Call this once during app.whenReady().
 */
export function createTray(): Tray {
  if (tray && !tray.isDestroyed()) {
    return tray;
  }

  // Try to load a tray icon; fall back to an empty 16x16 image
  let icon = nativeImage.createEmpty();
  try {
    const iconPath = join(__dirname, "../../resources/tray-icon.png");
    const loaded = nativeImage.createFromPath(iconPath);
    if (!loaded.isEmpty()) {
      // Resize to 16x16 for tray (macOS template image convention)
      icon = loaded.resize({ width: 16, height: 16 });
      icon.setTemplateImage(true);
    }
  } catch {
    // Use empty icon as fallback
  }

  tray = new Tray(icon);
  tray.setToolTip("YuMo");

  const contextMenu = Menu.buildFromTemplate([
    {
      label: "Show Window",
      click: () => {
        const win = getMainWindow();
        if (win) {
          win.show();
          win.focus();
        } else {
          createMainWindow();
        }
      },
    },
    { type: "separator" },
    {
      label: "Quit",
      click: () => {
        app.quit();
      },
    },
  ]);

  tray.setContextMenu(contextMenu);

  // Click on tray icon toggles main window visibility
  tray.on("click", () => {
    const win = getMainWindow();
    if (win) {
      if (win.isVisible()) {
        win.hide();
      } else {
        win.show();
        win.focus();
      }
    } else {
      createMainWindow();
    }
  });

  return tray;
}

/**
 * Destroy the tray icon.
 */
export function destroyTray(): void {
  if (tray && !tray.isDestroyed()) {
    tray.destroy();
    tray = null;
  }
}
