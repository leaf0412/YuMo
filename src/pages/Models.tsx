import { useEffect, useState, useCallback } from 'react';
import {
  Card, Button, Flex, Space, Tag, Typography, Row, Col, Progress, Select,
  Input, message, Divider, Badge,
} from 'antd';
import {
  DownloadOutlined, DeleteOutlined, CheckCircleOutlined,
  CloudOutlined, ImportOutlined, ApiOutlined,
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const { Title, Text } = Typography;

interface ModelInfo {
  id: string;
  name: string;
  size_mb: number;
  languages: string[];
  download_url: string;
  is_downloaded: boolean;
}

interface DownloadProgress {
  model_id: string;
  progress: number;
}

interface Settings {
  language?: string;
  selected_model_id?: string;
  cloud_provider?: string;
  cloud_api_key?: string;
}

const CLOUD_PROVIDERS = [
  { value: 'openai', label: 'OpenAI Whisper' },
  { value: 'deepgram', label: 'Deepgram' },
  { value: 'assemblyai', label: 'AssemblyAI' },
];

function formatSize(mb: number): string {
  if (mb >= 1000) {
    return `${(mb / 1000).toFixed(1)} GB`;
  }
  return `${mb} MB`;
}

function languageLabel(lang: string): string {
  switch (lang) {
    case 'en': return 'English';
    case 'multi': return '多语言';
    default: return lang;
  }
}

export default function Models() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [settings, setSettings] = useState<Settings>({});
  const [cloudApiKey, setCloudApiKey] = useState('');

  const loadModels = useCallback(async () => {
    try {
      const result = await invoke<ModelInfo[]>('list_available_models');
      setModels(result);
    } catch { /* ignore */ }
  }, []);

  const loadSettings = useCallback(async () => {
    try {
      const result = await invoke<Settings>('get_settings');
      setSettings(result);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadModels();
    loadSettings();
  }, [loadModels, loadSettings]);

  useEffect(() => {
    const unlisten = listen<DownloadProgress>('model-download-progress', (event) => {
      const { model_id, progress } = event.payload;
      setDownloadProgress((prev) => ({ ...prev, [model_id]: progress }));
      if (progress >= 100) {
        loadModels();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadModels]);

  const handleDownload = async (modelId: string) => {
    try {
      setDownloadProgress((prev) => ({ ...prev, [modelId]: 0 }));
      await invoke('download_model', { modelId });
      message.success('下载完成');
      loadModels();
    } catch {
      message.error('下载失败');
    }
  };

  const handleDeleteModel = async (modelId: string) => {
    try {
      await invoke('delete_model', { modelId });
      message.success('已删除');
      loadModels();
    } catch {
      message.error('删除失败');
    }
  };

  const handleSelect = async (modelId: string) => {
    try {
      await invoke('select_model', { modelId });
      setSettings((prev) => ({ ...prev, selected_model_id: modelId }));
      message.success('已切换模型');
    } catch {
      message.error('切换失败');
    }
  };

  const handleImport = async () => {
    try {
      await invoke('import_model');
      message.success('导入完成');
      loadModels();
    } catch {
      message.error('导入失败');
    }
  };

  const handleLanguageChange = async (value: string) => {
    try {
      await invoke('update_setting', { key: 'language', value });
      setSettings((prev) => ({ ...prev, language: value }));
    } catch {
      message.error('设置失败');
    }
  };

  const handleCloudProviderChange = async (value: string) => {
    try {
      await invoke('update_setting', { key: 'cloud_provider', value });
      setSettings((prev) => ({ ...prev, cloud_provider: value }));
    } catch {
      message.error('设置失败');
    }
  };

  const handleSaveApiKey = async () => {
    try {
      await invoke('update_setting', { key: 'cloud_api_key', value: cloudApiKey });
      message.success('API Key 已保存');
    } catch {
      message.error('保存失败');
    }
  };

  const handleTestConnection = async () => {
    try {
      await invoke('test_cloud_connection');
      message.success('连接成功');
    } catch {
      message.error('连接失败');
    }
  };

  const isSelected = (modelId: string) => settings.selected_model_id === modelId;

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Title level={3} style={{ margin: 0 }}>模型管理</Title>
        <Space>
          <Select
            placeholder="语言"
            value={settings.language}
            onChange={handleLanguageChange}
            style={{ width: 160 }}
            options={[
              { value: 'auto', label: '自动检测' },
              { value: 'zh', label: '中文' },
              { value: 'en', label: 'English' },
              { value: 'ja', label: '日本語' },
            ]}
          />
          <Button icon={<ImportOutlined />} onClick={handleImport}>
            导入模型
          </Button>
        </Space>
      </div>

      <Title level={4}>本地模型</Title>
      <Row gutter={[16, 16]}>
        {models.map((model) => {
          const selected = isSelected(model.id);
          const progress = downloadProgress[model.id];
          const downloading = progress !== undefined && progress < 100;

          return (
            <Col xs={24} sm={12} md={8} key={model.id}>
              <Badge.Ribbon
                text={model.is_downloaded ? '已下载' : '未下载'}
                color={model.is_downloaded ? 'green' : 'default'}
              >
                <Card
                  title={
                    <Space>
                      <ApiOutlined />
                      <span>{model.name}</span>
                    </Space>
                  }
                  extra={selected && <Tag color="green" icon={<CheckCircleOutlined />}>使用中</Tag>}
                  style={selected ? { borderColor: '#52c41a' } : undefined}
                >
                  <Flex vertical gap={8} style={{ width: '100%' }}>
                    <div>
                      <Text type="secondary">大小: </Text>
                      <Text strong>{formatSize(model.size_mb)}</Text>
                    </div>
                    <div>
                      <Text type="secondary">语言: </Text>
                      {model.languages.map((lang) => (
                        <Tag key={lang} color="blue">{languageLabel(lang)}</Tag>
                      ))}
                    </div>

                    {downloading && (
                      <Progress percent={Math.round(progress)} size="small" />
                    )}

                    <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
                      {model.is_downloaded ? (
                        <>
                          {!selected && (
                            <Button type="primary" size="small" onClick={() => handleSelect(model.id)}>
                              使用此模型
                            </Button>
                          )}
                          <Button
                            danger
                            size="small"
                            icon={<DeleteOutlined />}
                            onClick={() => handleDeleteModel(model.id)}
                          >
                            删除
                          </Button>
                        </>
                      ) : (
                        <Button
                          type="primary"
                          size="small"
                          icon={<DownloadOutlined />}
                          loading={downloading}
                          onClick={() => handleDownload(model.id)}
                        >
                          下载
                        </Button>
                      )}
                    </Space>
                  </Flex>
                </Card>
              </Badge.Ribbon>
            </Col>
          );
        })}
      </Row>

      <Divider />

      <Title level={4}>云端模型</Title>
      <Card>
        <Flex vertical gap={8} style={{ width: '100%' }}>
          <div>
            <Text>服务商</Text>
            <Select
              placeholder="选择云端服务商"
              value={settings.cloud_provider}
              onChange={handleCloudProviderChange}
              style={{ width: '100%', marginTop: 8 }}
              options={CLOUD_PROVIDERS}
            />
          </div>
          <div>
            <Text>API Key</Text>
            <Space.Compact style={{ width: '100%', marginTop: 8 }}>
              <Input.Password
                placeholder="输入 API Key"
                value={cloudApiKey}
                onChange={(e) => setCloudApiKey(e.target.value)}
              />
              <Button onClick={handleSaveApiKey}>保存</Button>
            </Space.Compact>
          </div>
          <Button icon={<CloudOutlined />} onClick={handleTestConnection}>
            测试连接
          </Button>
        </Flex>
      </Card>
    </Flex>
  );
}
