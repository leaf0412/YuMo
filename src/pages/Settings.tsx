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
import { invoke } from '@tauri-apps/api/core';

const { Text } = Typography;

interface AudioDevice {
  id: number;
  name: string;
  is_default: boolean;
}

interface AppSettings {
  audio_device?: string;
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
}

export default function Settings() {
  const [settings, setSettings] = useState<AppSettings>({});
  const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
  const [hotkeyInput, setHotkeyInput] = useState('');

  const loadSettings = useCallback(async () => {
    try {
      const result = await invoke<AppSettings>('get_settings');
      setSettings(result);
      setHotkeyInput(result.hotkey || '');
    } catch { /* ignore */ }
  }, []);

  const loadDevices = useCallback(async () => {
    try {
      const result = await invoke<AudioDevice[]>('list_audio_devices');
      setAudioDevices(result);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadSettings();
    loadDevices();
  }, [loadSettings, loadDevices]);

  const updateSetting = async (key: string, value: unknown) => {
    try {
      await invoke('update_setting', { key, value });
      setSettings((prev) => ({ ...prev, [key]: value }));
    } catch {
      message.error('设置更新失败');
    }
  };

  const handleRegisterHotkey = async () => {
    try {
      await invoke('register_hotkey', { shortcut: hotkeyInput });
      updateSetting('hotkey', hotkeyInput);
      message.success('快捷键已注册');
    } catch {
      message.error('注册失败');
    }
  };

  const handleClearHotkey = async () => {
    try {
      await invoke('unregister_hotkey');
      setHotkeyInput('');
      updateSetting('hotkey', '');
      message.success('快捷键已清除');
    } catch {
      message.error('清除失败');
    }
  };

  const handleClearAllHistory = async () => {
    try {
      await invoke('delete_all_transcriptions');
      message.success('已清空所有历史记录');
    } catch {
      message.error('清空失败');
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
      label: <Space><AudioOutlined />录音</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(
            '音频设备',
            <Select
              value={settings.audio_device}
              onChange={(v) => updateSetting('audio_device', v)}
              style={{ width: 250 }}
              placeholder="选择音频设备"
              options={audioDevices.map((d) => ({ value: d.id, label: d.name }))}
            />,
          )}
          {settingRow(
            '录音提示音',
            <Switch
              checked={settings.sound_enabled}
              onChange={(v) => updateSetting('sound_enabled', v)}
            />,
          )}
          {settingRow(
            '自定义提示音文件',
            <Input
              value={settings.custom_sound_file || ''}
              onChange={(e) => updateSetting('custom_sound_file', e.target.value)}
              placeholder="文件路径..."
              style={{ width: 250 }}
            />,
          )}
        </Flex>
      ),
    },
    {
      key: 'noise',
      label: <Space><FilterOutlined />降噪</Space>,
      children: settingRow(
        '启用降噪',
        <Switch
          checked={settings.noise_reduction}
          onChange={(v) => updateSetting('noise_reduction', v)}
        />,
      ),
    },
    {
      key: 'vad',
      label: <Space><ThunderboltOutlined />VAD 流式转录</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(
            '启用 VAD',
            <Switch
              checked={settings.vad_enabled}
              onChange={(v) => updateSetting('vad_enabled', v)}
            />,
          )}
          <div style={{ padding: '8px 0' }}>
            <Text>灵敏度</Text>
            <Slider
              min={0}
              max={100}
              value={settings.vad_sensitivity ?? 50}
              onChange={(v) => updateSetting('vad_sensitivity', v)}
            />
          </div>
          <div style={{ padding: '8px 0' }}>
            <Text>静音超时 (ms)</Text>
            <Slider
              min={100}
              max={5000}
              step={100}
              value={settings.vad_silence_timeout ?? 1000}
              onChange={(v) => updateSetting('vad_silence_timeout', v)}
            />
          </div>
        </Flex>
      ),
    },
    {
      key: 'paste',
      label: <Space><CopyOutlined />粘贴</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(
            '恢复剪贴板',
            <Switch
              checked={settings.clipboard_restore}
              onChange={(v) => updateSetting('clipboard_restore', v)}
            />,
          )}
          <div style={{ padding: '8px 0' }}>
            <Text>粘贴延迟 (ms)</Text>
            <Slider
              min={0}
              max={1000}
              step={50}
              value={settings.paste_delay ?? 100}
              onChange={(v) => updateSetting('paste_delay', v)}
            />
          </div>
        </Flex>
      ),
    },
    {
      key: 'format',
      label: <Space><FontSizeOutlined />文本格式化</Space>,
      children: settingRow(
        '自动大写',
        <Switch
          checked={settings.auto_capitalize}
          onChange={(v) => updateSetting('auto_capitalize', v)}
        />,
      ),
    },
    {
      key: 'system',
      label: <Space><DesktopOutlined />系统控制</Space>,
      children: settingRow(
        '录音时静音系统',
        <Switch
          checked={settings.system_mute}
          onChange={(v) => updateSetting('system_mute', v)}
        />,
      ),
    },
    {
      key: 'hotkey',
      label: <Space><KeyOutlined />快捷键</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          <Space.Compact style={{ width: '100%' }}>
            <Input
              placeholder="例如: CommandOrControl+Shift+Space"
              value={hotkeyInput}
              onChange={(e) => setHotkeyInput(e.target.value)}
            />
            <Button type="primary" onClick={handleRegisterHotkey}>注册</Button>
            <Button onClick={handleClearHotkey}>清除</Button>
          </Space.Compact>
        </Flex>
      ),
    },
    {
      key: 'tray',
      label: <Space><AppstoreOutlined />系统托盘</Space>,
      children: settingRow(
        '菜单栏模式',
        <Switch
          checked={settings.menu_bar_mode}
          onChange={(v) => updateSetting('menu_bar_mode', v)}
        />,
      ),
    },
    {
      key: 'history',
      label: <Space><HistoryOutlined />历史管理</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(
            '自动清理',
            <Switch
              checked={settings.auto_cleanup}
              onChange={(v) => updateSetting('auto_cleanup', v)}
            />,
          )}
          {settingRow(
            '保留天数',
            <InputNumber
              min={1}
              max={365}
              value={settings.auto_cleanup_days ?? 30}
              onChange={(v) => v && updateSetting('auto_cleanup_days', v)}
              style={{ width: 120 }}
            />,
          )}
          <Popconfirm
            title="确认清空所有历史记录？"
            onConfirm={handleClearAllHistory}
            okText="确认"
            cancelText="取消"
          >
            <Button danger icon={<ClearOutlined />}>
              清空所有记录
            </Button>
          </Popconfirm>
        </Flex>
      ),
    },
    {
      key: 'general',
      label: <Space><SettingOutlined />通用</Space>,
      children: (
        <Flex vertical gap={8} style={{ width: '100%' }}>
          {settingRow(
            '开机自启',
            <Switch
              checked={settings.autostart}
              onChange={(v) => updateSetting('autostart', v)}
            />,
          )}
          {settingRow(
            '数据目录',
            <Text type="secondary" copyable>{settings.data_path || '未设置'}</Text>,
          )}
        </Flex>
      ),
    },
  ];

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Typography.Title level={3}>设置</Typography.Title>
      <Collapse items={items} defaultActiveKey={['audio', 'hotkey']} />
    </Flex>
  );
}
