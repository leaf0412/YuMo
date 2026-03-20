import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import zhCN from './locales/zh-CN.json';
import en from './locales/en.json';
import { getResolvedLocale } from './utils';
import { invoke } from '@tauri-apps/api/core';

export async function initI18n(): Promise<void> {
  let uiLocale: string | undefined;
  try {
    const settings = await invoke<Record<string, unknown>>('get_settings');
    uiLocale = typeof settings.ui_locale === 'string' ? settings.ui_locale : undefined;
  } catch { /* use system default */ }

  const resolvedLocale = await getResolvedLocale(uiLocale);

  await i18n.use(initReactI18next).init({
    resources: {
      'zh-CN': { translation: zhCN },
      en: { translation: en },
    },
    lng: resolvedLocale,
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
  });
}

export default i18n;
