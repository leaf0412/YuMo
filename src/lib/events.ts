/**
 * Platform-aware event system — works with both Tauri and Electron.
 *
 * Usage:
 *   import { listen, emit } from '../lib/events';
 *   const unlisten = await listen('event-name', (payload) => { ... });
 *   emit('event-name', { data: 'value' });
 */

const isTauri = '__TAURI_INTERNALS__' in window;

type UnlistenFn = () => void;

// Simple event bus for Electron (Tauri has its own)
const electronListeners = new Map<string, Set<(payload: unknown) => void>>();

export async function listen<T = unknown>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (isTauri) {
    const { listen: tauriListen } = await import('@tauri-apps/api/event');
    const unlisten = await tauriListen<T>(event, (e) => handler(e.payload));
    return unlisten;
  }

  // Electron: use in-process event bus
  if (!electronListeners.has(event)) {
    electronListeners.set(event, new Set());
  }
  const wrapped = (payload: unknown) => handler(payload as T);
  electronListeners.get(event)!.add(wrapped);
  return () => {
    electronListeners.get(event)?.delete(wrapped);
  };
}

export async function emit(event: string, payload?: unknown): Promise<void> {
  if (isTauri) {
    const { emit: tauriEmit } = await import('@tauri-apps/api/event');
    await tauriEmit(event, payload);
    return;
  }

  // Electron: dispatch to local listeners
  const listeners = electronListeners.get(event);
  if (listeners) {
    for (const handler of listeners) {
      try {
        handler(payload);
      } catch (e) {
        console.error(`[events] handler error for "${event}":`, e);
      }
    }
  }
}
