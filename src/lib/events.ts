/**
 * Tauri event system thin wrapper.
 *
 * Handler receives Tauri-style event: { payload: T }
 * Usage:
 *   import { listen, emit } from '../lib/events';
 *   const unlisten = await listen<MyType>('event-name', (event) => {
 *     console.log(event.payload);
 *   });
 *
 * `__TAURI_INTERNALS__` is injected by the Tauri shell before the renderer
 * loads. We guard imports so that test environments (jsdom without the global)
 * never trigger the real `transformCallback` in `@tauri-apps/api/core` —
 * which would throw because the Tauri IPC isn't wired up.
 */

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

type UnlistenFn = () => void;

interface AppEvent<T> {
  payload: T;
}

export async function listen<T = unknown>(
  event: string,
  handler: (event: AppEvent<T>) => void,
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => {};
  }
  const { listen: tauriListen } = await import('@tauri-apps/api/event');
  return tauriListen<T>(event, (e) => handler({ payload: e.payload }));
}

export async function emit(event: string, payload?: unknown): Promise<void> {
  if (!isTauri()) return;
  const { emit: tauriEmit } = await import('@tauri-apps/api/event');
  await tauriEmit(event, payload);
}
