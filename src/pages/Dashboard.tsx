import { useEffect, useState, useCallback } from 'react';
import { Card, Alert, Button, List, Space, Tag, Typography, Row, Col } from 'antd';
import {
  AudioOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  SoundOutlined,
  ApiOutlined,
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';

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
      const m = await invoke<Model[]>('list_models');
      setModels(m);
    } catch { /* ignore */ }

    try {
      const t = await invoke<Transcription[]>('get_transcriptions', { limit: 5, offset: 0 });
      setTranscriptions(t);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const selectedModel = models.find((m) => m.selected);

  const handleRecord = () => {
    setRecording((prev) => !prev);
  };

  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
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
              <Alert
                message="未授权"
                type="error"
                showIcon
                icon={<CloseCircleOutlined />}
              />
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
              <Alert
                message="未授权"
                type="error"
                showIcon
                icon={<CloseCircleOutlined />}
              />
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
              <Alert message="未选择模型" type="warning" showIcon />
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
        <List
          dataSource={transcriptions}
          locale={{ emptyText: '暂无转录记录' }}
          renderItem={(item) => (
            <List.Item>
              <List.Item.Meta
                title={
                  <Space>
                    <Text type="secondary">{item.created_at}</Text>
                    <Tag>{item.model_name}</Tag>
                  </Space>
                }
                description={item.text.length > 100 ? `${item.text.slice(0, 100)}...` : item.text}
              />
            </List.Item>
          )}
        />
      </Card>
    </Space>
  );
}
