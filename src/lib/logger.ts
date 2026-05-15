/**
 * Unified logging layer for YuMo frontend (Tauri).
 *
 * Usage:
 *   import { invoke } from '../lib/logger';   // drop-in replacement
 *   const result = await invoke<T>('cmd_name', { arg1: 'x' });
 *
 * Every invoke call is automatically logged (command, args, result/error)
 * to both the browser console AND the backend log.txt file.
 *
 * `__TAURI_INTERNALS__` is injected by the Tauri shell before the renderer
 * loads. We guard the dynamic import so test environments (jsdom without
 * the global) never reach the real Tauri IPC machinery.
 */

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function platformInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new Error(`Tauri runtime not detected for invoke("${cmd}")`);
  }
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

function sendToBackend(level: 'info' | 'error', message: string) {
  platformInvoke('frontend_log', { level, message }).catch(() => {});
}

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

/** Extract readable message from Tauri invoke errors (serialized AppError). */
export function formatError(e: unknown, fallback: string): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object') {
    const vals = Object.values(e as Record<string, unknown>);
    if (vals.length > 0 && typeof vals[0] === 'string') return vals[0] as string;
  }
  return fallback;
}

/**
 * Log a structured event to both console and backend log.txt.
 * Usage: logEvent('Models', 'select_model', { module: 'Models', event: 'select_model' })
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
