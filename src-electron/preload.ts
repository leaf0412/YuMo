import { contextBridge, ipcRenderer } from "electron";

export interface ElectronAPI {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
  convertFileSrc(path: string): string;
  on(channel: string, callback: (...args: unknown[]) => void): () => void;
}

const api: ElectronAPI = {
  invoke(channel, ...args) {
    return ipcRenderer.invoke(channel, ...args);
  },

  convertFileSrc(path) {
    return `file://${path}`;
  },

  /** Listen for events from the main process. Returns an unsubscribe function. */
  on(channel, callback) {
    const listener = (_event: Electron.IpcRendererEvent, ...args: unknown[]) => {
      callback(...args);
    };
    ipcRenderer.on(channel, listener);
    return () => {
      ipcRenderer.removeListener(channel, listener);
    };
  },
};

contextBridge.exposeInMainWorld("electronAPI", api);
