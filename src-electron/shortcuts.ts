/**
 * Global shortcut management for Electron.
 *
 * Registers a global hotkey that triggers toggle-recording on the renderer.
 */
import { globalShortcut } from "electron";
import { getMainWindow, getRecorderWindow } from "./windows";

/**
 * Register a global shortcut that emits "toggle-recording" to renderers.
 *
 * Tauri and Electron use the same modifier format (e.g. "Alt+Enter",
 * "CommandOrControl+Shift+Space") so no conversion is needed for most keys.
 */
export function registerGlobalShortcut(shortcut: string): boolean {
  // Normalize Tauri-style modifiers to Electron equivalents
  const electronShortcut = normalizeShortcut(shortcut);

  try {
    const success = globalShortcut.register(electronShortcut, () => {
      // Emit toggle-recording to both windows
      for (const win of [getMainWindow(), getRecorderWindow()]) {
        if (win && !win.isDestroyed()) {
          win.webContents.send("toggle-recording");
        }
      }
    });

    if (!success) {
      console.error(`[shortcuts] failed to register: ${electronShortcut}`);
    }
    return success;
  } catch (err) {
    console.error(`[shortcuts] register error for "${electronShortcut}":`, err);
    return false;
  }
}

/**
 * Unregister all global shortcuts.
 */
export function unregisterAllShortcuts(): void {
  globalShortcut.unregisterAll();
}

/**
 * Normalize a Tauri-format shortcut string to Electron's accelerator format.
 *
 * Tauri uses "Super" for the meta key; Electron uses "Super" on Linux but
 * "Command" on macOS and "Meta" generically. Most common combos work as-is.
 */
function normalizeShortcut(shortcut: string): string {
  return shortcut
    .replace(/\bSuper\b/g, "Super")
    .replace(/\bOption\b/g, "Alt");
}
