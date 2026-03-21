/**
 * Electron main process logger — uses electron-log for timestamped,
 * file-persisted logs matching Tauri's simplelog format.
 *
 * Log file: ~/.voiceink/log.txt (same location as Tauri version)
 */
import { createRequire } from "node:module";
import { join } from "node:path";
import { app } from "electron";

const require = createRequire(import.meta.url);
const log = require("electron-log/main");

// Configure log file path to match Tauri's log location
const logPath = join(app.getPath("home"), ".voiceink", "log.txt");
log.transports.file.resolvePathFn = () => logPath;

// Format: "HH:MM:SS [LEVEL] message" (matches Tauri's simplelog)
log.transports.file.format = "{h}:{i}:{s} [{level}] {text}";
log.transports.console.format = "{h}:{i}:{s} [{level}] {text}";

export default log;
