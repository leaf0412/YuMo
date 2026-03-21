import { contextBridge, ipcRenderer } from "electron";

export interface ElectronAPI {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
  convertFileSrc(path: string): string;
}

const api: ElectronAPI = {
  invoke(channel, ...args) {
    return ipcRenderer.invoke(channel, ...args);
  },

  convertFileSrc(path) {
    return `file://${path}`;
  },
};

contextBridge.exposeInMainWorld("electronAPI", api);
