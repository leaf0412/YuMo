import log from "./logger";
import { globalShortcut } from "electron";
import { getMainWindow, getRecorderWindow } from "./windows";

interface HotkeyConfig {
  code: string;
  key: string;
  is_modifier: boolean;
}

export function registerGlobalShortcut(configOrShortcut: string): boolean {
  let config: HotkeyConfig;
  try {
    config = JSON.parse(configOrShortcut);
  } catch {
    // Backward compat: old accelerator string format
    return registerAccelerator(configOrShortcut);
  }

  if (config.is_modifier) {
    log.warn(`[shortcuts] modifier-only hotkeys not supported in Electron: ${config.key}`);
    return false;
  }

  const accelerator = codeToAccelerator(config.code);
  if (!accelerator) {
    log.error(`[shortcuts] cannot map code to Electron accelerator: ${config.code}`);
    return false;
  }

  return registerAccelerator(accelerator);
}

function registerAccelerator(accelerator: string): boolean {
  try {
    const success = globalShortcut.register(accelerator, () => {
      log.info("[shortcuts] hotkey triggered, sending toggle-recording");
      for (const win of [getMainWindow(), getRecorderWindow()]) {
        if (win && !win.isDestroyed()) {
          win.webContents.send("toggle-recording");
        }
      }
    });
    if (!success) {
      log.error(`[shortcuts] failed to register: ${accelerator}`);
    }
    return success;
  } catch (err) {
    log.error(`[shortcuts] register error for "${accelerator}":`, err);
    return false;
  }
}

function codeToAccelerator(code: string): string | null {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  const map: Record<string, string> = {
    Space: "Space", Enter: "Enter", Tab: "Tab",
    Backspace: "Backspace", Delete: "Delete", Escape: "Escape",
    ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
    F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
    F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
    Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown",
    Minus: "-", Equal: "=",
    BracketLeft: "[", BracketRight: "]",
    Backslash: "\\", Semicolon: ";", Quote: "'",
    Comma: ",", Period: ".", Slash: "/",
    Backquote: "`",
  };
  return map[code] ?? null;
}

export function unregisterAllShortcuts(): void {
  globalShortcut.unregisterAll();
}
