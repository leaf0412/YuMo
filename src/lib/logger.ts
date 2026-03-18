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
import { invoke as tauriInvoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------------
// Low-level: send a log line to backend log.txt (fire-and-forget)
// ---------------------------------------------------------------------------
function sendToBackend(level: 'info' | 'error', message: string) {
  tauriInvoke('frontend_log', { level, message }).catch(() => {});
}

// ---------------------------------------------------------------------------
// Public: drop-in replacement for tauri invoke — auto-logs everything
// ---------------------------------------------------------------------------

/** Drop-in replacement for `invoke` from `@tauri-apps/api/core`.
 *  Automatically logs command name, arguments, success, and errors
 *  to both console and backend log.txt. */
export async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  // Don't log the logging command itself to avoid infinite recursion
  if (cmd === 'frontend_log') {
    return tauriInvoke<T>(cmd, args);
  }

  const argsStr = args ? ` ${JSON.stringify(args)}` : '';
  const tag = `[invoke] ${cmd}`;

  console.log(`${tag}${argsStr}`);
  sendToBackend('info', `${tag}${argsStr}`);

  try {
    const result = await tauriInvoke<T>(cmd, args);
    console.log(`${tag} ok`);
    sendToBackend('info', `${tag} ok`);
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
