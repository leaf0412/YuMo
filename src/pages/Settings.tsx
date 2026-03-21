import { useEffect, useState, useCallback } from 'react';
import {
  Collapse, Switch, Slider, Select, Input, Button, Flex, Space, Typography,
  message, Popconfirm, InputNumber,
} from 'antd';
import {
  AudioOutlined, FilterOutlined, ThunderboltOutlined, CopyOutlined,
  FontSizeOutlined, DesktopOutlined, KeyOutlined, AppstoreOutlined,
  HistoryOutlined, SettingOutlined, ClearOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { emit } from '@tauri-apps/api/event';
import i18n from '../i18n';
import { getResolvedLocale, type UiLocale } from '../i18n/utils';
import { invoke, formatError, logEvent } from '../lib/logger';

const { Text } = Typography;

interface AudioDevice {
  id: number;
  name: string;
  is_default: boolean;
}

interface AppSettings {
  audio_device?: string;
  language?: string;
  sound_enabled?: boolean;
  custom_sound_file?: string;
  noise_reduction?: boolean;
  vad_enabled?: boolean;
  vad_sensitivity?: number;
  vad_silence_timeout?: number;
  clipboard_restore?: boolean;
  paste_delay?: number;
  auto_capitalize?: boolean;
  system_mute?: boolean;
  hotkey?: string;
  menu_bar_mode?: boolean;
  auto_cleanup?: boolean;
  auto_cleanup_days?: number;
  autostart?: boolean;
  data_path?: string;
  ui_locale?: string;
}

export default function Settings() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<AppSettings>({});
  const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
  const [hotkeyInput, setHotkeyInput] = useState('');

  const loadSettings = useCallback(async () => {
    try {
      const result = await invoke<AppSettings>('get_settings');
      setSettings(result);
      setHotkeyInput(result.hotkey || '');
    } catch { /* logged */ }
  }, []);

  const loadDevices = useCallback(async () => {
    try {
      const result = await invoke<AudioDevice[]>('list_audio_devices');
      setAudioDevices(result);
    } catch { /* logged */ }
  }, []);

  useEffect(() => {
    loadSettings();
    loadDevices();
  }, [loadSettings, loadDevices]);

  const updateSetting = async (key: string, value: unknown) => {
    try {
      await invoke('update_setting', { key, value });
      setSettings((prev) => ({ ...prev, [key]: value }));
      logEvent('Settings', 'setting_changed', { key, value });
    } catch (e) {
      message.error(formatError(e, t('settings.updateFailed')));
    }
  };

  const [recordingHotkey, setRecordingHotkey] = useState(false);

  const keyEventToShortcut = (e: React.KeyboardEvent): string | null => {
    const parts: string[] = [];
    if (e.metaKey || e.ctrlKey) parts.push('CommandOrControl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');

    const key = e.key;
    if (['Meta', 'Control', 'Alt', 'Shift'].includes(key)) return null;

    const keyMap: Record<string, string> = {
      ' ': 'Space', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right',
      Enter: 'Enter', Backspace: 'Backspace', Delete: 'Delete', Escape: 'Escape',
      Tab: 'Tab', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
    };
    const mapped = keyMap[key] || (key.length === 1 ? key.toUpperCase() : key);
    parts.push(mapped);

    if (parts.length < 2) return null;
    return parts.join('+');
  };

  const handleHotkeyKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const shortcut = keyEventToShortcut(e);
    if (shortcut) {
      setHotkeyInput(shortcut);
      setRecordingHotkey(false);
      logEvent('Settings', 'hotkey_captured', { shortcut });
      (async () => {
        try {
          await invoke('register_hotkey', { shortcut });
          updateSetting('hotkey', shortcut);
          logEvent('Settings', 'hotkey_registered', { shortcut });
          message.success(t('settings.hotkeySet', { shortcut }));
        } catch (e: unknown) {
          message.error(formatError(e, t('settings.registerFailed')));
        }
      })();
    }
  };

  const handleClearHotkey = async () => {
    try {
      await invoke('unregister_hotkey');
      setHotkeyInput('');
      setRecordingHotkey(false);
      updateSetting('hotkey', '');
      logEvent('Settings', 'hotkey_cleared');
      message.success(t('settings.hotkeyCleared'));
    } catch (e) {
      message.error(formatError(e, t('settings.clearFailed')));
    }
  };

  const handleClearAllHistory = async () => {
    try {
      await invoke('delete_all_transcriptions');
      logEvent('Settings', 'history_cleared');
      message.success(t('settings.historyClearedSuccess'));
    } catch (e) {
      message.error(formatError(e, t('settings.clearHistoryFailed')));
    }
  };

  const settingRow = (label: string, control: React.ReactNode) => (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0' }}>
      <Text>{label}</Text>
      {control}
    </div>
  );

  const items = [
    {
      key: 'audio',
      label: <Space data-testid="settings-recording"><AudioOutlined />{t('settings.sectionAudio')}</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(t('settings.audioDevice'),
            <Select value={settings.audio_device} onChange={(v) => updateSetting('audio_device', v)} style={{ width: 250 }} placeholder={t('settings.audioDevicePlaceholder')} options={audioDevices.map((d) => ({ value: d.id, label: d.name }))} />,
          )}
          {settingRow(t('settings.transcriptionLang'),
            <Select value={settings.language || 'auto'} onChange={(v) => updateSetting('language', v)} style={{ width: 250 }}
              options={[
                { value: 'auto', label: t('settings.langAutoDetect') },
                { value: 'zh', label: t('settings.langChinese') },
                { value: 'en', label: 'English' },
                { value: 'ja', label: '\u65E5\u672C\u8A9E' },
                { value: 'ko', label: '\uD55C\uAD6D\uC5B4' },
                { value: 'fr', label: 'Fran\u00E7ais' },
                { value: 'de', label: 'Deutsch' },
                { value: 'es', label: 'Espa\u00F1ol' },
                { value: 'ru', label: '\u0420\u0443\u0441\u0441\u043A\u0438\u0439' },
              ]}
            />,
          )}
          {settingRow(t('settings.recordingSound'),
            <Switch checked={settings.sound_enabled} onChange={(v) => updateSetting('sound_enabled', v)} />,
          )}
          {settingRow(t('settings.customSoundFile'),
            <Input value={settings.custom_sound_file || ''} onChange={(e) => updateSetting('custom_sound_file', e.target.value)} placeholder={t('settings.customSoundFilePlaceholder')} style={{ width: 250 }} />,
          )}
        </Flex>
      ),
    },
    {
      key: 'noise',
      label: <Space><FilterOutlined />{t('settings.sectionNoise')}</Space>,
      children: settingRow(t('settings.enableNoise'), <Switch checked={settings.noise_reduction} onChange={(v) => updateSetting('noise_reduction', v)} />),
    },
    {
      key: 'vad',
      label: <Space data-testid="settings-transcription"><ThunderboltOutlined />{t('settings.sectionVad')}</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(t('settings.enableVad'), <Switch checked={settings.vad_enabled} onChange={(v) => updateSetting('vad_enabled', v)} />)}
          <div style={{ padding: '8px 0' }}>
            <Text>{t('settings.vadSensitivity')}</Text>
            <Slider min={0} max={100} value={settings.vad_sensitivity ?? 50} onChange={(v) => updateSetting('vad_sensitivity', v)} />
          </div>
          <div style={{ padding: '8px 0' }}>
            <Text>{t('settings.vadSilenceTimeout')}</Text>
            <Slider min={100} max={5000} step={100} value={settings.vad_silence_timeout ?? 1000} onChange={(v) => updateSetting('vad_silence_timeout', v)} />
          </div>
        </Flex>
      ),
    },
    {
      key: 'paste',
      label: <Space><CopyOutlined />{t('settings.sectionPaste')}</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(t('settings.clipboardRestore'), <Switch checked={settings.clipboard_restore} onChange={(v) => updateSetting('clipboard_restore', v)} />)}
          <div style={{ padding: '8px 0' }}>
            <Text>{t('settings.pasteDelay')}</Text>
            <Slider min={0} max={1000} step={50} value={settings.paste_delay ?? 100} onChange={(v) => updateSetting('paste_delay', v)} />
          </div>
        </Flex>
      ),
    },
    {
      key: 'format',
      label: <Space><FontSizeOutlined />{t('settings.sectionFormat')}</Space>,
      children: settingRow(t('settings.autoCapitalize'), <Switch checked={settings.auto_capitalize} onChange={(v) => updateSetting('auto_capitalize', v)} />),
    },
    {
      key: 'system',
      label: <Space><DesktopOutlined />{t('settings.sectionSystem')}</Space>,
      children: settingRow(t('settings.systemMute'), <Switch checked={settings.system_mute} onChange={(v) => updateSetting('system_mute', v)} />),
    },
    {
      key: 'hotkey',
      label: <Space><KeyOutlined />{t('settings.sectionHotkey')}</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          <Space.Compact style={{ width: '100%' }}>
            <Input
              placeholder={recordingHotkey ? t('settings.hotkeyPlaceholderRecording') : t('settings.hotkeyPlaceholderIdle')}
              value={hotkeyInput}
              readOnly
              onKeyDown={recordingHotkey ? handleHotkeyKeyDown : undefined}
              style={recordingHotkey ? { borderColor: '#1677ff', boxShadow: '0 0 0 2px rgba(22,119,255,0.2)' } : {}}
            />
            <Button type={recordingHotkey ? 'default' : 'primary'} onClick={() => setRecordingHotkey(!recordingHotkey)}>
              {recordingHotkey ? t('settings.hotkeyCancel') : t('settings.hotkeyRecord')}
            </Button>
            <Button onClick={handleClearHotkey}>{t('settings.hotkeyClear')}</Button>
          </Space.Compact>
        </Flex>
      ),
    },
    {
      key: 'tray',
      label: <Space><AppstoreOutlined />{t('settings.sectionTray')}</Space>,
      children: settingRow(t('settings.menuBarMode'), <Switch checked={settings.menu_bar_mode} onChange={(v) => updateSetting('menu_bar_mode', v)} />),
    },
    {
      key: 'history',
      label: <Space><HistoryOutlined />{t('settings.sectionHistory')}</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(t('settings.autoCleanup'), <Switch checked={settings.auto_cleanup} onChange={(v) => updateSetting('auto_cleanup', v)} />)}
          {settingRow(t('settings.autoCleanupDays'), <InputNumber min={1} max={365} value={settings.auto_cleanup_days ?? 30} onChange={(v) => v && updateSetting('auto_cleanup_days', v)} style={{ width: 120 }} />)}
          <Popconfirm title={t('settings.confirmClearHistory')} onConfirm={handleClearAllHistory} okText={t('common.confirm')} cancelText={t('common.cancel')}>
            <Button danger icon={<ClearOutlined />}>{t('settings.clearAllHistory')}</Button>
          </Popconfirm>
        </Flex>
      ),
    },
    {
      key: 'general',
      label: <Space data-testid="settings-general"><SettingOutlined />{t('settings.sectionGeneral')}</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(t('settings.language'),
            <Select
              value={(settings.ui_locale as string) || 'system'}
              onChange={async (v: string) => {
                await updateSetting('ui_locale', v);
                const resolved = await getResolvedLocale(v as UiLocale);
                i18n.changeLanguage(resolved);
                emit('language-changed', resolved);
              }}
              style={{ width: 250 }}
              data-testid="language-select"
              options={[
                { value: 'system', label: t('settings.langFollowSystem') },
                { value: 'zh-CN', label: t('settings.langChinese'), 'data-testid': 'language-zh' },
                { value: 'en', label: t('settings.langEnglish'), 'data-testid': 'language-en' },
              ]}
            />,
          )}
          {settingRow(t('settings.autostart'), <Switch checked={settings.autostart} onChange={(v) => updateSetting('autostart', v)} />)}
          {settingRow(t('settings.dataPath'), <Text type="secondary" copyable>{settings.data_path || t('settings.dataPathNotSet')}</Text>)}
        </Flex>
      ),
    },
  ];

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Typography.Title level={3}>{t('settings.title')}</Typography.Title>
      <Collapse items={items} defaultActiveKey={['audio', 'hotkey']} />
    </Flex>
  );
}
