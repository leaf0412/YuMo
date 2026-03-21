/**
 * Platform-aware event system — works with both Tauri and Electron.
 *
 * Handler receives Tauri-style event: { payload: T }
 * Usage:
 *   import { listen, emit } from '../lib/events';
 *   const unlisten = await listen<MyType>('event-name', (event) => {
 *     console.log(event.payload);
 *   });
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

/** Tauri-compatible event shape */
interface AppEvent<T> {
  payload: T;
}

// Local event bus for Electron (in-renderer emit/listen)
const electronListeners = new Map<string, Set<(event: AppEvent<unknown>) => void>>();

export async function listen<T = unknown>(
  event: string,
  handler: (event: AppEvent<T>) => void,
): Promise<UnlistenFn> {
  if (isTauri()) {
    const { listen: tauriListen } = await import('@tauri-apps/api/event');
    const unlisten = await tauriListen<T>(event, (e) => handler({ payload: e.payload }));
    return unlisten;
  }

  // Electron: register on local event bus
  if (!electronListeners.has(event)) {
    electronListeners.set(event, new Set());
  }
  const wrapped = (evt: AppEvent<unknown>) => handler(evt as AppEvent<T>);
  electronListeners.get(event)!.add(wrapped);

  // Also listen for main process → renderer IPC events via preload
  let unlistenIpc: (() => void) | undefined;
  const api = getElectronAPI();
  if (api) {
    unlistenIpc = api.on(event, (...args: unknown[]) => {
      handler({ payload: (args[0] ?? null) as T });
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
        handler({ payload });
      } catch (e) {
        console.error(`[events] handler error for "${event}":`, e);
      }
    }
  }
}
