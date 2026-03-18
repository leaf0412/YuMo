import { useEffect, useState, useCallback } from 'react';
import {
  Card, Button, Space, Tag, Typography, Row, Col, Progress, Select,
  Input, message, Divider,
} from 'antd';
import {
  DownloadOutlined, DeleteOutlined, CheckCircleOutlined,
  CloudOutlined, ImportOutlined, ApiOutlined,
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const { Title, Text } = Typography;

interface LocalModel {
  name: string;
  size: string;
  languages: string[];
  downloaded: boolean;
  selected: boolean;
}

interface DownloadProgress {
  model: string;
  progress: number;
}

interface Settings {
  language?: string;
  cloud_provider?: string;
  cloud_api_key?: string;
}

const CLOUD_PROVIDERS = [
  { value: 'openai', label: 'OpenAI Whisper' },
  { value: 'deepgram', label: 'Deepgram' },
  { value: 'assemblyai', label: 'AssemblyAI' },
];

export default function Models() {
  const [models, setModels] = useState<LocalModel[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [settings, setSettings] = useState<Settings>({});
  const [cloudApiKey, setCloudApiKey] = useState('');

  const loadModels = useCallback(async () => {
    try {
      const result = await invoke<LocalModel[]>('list_models');
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
      const { model, progress } = event.payload;
      setDownloadProgress((prev) => ({ ...prev, [model]: progress }));
      if (progress >= 100) {
        loadModels();
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loadModels]);

  const handleDownload = async (name: string) => {
    try {
      setDownloadProgress((prev) => ({ ...prev, [name]: 0 }));
      await invoke('download_model', { name });
      message.success(`${name} 下载完成`);
      loadModels();
    } catch {
      message.error('下载失败');
    }
  };

  const handleDeleteModel = async (name: string) => {
    try {
      await invoke('delete_model', { name });
      message.success('已删除');
      loadModels();
    } catch {
      message.error('删除失败');
    }
  };

  const handleSelect = async (name: string) => {
    try {
      await invoke('select_model', { name });
      message.success('已切换模型');
      loadModels();
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

  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
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
        {models.map((model) => (
          <Col xs={24} sm={12} md={8} key={model.name}>
            <Card
              title={
                <Space>
                  <ApiOutlined />
                  <span>{model.name}</span>
                </Space>
              }
              extra={model.selected && <Tag color="green" icon={<CheckCircleOutlined />}>使用中</Tag>}
              style={model.selected ? { borderColor: '#52c41a' } : undefined}
            >
              <Space direction="vertical" style={{ width: '100%' }}>
                <div>
                  <Text type="secondary">大小: </Text>
                  <Text>{model.size}</Text>
                </div>
                <div>
                  <Text type="secondary">语言: </Text>
                  {model.languages.map((lang) => (
                    <Tag key={lang}>{lang}</Tag>
                  ))}
                </div>

                {downloadProgress[model.name] !== undefined && downloadProgress[model.name] < 100 && (
                  <Progress percent={Math.round(downloadProgress[model.name])} size="small" />
                )}

                <Space style={{ width: '100%', justifyContent: 'flex-end' }}>
                  {model.downloaded ? (
                    <>
                      {!model.selected && (
                        <Button type="primary" size="small" onClick={() => handleSelect(model.name)}>
                          使用此模型
                        </Button>
                      )}
                      <Button
                        danger
                        size="small"
                        icon={<DeleteOutlined />}
                        onClick={() => handleDeleteModel(model.name)}
                      >
                        删除
                      </Button>
                    </>
                  ) : (
                    <Button
                      type="primary"
                      size="small"
                      icon={<DownloadOutlined />}
                      onClick={() => handleDownload(model.name)}
                    >
                      下载
                    </Button>
                  )}
                </Space>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>

      <Divider />

      <Title level={4}>云端模型</Title>
      <Card>
        <Space direction="vertical" style={{ width: '100%' }}>
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
        </Space>
      </Card>
    </Space>
  );
}
