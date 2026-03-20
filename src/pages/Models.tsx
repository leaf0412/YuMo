import { useEffect, useState } from 'react';
import {
  Card, Button, Flex, Space, Tag, Typography, Row, Col, Select,
  Input, InputNumber, Slider, Progress, message, Divider, Tabs, Badge,
} from 'antd';
import {
  CheckCircleOutlined, CloudOutlined, ImportOutlined, ThunderboltOutlined,
} from '@ant-design/icons';
import { listen } from '@tauri-apps/api/event';
import { invoke, formatError, logEvent } from '../lib/logger';
import useAppStore from '../stores/useAppStore';
const { Title, Text } = Typography;

interface ModelSettings {
  temperature: number;
  max_tokens: number;
}

const DEFAULT_MODEL_SETTINGS: ModelSettings = { temperature: 0, max_tokens: 1900 };

const CLOUD_PROVIDERS = [
  { value: 'openai', label: 'OpenAI Whisper' },
  { value: 'deepgram', label: 'Deepgram' },
  { value: 'assemblyai', label: 'AssemblyAI' },
];

function formatSize(mb: number): string {
  return mb >= 1000 ? `${(mb / 1000).toFixed(1)} GB` : `${mb} MB`;
}

function languageLabel(lang: string): string {
  switch (lang) {
    case 'en': return 'English';
    case 'multi': return '多语言';
    default: return lang;
  }
}

export default function Models() {
  const { models, settings, daemonStatus, fetchModels, fetchSettings, fetchDaemonStatus, setSettings: storeSetSettings, setDaemonStatus: storeSetDaemonStatus } = useAppStore();
  const [cloudApiKey, setCloudApiKey] = useState('');
  const [activeTab, setActiveTab] = useState('mlx');
  const [loadingModel, setLoadingModel] = useState<string | null>(null);
  const [daemonBusy, setDaemonBusy] = useState(false);
  const [setupMessage, setSetupMessage] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [modelSettings, setModelSettings] = useState<Record<string, ModelSettings>>({});

  const getModelSettings = (modelId: string): ModelSettings =>
    modelSettings[modelId] ?? DEFAULT_MODEL_SETTINGS;

  const handleModelSettingChange = async (modelId: string, key: keyof ModelSettings, value: number) => {
    const settingKey = `model_${modelId}_${key}`;
    try {
      await invoke('update_setting', { key: settingKey, value: JSON.stringify(value) });
      setModelSettings((prev) => ({
        ...prev,
        [modelId]: { ...getModelSettings(modelId), [key]: value },
      }));
    } catch (e) {
      message.error(formatError(e, '设置失败'));
    }
  };

  // Extract per-model settings whenever store settings change
  useEffect(() => {
    const ms: Record<string, ModelSettings> = {};
    for (const [k, v] of Object.entries(settings)) {
      const match = k.match(/^model_(.+)_(temperature|max_tokens)$/);
      if (match) {
        const [, modelId, field] = match;
        if (!ms[modelId]) ms[modelId] = { ...DEFAULT_MODEL_SETTINGS };
        ms[modelId][field as keyof ModelSettings] = typeof v === 'string' ? parseFloat(v as string) : Number(v);
      }
    }
    setModelSettings(ms);
  }, [settings]);

  useEffect(() => {
    fetchModels();
    fetchSettings();
  }, [fetchModels, fetchSettings]);

  // Listen for model download progress events
  useEffect(() => {
    const lastLoggedProgress: Record<string, number> = {};
    const unlisten = listen<{ model_repo: string; progress: number }>('model-download-progress', (event) => {
      const { model_repo, progress } = event.payload;
      setDownloadProgress((prev) => ({ ...prev, [model_repo]: progress }));
      const bucket = Math.floor(progress / 10) * 10;
      if ((lastLoggedProgress[model_repo] ?? -1) < bucket) {
        lastLoggedProgress[model_repo] = bucket;
        logEvent('Models', 'download_progress', { repo: model_repo, progress: bucket });
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  useEffect(() => {
    if (activeTab !== 'mlx') return;
    fetchDaemonStatus();
    const interval = setInterval(fetchDaemonStatus, 3000);
    return () => clearInterval(interval);
  }, [activeTab, fetchDaemonStatus]);

  // Listen for daemon setup status (venv bootstrap, etc.)
  useEffect(() => {
    const unlisten = listen<{ stage: string; message?: string }>('daemon-setup-status', (event) => {
      const { stage, message: msg } = event.payload;
      logEvent('Models', 'daemon_setup_stage', { stage });
      if (stage === 'ready') {
        setSetupMessage(null);
      } else if (msg) {
        setSetupMessage(msg);
      } else if (stage === 'checking_python') {
        setSetupMessage('检查 Python 环境...');
      } else if (stage === 'starting_daemon') {
        setSetupMessage('启动 Daemon...');
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);


  const handleSelect = async (modelId: string) => {
    logEvent('Models', 'select_model', { model_id: modelId });
    try {
      await invoke('select_model', { modelId });
      storeSetSettings({ selected_model_id: modelId });
      message.success('已切换模型');
    } catch (e) {
      message.error(formatError(e, '切换失败'));
    }
  };

  const handleImport = async () => {
    try {
      const imported = await invoke<boolean>('import_model');
      if (imported) {
        message.success('导入完成');
        fetchModels();
      }
    } catch (e) {
      message.error(formatError(e, '导入失败'));
    }
  };

  const handleLanguageChange = async (value: string) => {
    logEvent('Models', 'language_change', { value });
    try {
      await invoke('update_setting', { key: 'language', value });
      storeSetSettings({ language: value });
    } catch (e) {
      message.error(formatError(e, '设置失败'));
    }
  };

  const handleCloudProviderChange = async (value: string) => {
    try {
      await invoke('update_setting', { key: 'cloud_provider', value });
      storeSetSettings({ cloud_provider: value });
    } catch (e) {
      message.error(formatError(e, '设置失败'));
    }
  };

  const handleSaveApiKey = async () => {
    logEvent('Models', 'save_api_key', { provider: settings.cloud_provider ?? 'unknown' });
    try {
      await invoke('update_setting', { key: 'cloud_api_key', value: cloudApiKey });
      message.success('API Key 已保存');
    } catch (e) {
      message.error(formatError(e, '保存失败'));
    }
  };

  const handleTestConnection = () => {
    message.info('云端连接测试暂未实现');
  };

  const handleDaemonStart = async () => {
    if (daemonBusy) return;
    logEvent('Models', 'daemon_start');
    setDaemonBusy(true);
    try {
      await invoke('daemon_start');
      message.success('Daemon 已启动');
      fetchDaemonStatus();
    } catch (e) {
      message.error(formatError(e, 'Daemon 启动失败，请检查 Python 3 和 mlx-audio 是否已安装'));
    } finally {
      setDaemonBusy(false);
    }
  };

  const handleDaemonStop = async () => {
    logEvent('Models', 'daemon_stop');
    try {
      await invoke('daemon_stop');
      await invoke('update_setting', { key: 'selected_model_id', value: '' });
      storeSetDaemonStatus({ running: false, loaded_model: null });
      storeSetSettings({ selected_model_id: undefined });
      message.success('Daemon 已停止');
    } catch (e) {
      message.error(formatError(e, '停止失败'));
    }
  };

  const handleLoadModel = async (modelRepo: string, modelId: string) => {
    if (loadingModel || daemonBusy) return;
    logEvent('Models', 'load_model_start', { model_id: modelId, repo: modelRepo });
    setLoadingModel(modelId);
    setDaemonBusy(true);
    try {
      await invoke('daemon_load_model', { modelRepo });
      await invoke('select_model', { modelId });
      storeSetSettings({ selected_model_id: modelId });
      logEvent('Models', 'load_model_complete', { model_id: modelId });
      message.success('模型已加载');
      fetchDaemonStatus();
      fetchModels();
    } catch (e) {
      logEvent('Models', 'load_model_error', { model_id: modelId, error: formatError(e, 'unknown') });
      message.error(formatError(e, '模型加载失败'));
    } finally {
      setLoadingModel(null);
      setDaemonBusy(false);
      setSetupMessage(null);
      setDownloadProgress((prev) => {
        const next = { ...prev };
        delete next[modelRepo];
        return next;
      });
    }
  };

  const handleDeleteModel = async (modelId: string) => {
    logEvent('Models', 'delete_model', { model_id: modelId });
    try {
      await invoke('delete_model', { modelId });
      storeSetDaemonStatus({ ...daemonStatus, loaded_model: null });
      storeSetSettings({ selected_model_id: undefined });
      fetchModels();
      message.success('模型已删除');
    } catch (e) {
      message.error(formatError(e, '删除失败'));
    }
  };

  const isSelected = (modelId: string) => settings.selected_model_id === modelId;
  const LOCAL_PROVIDERS = ['local'];
  const MLX_PROVIDERS = ['mlxWhisper', 'mlxFunASR'];
  const CLOUD_PROVIDERS_LIST = ['groq', 'deepgram', 'elevenLabs', 'mistral', 'gemini', 'soniox'];

  const localModels = models.filter(m => LOCAL_PROVIDERS.includes(m.provider));
  const mlxModels = models.filter(m => MLX_PROVIDERS.includes(m.provider));
  const cloudModels = models.filter(m => CLOUD_PROVIDERS_LIST.includes(m.provider));

  // Local whisper models tab hidden — kept for future use
  void localModels;

  const mlxTabContent = (
    <>
      <Flex justify="space-between" align="center" style={{ marginBottom: 16, padding: '12px 16px', background: '#fafafa', borderRadius: 8 }}>
        <Space>
          <Badge status={daemonStatus.running ? (daemonStatus.loaded_model ? 'success' : 'warning') : 'default'} />
          <Text>{daemonStatus.running ? (daemonStatus.loaded_model ? 'Daemon 运行中' : 'Daemon 空闲') : 'Daemon 未启动'}</Text>
          {daemonStatus.loaded_model && <Tag color="blue">已加载: {daemonStatus.loaded_model.split('/').pop()}</Tag>}
        </Space>
        <Space>
          {daemonStatus.running
            ? <Button size="small" onClick={handleDaemonStop}>停止</Button>
            : <Button type="primary" size="small" loading={daemonBusy} onClick={handleDaemonStart}>启动 Daemon</Button>}
        </Space>
      </Flex>
      <Row gutter={[16, 16]}>
        {mlxModels.map((model) => (
          <Col xs={24} sm={12} md={8} key={model.id}>
            <Card style={isSelected(model.id) ? { borderColor: '#52c41a' } : undefined} styles={{ body: { padding: 16 } }}>
              <Flex vertical gap={12}>
                <Flex justify="space-between" align="center">
                  <Space><ThunderboltOutlined /><Text strong>{model.name}</Text></Space>
                  {daemonStatus.loaded_model === model.model_repo
                    ? <Tag color="blue" icon={isSelected(model.id) ? <CheckCircleOutlined /> : undefined}>{isSelected(model.id) ? '使用中 · 已加载' : '已加载'}</Tag>
                    : model.is_downloaded
                      ? <Tag color="green" icon={isSelected(model.id) ? <CheckCircleOutlined /> : undefined}>{isSelected(model.id) ? '使用中 · 已缓存' : '已缓存'}</Tag>
                      : isSelected(model.id)
                        ? <Tag color="red">需要下载</Tag>
                        : <Tag>未下载</Tag>}
                </Flex>
                {model.description && <Text type="secondary" style={{ fontSize: 12 }}>{model.description}</Text>}
                <Flex gap={16}>
                  <div><Text type="secondary">大小: </Text><Text>{formatSize(model.size_mb)}</Text></div>
                  <div><Text type="secondary">语言: </Text>{model.languages.map((lang) => <Tag key={lang} color="blue" bordered={false}>{languageLabel(lang)}</Tag>)}</div>
                </Flex>
                {model.is_downloaded && (
                  <Flex vertical gap={4} style={{ padding: '8px 0' }}>
                    <Flex align="center" gap={8}>
                      <Text type="secondary" style={{ fontSize: 12, minWidth: 52 }}>温度:</Text>
                      <Slider
                        min={0} max={1} step={0.1}
                        value={getModelSettings(model.id).temperature}
                        onChange={(v) => handleModelSettingChange(model.id, 'temperature', v)}
                        style={{ flex: 1 }}
                      />
                      <Text style={{ fontSize: 12, minWidth: 28 }}>{getModelSettings(model.id).temperature}</Text>
                    </Flex>
                    <Flex align="center" gap={8}>
                      <Text type="secondary" style={{ fontSize: 12, minWidth: 52 }}>Token:</Text>
                      <InputNumber
                        size="small"
                        min={100} max={10000} step={100}
                        value={getModelSettings(model.id).max_tokens}
                        onChange={(v) => v != null && handleModelSettingChange(model.id, 'max_tokens', v)}
                        style={{ flex: 1 }}
                      />
                    </Flex>
                  </Flex>
                )}
                {loadingModel === model.id && setupMessage && (
                  <Text type="warning" style={{ fontSize: 12 }}>{setupMessage}</Text>
                )}
                {model.model_repo && downloadProgress[model.model_repo] != null && (
                  <Progress percent={downloadProgress[model.model_repo]} size="small" status="active" />
                )}
                <Flex justify="flex-end" gap={8}>
                  {daemonStatus.loaded_model === model.model_repo ? (
                    <>
                      {!isSelected(model.id) && <Button type="primary" size="small" onClick={() => handleSelect(model.id)}>设为默认</Button>}
                      <Button size="small" danger onClick={() => handleDeleteModel(model.id)}>删除模型</Button>
                    </>
                  ) : model.is_downloaded ? (
                    <>
                      <Button type="primary" size="small" onClick={() => handleLoadModel(model.model_repo!, model.id)}>加载模型</Button>
                      <Button size="small" danger onClick={() => handleDeleteModel(model.id)}>删除</Button>
                    </>
                  ) : (
                    <Button type="primary" size="small" loading={loadingModel === model.id} onClick={() => handleLoadModel(model.model_repo!, model.id)}>
                      {loadingModel === model.id
                        ? (model.model_repo && downloadProgress[model.model_repo] != null ? '下载中...' : '加载中...')
                        : '加载模型'}
                    </Button>
                  )}
                </Flex>
              </Flex>
            </Card>
          </Col>
        ))}
      </Row>
    </>
  );

  const cloudTabContent = (
    <>
      <Row gutter={[16, 16]}>
        {cloudModels.map((model) => (
          <Col xs={24} sm={12} md={8} key={model.id}>
            <Card style={isSelected(model.id) ? { borderColor: '#52c41a' } : undefined} styles={{ body: { padding: 16 } }}>
              <Flex vertical gap={12}>
                <Flex justify="space-between" align="center">
                  <Space><CloudOutlined /><Text strong>{model.name}</Text></Space>
                  {isSelected(model.id) ? (
                    <Tag color="green" icon={<CheckCircleOutlined />}>使用中</Tag>
                  ) : (
                    <Tag color="blue">云端可用</Tag>
                  )}
                </Flex>
                {model.description && <Text type="secondary" style={{ fontSize: 12 }}>{model.description}</Text>}
                <Flex vertical gap={4} style={{ padding: '8px 0' }}>
                  <Flex align="center" gap={8}>
                    <Text type="secondary" style={{ fontSize: 12, minWidth: 52 }}>温度:</Text>
                    <Slider
                      min={0} max={1} step={0.1}
                      value={getModelSettings(model.id).temperature}
                      onChange={(v) => handleModelSettingChange(model.id, 'temperature', v)}
                      style={{ flex: 1 }}
                    />
                    <Text style={{ fontSize: 12, minWidth: 28 }}>{getModelSettings(model.id).temperature}</Text>
                  </Flex>
                  <Flex align="center" gap={8}>
                    <Text type="secondary" style={{ fontSize: 12, minWidth: 52 }}>Token:</Text>
                    <InputNumber
                      size="small"
                      min={100} max={10000} step={100}
                      value={getModelSettings(model.id).max_tokens}
                      onChange={(v) => v != null && handleModelSettingChange(model.id, 'max_tokens', v)}
                      style={{ flex: 1 }}
                    />
                  </Flex>
                </Flex>
                <Flex justify="flex-end" gap={8}>
                  {!isSelected(model.id) && (
                    <Button type="primary" size="small" onClick={() => handleSelect(model.id)}>使用此模型</Button>
                  )}
                </Flex>
              </Flex>
            </Card>
          </Col>
        ))}
      </Row>
      <Divider />
      <Card title="API 配置">
        <Flex vertical gap={8} style={{ width: '100%' }}>
          <div>
            <Text>服务商</Text>
            <Select placeholder="选择云端服务商" value={settings.cloud_provider} onChange={handleCloudProviderChange} style={{ width: '100%', marginTop: 8 }} options={CLOUD_PROVIDERS} />
          </div>
          <div>
            <Text>API Key</Text>
            <Space.Compact style={{ width: '100%', marginTop: 8 }}>
              <Input.Password placeholder="输入 API Key" value={cloudApiKey} onChange={(e) => setCloudApiKey(e.target.value)} />
              <Button onClick={handleSaveApiKey}>保存</Button>
            </Space.Compact>
          </div>
          <Button icon={<CloudOutlined />} onClick={handleTestConnection}>测试连接</Button>
        </Flex>
      </Card>
    </>
  );

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={3} style={{ margin: 0 }}>模型管理</Title>
        <Space>
          <Select placeholder="语言" value={settings.language} onChange={handleLanguageChange} style={{ width: 160 }}
            options={[
              { value: 'auto', label: '自动检测' },
              { value: 'zh', label: '中文' },
              { value: 'en', label: 'English' },
              { value: 'ja', label: '日本語' },
            ]}
          />
          <Button icon={<ImportOutlined />} onClick={handleImport}>导入模型</Button>
        </Space>
      </div>
      <Tabs activeKey={activeTab} onChange={setActiveTab}
        items={[
          { key: 'mlx', label: `MLX 模型 (${mlxModels.length})`, children: mlxTabContent },
          { key: 'cloud', label: `云端模型 (${cloudModels.length})`, children: cloudTabContent },
        ]}
      />
    </Flex>
  );
}
