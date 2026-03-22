import { create } from 'zustand';
import { invoke } from '../lib/logger';

export interface ModelInfo {
  id: string;
  name: string;
  size_mb: number;
  languages: string[];
  download_url: string;
  is_downloaded: boolean;
  provider: string;
  model_repo?: string;
  description?: string;
  speed?: number;
  accuracy?: number;
  is_recommended?: boolean;
  supported_languages?: Record<string, string>;
}

export interface DaemonStatus {
  running: boolean;
  loaded_model: string | null;
}

export interface AppSettings {
  [key: string]: unknown;
  language?: string;
  selected_model_id?: string;
  cloud_provider?: string;
  cloud_api_key?: string;
}

interface AppState {
  // Data
  settings: AppSettings;
  models: ModelInfo[];
  daemonStatus: DaemonStatus;
  downloadingModelId: string | null;
  permissions: { microphone: boolean; accessibility: boolean };
  uiLocale: string;

  // Navigation
  activeKey: string;
  setActiveKey: (key: string) => void;

  // Actions
  fetchSettings: () => Promise<void>;
  fetchModels: () => Promise<void>;
  fetchDaemonStatus: () => Promise<void>;
  fetchPermissions: () => Promise<void>;
  updateSetting: (key: string, value: string) => Promise<void>;
  setSettings: (partial: Partial<AppSettings>) => void;
  setDaemonStatus: (status: DaemonStatus) => void;
  setDownloadingModelId: (id: string | null) => void;
}

/** Dedup guard: skip invoke if called again within the cooldown window. */
const inflight = new Map<string, Promise<void>>();
function dedup(key: string, fn: () => Promise<void>): Promise<void> {
  const existing = inflight.get(key);
  if (existing) return existing;
  const p = fn().finally(() => inflight.delete(key));
  inflight.set(key, p);
  return p;
}

const useAppStore = create<AppState>((set, get) => ({
  settings: {},
  models: [],
  daemonStatus: { running: false, loaded_model: null },
  downloadingModelId: null,
  permissions: { microphone: false, accessibility: false },
  uiLocale: 'system',
  activeKey: '/',
  setActiveKey: (key) => set({ activeKey: key }),

  fetchSettings: () => dedup('settings', async () => {
    try {
      const result = await invoke<AppSettings>('get_settings');
      set({ settings: result });
      if (typeof result.ui_locale === 'string') {
        set({ uiLocale: result.ui_locale });
      }
    } catch { /* logged */ }
  }),

  fetchModels: () => dedup('models', async () => {
    try {
      const result = await invoke<ModelInfo[]>('list_available_models');
      set({ models: result });
    } catch { /* logged */ }
  }),

  fetchDaemonStatus: () => dedup('daemon', async () => {
    try {
      const status = await invoke<DaemonStatus>('daemon_status');
      set({ daemonStatus: status });
    } catch { /* logged */ }
  }),

  fetchPermissions: () => dedup('permissions', async () => {
    try {
      const result = await invoke<{ microphone: boolean; accessibility: boolean }>('check_permissions');
      set({ permissions: result });
    } catch { /* logged */ }
  }),

  updateSetting: async (key: string, value: string) => {
    await invoke('update_setting', { key, value });
    set({ settings: { ...get().settings, [key]: value } });
  },

  setSettings: (partial) => {
    set({ settings: { ...get().settings, ...partial } });
  },

  setDaemonStatus: (status) => {
    set({ daemonStatus: status });
  },

  setDownloadingModelId: (id) => {
    set({ downloadingModelId: id });
  },
}));

export default useAppStore;
