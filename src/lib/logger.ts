/**
 * Unified logging layer for VoiceInk frontend.
 *
 * Usage:
 *   import { invoke } from '../lib/logger';   // drop-in replacement
 *   const result = await invoke<T>('cmd_name', { arg1: 'x' });
 *
 * Every invoke call is automatically logged (command, args, result/error)
 * to both the browser console AND the backend log.txt file.
 */
// ---------------------------------------------------------------------------
// Platform-aware invoke: Tauri or Electron IPC
// ---------------------------------------------------------------------------

type ElectronAPI = {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
};

const isTauri = '__TAURI_INTERNALS__' in window;
const electronAPI = !isTauri ? (window as unknown as { electronAPI?: ElectronAPI }).electronAPI : undefined;

async function platformInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  if (electronAPI) {
    // Electron IPC: convert snake_case command to kebab-case channel
    const channel = cmd.replace(/_/g, '-');
    const result = await electronAPI.invoke(channel, args);
    return result as T;
  }
  throw new Error(`No backend available for invoke("${cmd}")`);
}

// ---------------------------------------------------------------------------
// Low-level: send a log line to backend log.txt (fire-and-forget)
// ---------------------------------------------------------------------------
function sendToBackend(level: 'info' | 'error', message: string) {
  platformInvoke('frontend_log', { level, message }).catch(() => {});
}

// ---------------------------------------------------------------------------
// Public: drop-in replacement for tauri invoke — auto-logs everything
// ---------------------------------------------------------------------------

const MAX_LOG_LEN = 200;

/** Summarise a result for logging: arrays show count, objects are truncated. */
function summariseResult(result: unknown): string {
  if (result == null || typeof result !== 'object') return '';
  if (Array.isArray(result)) return ` => Array(${result.length})`;
  const json = JSON.stringify(result);
  if (json.length <= MAX_LOG_LEN) return ` => ${json}`;
  return ` => ${json.slice(0, MAX_LOG_LEN)}…`;
}

/** Drop-in replacement for `invoke` from `@tauri-apps/api/core`.
 *  Automatically logs command name, arguments, success, and errors
 *  to both console and backend log.txt. */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  // Don't log the logging command itself to avoid infinite recursion
  if (cmd === 'frontend_log') {
    return platformInvoke<T>(cmd, args);
  }

  const argsStr = args ? ` ${JSON.stringify(args)}` : '';
  const tag = `[invoke] ${cmd}`;

  console.log(`${tag}${argsStr}`);
  sendToBackend('info', `${tag}${argsStr}`);

  try {
    const result = await platformInvoke<T>(cmd, args);
    const resultStr = summariseResult(result);
    console.log(`${tag} ok${resultStr}`);
    sendToBackend('info', `${tag} ok${resultStr}`);
    return result;
  } catch (e) {
    const detail = formatError(e, 'unknown error');
    console.error(`${tag} FAILED: ${detail}`);
    sendToBackend('error', `${tag} FAILED: ${detail}`);
    throw e;
  }
}

// ---------------------------------------------------------------------------
// Public: extract readable message from Tauri AppError
// ---------------------------------------------------------------------------

/** Extract readable message from Tauri invoke errors (serialized AppError). */
export function formatError(e: unknown, fallback: string): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object') {
    const vals = Object.values(e as Record<string, unknown>);
    if (vals.length > 0 && typeof vals[0] === 'string') return vals[0] as string;
  }
  return fallback;
}

// ---------------------------------------------------------------------------
// Public: log structured frontend events
// ---------------------------------------------------------------------------

/**
 * Log a structured event to both console and backend log.txt.
 * Usage: logEvent('Models', 'select_model', { model_id: 'xxx' })
 * Output: [frontend:Models] [select_model] model_id="xxx"
 */
export function logEvent(module: string, event: string, data?: Record<string, unknown>) {
  const kvs = data
    ? ' ' + Object.entries(data).map(([k, v]) => `${k}=${JSON.stringify(v)}`).join(' ')
    : '';
  const msg = `[frontend:${module}] [${event}]${kvs}`;
  console.log(msg);
  sendToBackend('info', msg);
}
