import { useEffect, useState, useCallback, useRef } from 'react';
import {
  Collapse, Switch, Slider, Select, Input, Button, Flex, Space, Typography,
  message, Popconfirm, InputNumber,
} from 'antd';
import {
  AudioOutlined, CopyOutlined,
  DesktopOutlined, KeyOutlined,
  HistoryOutlined, SettingOutlined, ClearOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { emit, listen } from '../lib/events';
import i18n from '../i18n';
import { getResolvedLocale, type UiLocale } from '../i18n/utils';
import { invoke, formatError, logEvent } from '../lib/logger';

const { Text } = Typography;

interface ImportResult {
  transcriptions_imported: number;
  transcriptions_skipped: number;
  vocabulary_imported: number;
  replacements_imported: number;
  recordings_copied: number;
}

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

  const detectLegacyPath = useCallback(async () => {
    try {
      const path = await invoke<string | null>('detect_voiceink_legacy_path');
      setLegacyPath(path);
    } catch { /* ignore */ }
  }, []);

  const [pythonPath, setPythonPath] = useState('');
  const [pythonPathDirty, setPythonPathDirty] = useState(false);

  const loadPythonPath = useCallback(async () => {
    try {
      const path = await invoke<string>('get_python_path');
      setPythonPath(path);
    } catch { /* logged */ }
  }, []);

  useEffect(() => {
    loadSettings();
    loadDevices();
    detectLegacyPath();
    loadPythonPath();
  }, [loadSettings, loadDevices, detectLegacyPath, loadPythonPath]);

  // Listen for device hot-plug/unplug events from backend
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<AudioDevice[]>('devices-changed', (event) => {
      setAudioDevices(event.payload);
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, []);

  const updateSetting = async (key: string, value: unknown) => {
    try {
      await invoke('update_setting', { key, value });
      setSettings((prev) => ({ ...prev, [key]: value }));
      logEvent('Settings', 'setting_changed', { key, value });
    } catch (e) {
      message.error(formatError(e, t('settings.updateFailed')));
    }
  };

  const savePythonPath = async () => {
    try {
      await invoke('set_python_path', { path: pythonPath });
      setPythonPathDirty(false);
      message.success(t('settings.pythonPathSaved'));
    } catch (e) {
      message.error(formatError(e, t('settings.updateFailed')));
    }
  };

  const detectPythonPath = async () => {
    try {
      const path = await invoke<string>('get_python_path');
      setPythonPath(path);
      await invoke('set_python_path', { path });
      setPythonPathDirty(false);
      message.success(t('settings.pythonPathSaved'));
    } catch (e) {
      message.error(formatError(e, t('settings.updateFailed')));
    }
  };

  const [recordingHotkey, setRecordingHotkey] = useState(false);
  const hadNonModifierRef = useRef(false);

  // Data import state
  const [legacyPath, setLegacyPath] = useState<string | null>(null);
  const [importing, setImporting] = useState(false);

  const MODIFIER_KEYS = ['Meta', 'Control', 'Alt', 'Shift'];

  const KEY_MAP: Record<string, string> = {
    ' ': 'Space', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right',
    Enter: 'Enter', Backspace: 'Backspace', Delete: 'Delete', Escape: 'Escape',
    Tab: 'Tab', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
  };

  const modifierKeyToShortcut = (key: string): string => {
    if (key === 'Meta' || key === 'Control') return 'CommandOrControl';
    if (key === 'Alt') return 'Alt';
    return 'Shift';
  };

  const keyEventToShortcut = (e: React.KeyboardEvent): string | null => {
    const parts: string[] = [];
    if (e.metaKey || e.ctrlKey) parts.push('CommandOrControl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');

    const key = e.key;
    if (MODIFIER_KEYS.includes(key)) return null;

    const mapped = KEY_MAP[key] || (key.length === 1 ? key.toUpperCase() : key);
    parts.push(mapped);

    return parts.join('+') || null;
  };

  const registerShortcut = async (shortcut: string) => {
    setHotkeyInput(shortcut);
    setRecordingHotkey(false);
    logEvent('Settings', 'hotkey_captured', { shortcut });
    try {
      await invoke('register_hotkey', { shortcut });
      updateSetting('hotkey', shortcut);
      logEvent('Settings', 'hotkey_registered', { shortcut });
      message.success(t('settings.hotkeySet', { shortcut }));
    } catch (e: unknown) {
      message.error(formatError(e, t('settings.registerFailed')));
    }
  };

  const handleHotkeyKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (MODIFIER_KEYS.includes(e.key)) {
      hadNonModifierRef.current = false;
      return;
    }
    hadNonModifierRef.current = true;
    const shortcut = keyEventToShortcut(e);
    if (shortcut) registerShortcut(shortcut);
  };

  const handleHotkeyKeyUp = (e: React.KeyboardEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (MODIFIER_KEYS.includes(e.key) && !hadNonModifierRef.current) {
      registerShortcut(modifierKeyToShortcut(e.key));
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

  const showImportResult = (result: ImportResult) => {
    let msg = t('settings.importSuccess', {
      transcriptions: result.transcriptions_imported,
      vocabulary: result.vocabulary_imported,
      replacements: result.replacements_imported,
      recordings: result.recordings_copied,
    });
    if (result.transcriptions_skipped > 0) {
      msg += t('settings.importSkipped', { skipped: result.transcriptions_skipped });
    }
    message.success(msg);
  };

  const handleImportLegacyAuto = async () => {
    if (!legacyPath) return;
    setImporting(true);
    try {
      const result = await invoke<ImportResult>('import_voiceink_legacy', { storePath: legacyPath });
      logEvent('Settings', 'import_legacy_auto', { ...result });
      showImportResult(result);
    } catch (e) {
      message.error(formatError(e, t('settings.importFailed')));
    }
    setImporting(false);
  };

  const handleImportLegacyManual = async () => {
    setImporting(true);
    try {
      const result = await invoke<ImportResult>('import_voiceink_from_dialog');
      logEvent('Settings', 'import_legacy_manual', { ...result });
      showImportResult(result);
    } catch (e) {
      message.error(formatError(e, t('settings.importFailed')));
    }
    setImporting(false);
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
            <Select value={settings.audio_device ?? 0} onChange={(v) => updateSetting('audio_device', v)} style={{ width: 250 }} placeholder={t('settings.audioDevicePlaceholder')} options={[{ value: 0, label: t('settings.audioDeviceDefault') }, ...audioDevices.map((d) => ({ value: d.id, label: d.name }))]} />,
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
          {settingRow(t('settings.autoCapitalize'), <Switch checked={settings.auto_capitalize} onChange={(v) => updateSetting('auto_capitalize', v)} />)}
        </Flex>
      ),
    },
    {
      key: 'system',
      label: <Space><DesktopOutlined />{t('settings.sectionSystem')}</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(t('settings.systemMute'), <Switch checked={settings.system_mute} onChange={(v) => updateSetting('system_mute', v)} />)}
          {settingRow(t('settings.menuBarMode'), <Switch checked={settings.menu_bar_mode} onChange={(v) => updateSetting('menu_bar_mode', v)} />)}
          <div style={{ padding: '8px 0' }}>
            <Text>{t('settings.pythonPath')}</Text>
            <Space.Compact style={{ width: '100%', marginTop: 4 }}>
              <Input
                value={pythonPath}
                onChange={(e) => { setPythonPath(e.target.value); setPythonPathDirty(true); }}
                placeholder={t('settings.pythonPathPlaceholder')}
              />
              <Button onClick={detectPythonPath}>{t('settings.pythonPathDetect')}</Button>
              {pythonPathDirty && <Button type="primary" onClick={savePythonPath}>{t('common.confirm')}</Button>}
            </Space.Compact>
          </div>
        </Flex>
      ),
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
              onKeyUp={recordingHotkey ? handleHotkeyKeyUp : undefined}
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
          <div style={{ padding: '8px 0' }}>
            <Text strong>{t('settings.sectionImport')}</Text>
          </div>
          {legacyPath ? (
            <>
              <Text type="success">{t('settings.importLegacyDetected')}</Text>
              <Text type="secondary" copyable style={{ fontSize: 12 }}>{legacyPath}</Text>
              <Space>
                <Button type="primary" onClick={handleImportLegacyAuto} loading={importing}>
                  {t('settings.importLegacyAuto')}
                </Button>
                <Button onClick={handleImportLegacyManual} loading={importing}>
                  {t('settings.importLegacyManual')}
                </Button>
              </Space>
            </>
          ) : (
            <>
              <Text type="secondary">{t('settings.importLegacyNotDetected')}</Text>
              <Button onClick={handleImportLegacyManual} loading={importing}>
                {t('settings.importLegacyManual')}
              </Button>
            </>
          )}
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
