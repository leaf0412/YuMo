import { invoke } from '@tauri-apps/api/core';

export type UiLocale = 'system' | 'zh-CN' | 'en';
export type ResolvedLocale = 'zh-CN' | 'en';

export function resolveSystemLocale(systemLocale: string): ResolvedLocale {
  const lower = systemLocale.toLowerCase().replace('_', '-');
  if (lower.startsWith('zh')) return 'zh-CN';
  return 'en';
}

export async function getResolvedLocale(uiLocale?: string): Promise<ResolvedLocale> {
  if (uiLocale && uiLocale !== 'system') {
    return uiLocale as ResolvedLocale;
  }
  try {
    const systemLocale = await invoke<string>('get_system_locale');
    return resolveSystemLocale(systemLocale);
  } catch {
    return 'en'; // fallback consistent with i18next fallbackLng
  }
}
