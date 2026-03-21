/**
 * Platform-aware event system — works with both Tauri and Electron.
 *
 * Usage:
 *   import { listen, emit } from '../lib/events';
 *   const unlisten = await listen('event-name', (payload) => { ... });
 *   emit('event-name', { data: 'value' });
 */

type ElectronAPI = {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
  on(channel: string, callback: (...args: unknown[]) => void): () => void;
};

function isTauri(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

function getElectronAPI(): ElectronAPI | undefined {
  return (window as unknown as { electronAPI?: ElectronAPI }).electronAPI;
}

type UnlistenFn = () => void;

// Local event bus for Electron (in-renderer emit/listen)
const electronListeners = new Map<string, Set<(payload: unknown) => void>>();

export async function listen<T = unknown>(
  event: string,
  handler: (payload: T) => void,
): Promise<UnlistenFn> {
  if (isTauri()) {
    const { listen: tauriListen } = await import('@tauri-apps/api/event');
    // Tauri passes { event, payload, ... } — forward the full object
    // Existing code accesses handler(e).payload so we pass the event as-is
    const unlisten = await tauriListen<T>(event, (e) => handler(e as unknown as T));
    return unlisten;
  }

  // Electron: register on local event bus
  if (!electronListeners.has(event)) {
    electronListeners.set(event, new Set());
  }
  const wrapped = (payload: unknown) => handler({ payload } as unknown as T);
  electronListeners.get(event)!.add(wrapped);

  // Also listen for main process → renderer IPC events via preload
  let unlistenIpc: (() => void) | undefined;
  const api = getElectronAPI();
  console.log(`[events] listen("${event}") electronAPI=${!!api} isTauri=${isTauri()}`);
  if (api) {
    unlistenIpc = api.on(event, (...args: unknown[]) => {
      // Wrap in { payload } to match Tauri event shape that existing code expects
      handler({ payload: args[0] ?? null } as unknown as T);
    });
  }

  return () => {
    electronListeners.get(event)?.delete(wrapped);
    unlistenIpc?.();
  };
}

export async function emit(event: string, payload?: unknown): Promise<void> {
  if (isTauri()) {
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
