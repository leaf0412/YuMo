/**
 * Electron main process logger — uses electron-log for timestamped,
 * file-persisted logs matching Tauri's simplelog format.
 *
 * Log file: ~/.voiceink/log.txt (same location as Tauri version)
 */
import log from "electron-log/main";
import { join } from "node:path";
import { app } from "electron";

// Configure log file path to match Tauri's log location
const logPath = join(app.getPath("home"), ".voiceink", "log.txt");
log.transports.file.resolvePathFn = () => logPath;

// Format: "HH:MM:SS [LEVEL] message" (matches Tauri's simplelog)
log.transports.file.format = "{h}:{i}:{s} [{level}] {text}";
log.transports.console.format = "{h}:{i}:{s} [{level}] {text}";

export default log;
