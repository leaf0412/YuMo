/**
 * Shared electronAPI accessor for the CustomModels feature.
 *
 * Custom-model IPC requires the Electron host (no Tauri fallback for now —
 * the Python daemon spawn lives in main process). Centralised here so the
 * hook, section, and card share one cast + one error message.
 */
export type ElectronAPI = {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
};

export function getElectronAPI(): ElectronAPI {
  const api = (window as unknown as { electronAPI?: ElectronAPI }).electronAPI;
  if (!api) {
    throw new Error(
      'window.electronAPI is unavailable — custom models require the Electron host',
    );
  }
  return api;
}
