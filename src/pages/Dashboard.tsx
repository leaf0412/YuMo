import { useEffect, useState, useCallback, useRef } from 'react';
import { Card, Alert, Button, Flex, Space, Tag, Typography, Row, Col, message } from 'antd';
import {
  AudioOutlined,
  CheckCircleOutlined,
  SoundOutlined,
  SettingOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke, formatError } from '../lib/logger';
import SpriteAnimation, { type SpriteManifest } from '../components/SpriteAnimation';
import useAppStore from '../stores/useAppStore';

const { Title, Text, Paragraph } = Typography;

interface Permissions {
  microphone: boolean;
  accessibility: boolean;
}

interface Transcription {
  id: string;
  text: string;
  timestamp: string;
  model_name: string;
}

export default function Dashboard() {
  const [permissions, setPermissions] = useState<Permissions>({ microphone: false, accessibility: false });
  const [transcriptions, setTranscriptions] = useState<Transcription[]>([]);
  const [recording, setRecording] = useState(false);
  const [pipelineState, setPipelineState] = useState<string>('idle');

  // Sprite animation state
  const [spriteManifest, setSpriteManifest] = useState<SpriteManifest | null>(null);
  const [spriteImageSrc, setSpriteImageSrc] = useState<string | null>(null);

  // Permission polling state: active after user clicks "go to settings"
  const permPollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [refreshingPerm, setRefreshingPerm] = useState(false);

  // Global store
  const { models, settings, daemonStatus, fetchSettings, fetchModels, fetchDaemonStatus, setActiveKey } = useAppStore();
  const selectedModelId = typeof settings.selected_model_id === 'string' ? settings.selected_model_id : null;
  const selectedModel = selectedModelId ? models.find((m) => m.id === selectedModelId) : null;

  const loadPermissions = useCallback(async () => {
    try {
      const perms = await invoke<Permissions>('check_permissions');
      setPermissions(perms);
      return perms;
    } catch { return null; }
  }, []);

  const loadData = useCallback(async () => {
    loadPermissions();
    fetchModels();
    fetchSettings();
    fetchDaemonStatus();

    try {
      const result = await invoke<{ items: Transcription[], next_cursor: string | null }>('get_transcriptions', { limit: 5 });
      setTranscriptions(result.items || []);
    } catch { /* logged */ }
  }, [loadPermissions, fetchModels, fetchSettings, fetchDaemonStatus]);

  // Load first available sprite
  const loadSprite = useCallback(async () => {
    try {
      const sprites = await invoke<(SpriteManifest & { dirId: string })[]>('list_sprites');
      if (sprites.length === 0) return;
      const first = sprites[0];
      setSpriteManifest(first);
      // Prefer processed image, fallback to original
      const fileName = 'sprite_processed.png';
      try {
        const dataUri = await invoke<string>('get_sprite_image', { dirId: first.dirId, fileName });
        setSpriteImageSrc(dataUri);
      } catch {
        const dataUri = await invoke<string>('get_sprite_image', { dirId: first.dirId, fileName: first.spriteFile });
        setSpriteImageSrc(dataUri);
      }
    } catch { /* no sprites available */ }
  }, []);

  useEffect(() => { loadData(); loadSprite(); }, [loadData, loadSprite]);

  // 1. Window focus: refresh permissions when app regains focus
  useEffect(() => {
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) loadPermissions();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [loadPermissions]);

  // 2. Short-term polling: cleanup on unmount or when all permissions granted
  useEffect(() => {
    if (permissions.microphone && permissions.accessibility && permPollRef.current) {
      clearInterval(permPollRef.current);
      permPollRef.current = null;
    }
    return () => {
      if (permPollRef.current) {
        clearInterval(permPollRef.current);
        permPollRef.current = null;
      }
    };
  }, [permissions.microphone, permissions.accessibility]);

  useEffect(() => {
    const unlisten = listen<{ state: string }>('recording-state', (event) => {
      const s = event.payload.state;
      setPipelineState(s);
      setRecording(s === 'recording');
      if (s === 'idle') loadData();
    });
    return () => { unlisten.then((fn) => fn()); };
  }, [loadData]);

  const handleRecord = async () => {
    if (recording) {
      try {
        await invoke('stop_recording');
        message.success('转录完成');
      } catch (e: unknown) {
        message.error(formatError(e, '停止录音失败'));
      } finally {
        setRecording(false);
        loadData();
      }
    } else {
      try {
        await invoke('start_recording');
        setRecording(true);
      } catch (e: unknown) {
        message.error(formatError(e, '开始录音失败'));
      }
    }
  };

  // 2. Start short-term polling after opening system settings
  const openSettings = async (permissionType: string) => {
    try {
      await invoke('request_permission', { permissionType });
    } catch {
      message.error('无法打开系统设置');
      return;
    }
    // Start 1s polling until permission granted (auto-stops via effect above)
    if (!permPollRef.current) {
      permPollRef.current = setInterval(() => loadPermissions(), 1000);
    }
  };

  // 3. Manual refresh button handler
  const handleRefreshPermissions = async () => {
    setRefreshingPerm(true);
    await loadPermissions();
    setTimeout(() => setRefreshingPerm(false), 500);
  };

  const hasSprite = spriteManifest && spriteImageSrc;

  const statusText = () => {
    switch (pipelineState) {
      case 'recording': return '录音中...';
      case 'transcribing': return '转录中...';
      case 'enhancing': return 'AI 增强中...';
      case 'pasting': return '粘贴中...';
      default: return '点击开始录音';
    }
  };

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Title level={3}>仪表盘</Title>

      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12} md={8}>
          <Card title="麦克风权限" size="small" extra={
            <ReloadOutlined spin={refreshingPerm} onClick={handleRefreshPermissions} style={{ cursor: 'pointer' }} />
          }>
            {permissions.microphone ? (
              <Alert message="已授权" type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={8}>
                <Alert message="待授权" description="首次录音时系统会弹出授权弹窗，或前往设置手动开启" type="warning" showIcon />
                <Button size="small" icon={<SettingOutlined />} onClick={() => openSettings('microphone')}>前往系统设置</Button>
              </Flex>
            )}
          </Card>
        </Col>
        <Col xs={24} sm={12} md={8}>
          <Card title="辅助功能权限" size="small" extra={
            <ReloadOutlined spin={refreshingPerm} onClick={handleRefreshPermissions} style={{ cursor: 'pointer' }} />
          }>
            {permissions.accessibility ? (
              <Alert message="已授权" type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={8}>
                <Alert message="待授权" description="需要辅助功能权限才能自动粘贴，前往设置添加本应用" type="warning" showIcon />
                <Button size="small" icon={<SettingOutlined />} onClick={() => openSettings('accessibility')}>前往系统设置</Button>
              </Flex>
            )}
          </Card>
        </Col>
        <Col xs={24} sm={12} md={8}>
          <Card title="当前模型" size="small" extra={
            <Button type="link" size="small" onClick={() => setActiveKey('/models')}>模型管理</Button>
          }>
            {selectedModel ? (
              <Flex vertical gap={4}>
                <Text strong>{selectedModel.name}</Text>
                {!selectedModel.is_downloaded && ['mlxWhisper', 'mlxFunASR', 'local'].includes(selectedModel.provider) && (
                  <Tag color="orange">模型未下载</Tag>
                )}
                {['mlxWhisper', 'mlxFunASR'].includes(selectedModel.provider) && (
                  <Tag color={daemonStatus.running ? 'green' : 'default'}>
                    {daemonStatus.running
                      ? daemonStatus.loaded_model ? `已加载: ${daemonStatus.loaded_model.split('/').pop()}` : 'Daemon 运行中'
                      : 'Daemon 未启动'}
                  </Tag>
                )}
              </Flex>
            ) : (
              <Alert
                message="未选择模型"
                description={<>请先前往<Button type="link" size="small" style={{ padding: 0 }} onClick={() => setActiveKey('/models')}>模型管理</Button>下载并选择一个转录模型</>}
                type="warning"
                showIcon
              />
            )}
          </Card>
        </Col>
      </Row>

      {/* Recording area with sprite animation */}
      <div style={{ textAlign: 'center', padding: '24px 0', position: 'relative' }}>
        {hasSprite ? (
          <div style={{ cursor: 'pointer', display: 'inline-block' }} onClick={handleRecord}>
            <SpriteAnimation
              manifest={spriteManifest}
              imageSrc={spriteImageSrc}
              isPlaying={recording}
              width={160}
              height={160}
            />
          </div>
        ) : (
          <Button type="primary" shape="circle" size="large" danger={recording}
            icon={recording ? <SoundOutlined /> : <AudioOutlined />}
            onClick={handleRecord} style={{ width: 80, height: 80, fontSize: 32 }}
            aria-label={recording ? '停止录音' : '开始录音'}
          />
        )}
        <Paragraph style={{ marginTop: 8 }}>
          {statusText()}
          {pipelineState === 'transcribing' && <span className="loading-dots"> ...</span>}
        </Paragraph>
      </div>

      <Card title="最近转录">
        {transcriptions.length === 0 ? (
          <Text type="secondary">暂无转录记录</Text>
        ) : (
          transcriptions.map((item) => (
            <div key={item.id} style={{ padding: '12px 0', borderBottom: '1px solid #f0f0f0' }}>
              <Space>
                <Text type="secondary">{item.timestamp}</Text>
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
