import { useEffect, useState, useCallback } from 'react';
import { Card, Alert, Button, Flex, Space, Tag, Typography, Row, Col, message } from 'antd';
import {
  AudioOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  SoundOutlined,
  ApiOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const { Title, Text, Paragraph } = Typography;

interface Permissions {
  microphone: boolean;
  accessibility: boolean;
}

interface Model {
  name: string;
  size: string;
  selected: boolean;
}

interface Transcription {
  id: number;
  text: string;
  created_at: string;
  model_name: string;
}

export default function Dashboard() {
  const [permissions, setPermissions] = useState<Permissions>({ microphone: false, accessibility: false });
  const [models, setModels] = useState<Model[]>([]);
  const [transcriptions, setTranscriptions] = useState<Transcription[]>([]);
  const [recording, setRecording] = useState(false);

  const loadData = useCallback(async () => {
    try {
      const perms = await invoke<Permissions>('check_permissions');
      setPermissions(perms);
    } catch { /* ignore */ }

    try {
      const m = await invoke<Model[]>('list_available_models');
      setModels(m);
    } catch { /* ignore */ }

    try {
      const result = await invoke<{ items: Transcription[], next_cursor: string | null }>('get_transcriptions', { limit: 5 });
      setTranscriptions(result.items || []);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  // Listen to recording state changes from backend
  useEffect(() => {
    const unlisten = listen<{ state: string }>('recording-state', (event) => {
      const s = event.payload.state;
      setRecording(s === 'recording');
      if (s === 'idle') {
        loadData(); // refresh permissions & transcriptions
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [loadData]);

  const selectedModel = models.find((m) => m.selected);

  const handleRecord = async () => {
    if (recording) {
      try {
        await invoke('stop_recording');
        message.success('转录完成');
      } catch (e: unknown) {
        message.error(typeof e === 'string' ? e : '停止录音失败');
      } finally {
        setRecording(false);
        loadData();
      }
    } else {
      try {
        await invoke('start_recording', {});
        setRecording(true);
      } catch (e: unknown) {
        message.error(typeof e === 'string' ? e : '开始录音失败');
      }
    }
  };

  const openSettings = async (permissionType: string) => {
    try {
      await invoke('request_permission', { permissionType });
    } catch {
      message.error('无法打开系统设置');
    }
  };

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Title level={3}>仪表盘</Title>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} md={8}>
          <Card title="麦克风权限" size="small">
            {permissions.microphone ? (
              <Alert
                message="已授权"
                type="success"
                showIcon
                icon={<CheckCircleOutlined />}
              />
            ) : (
              <Flex vertical gap={8}>
                <Alert
                  message="待授权"
                  description="首次录音时系统会弹出授权弹窗，或前往设置手动开启"
                  type="warning"
                  showIcon
                />
                <Button
                  size="small"
                  icon={<SettingOutlined />}
                  onClick={() => openSettings('microphone')}
                >
                  前往系统设置
                </Button>
              </Flex>
            )}
          </Card>
        </Col>

        <Col xs={24} sm={12} md={8}>
          <Card title="辅助功能权限" size="small">
            {permissions.accessibility ? (
              <Alert
                message="已授权"
                type="success"
                showIcon
                icon={<CheckCircleOutlined />}
              />
            ) : (
              <Flex vertical gap={8}>
                <Alert
                  message="待授权"
                  description="需要辅助功能权限才能自动粘贴，前往设置添加本应用"
                  type="warning"
                  showIcon
                />
                <Button
                  size="small"
                  icon={<SettingOutlined />}
                  onClick={() => openSettings('accessibility')}
                >
                  前往系统设置
                </Button>
              </Flex>
            )}
          </Card>
        </Col>

        <Col xs={24} sm={12} md={8}>
          <Card title="当前模型" size="small">
            {selectedModel ? (
              <Space>
                <ApiOutlined />
                <Text strong>{selectedModel.name}</Text>
                <Tag color="blue">{selectedModel.size}</Tag>
              </Space>
            ) : (
              <Alert title="未选择模型" type="warning" showIcon />
            )}
          </Card>
        </Col>
      </Row>

      <div style={{ textAlign: 'center', padding: '24px 0' }}>
        <Button
          type="primary"
          shape="circle"
          size="large"
          danger={recording}
          icon={recording ? <SoundOutlined /> : <AudioOutlined />}
          onClick={handleRecord}
          style={{ width: 80, height: 80, fontSize: 32 }}
          aria-label={recording ? '停止录音' : '开始录音'}
        />
        <Paragraph style={{ marginTop: 8 }}>
          {recording ? '录音中...' : '点击开始录音'}
        </Paragraph>
      </div>

      <Card title="最近转录">
        {transcriptions.length === 0 ? (
          <Text type="secondary">暂无转录记录</Text>
        ) : (
          transcriptions.map((item) => (
            <div key={item.id} style={{ padding: '12px 0', borderBottom: '1px solid #f0f0f0' }}>
              <Space>
                <Text type="secondary">{item.created_at}</Text>
                <Tag>{item.model_name}</Tag>
              </Space>
              <Paragraph type="secondary" style={{ marginBottom: 0, marginTop: 4 }}>
                {item.text.length > 100 ? `${item.text.slice(0, 100)}...` : item.text}
              </Paragraph>
            </div>
          ))
        )}
      </Card>
    </Flex>
  );
}
