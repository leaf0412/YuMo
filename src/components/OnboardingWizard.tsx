import { useState, useCallback, useRef, useEffect } from 'react';
import { Modal, Steps, Button, Typography, Alert, Flex, Card, Progress, Input, Tag, message, Space } from 'antd';
import {
  SmileOutlined,
  AudioOutlined,
  DesktopOutlined,
  CloudDownloadOutlined,
  ThunderboltOutlined,
  CheckCircleOutlined,
  LoadingOutlined,
} from '@ant-design/icons';
import { listen } from '@tauri-apps/api/event';
import { invoke, formatError, logEvent } from '../lib/logger';
import useAppStore, { type ModelInfo } from '../stores/useAppStore';

const { Title, Text, Paragraph } = Typography;

/** Recommended models shown in the wizard (order matters) */
const RECOMMENDED_IDS = [
  'mlx-funasr-nano-8bit',   // Best for Apple Silicon
  'ggml-base',              // Good balance, multilingual
  'ggml-base.en',           // Good balance, English
];

interface Props {
  onComplete: () => void;
}

export default function OnboardingWizard({ onComplete }: Props) {
  const [step, setStep] = useState(0);
  const [ready, setReady] = useState(false);

  // Permissions
  const [micOk, setMicOk] = useState(false);
  const [accOk, setAccOk] = useState(false);
  const [checkingPerm, setCheckingPerm] = useState(false);
  const permPollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Model
  const { models, fetchModels, fetchSettings, updateSetting } = useAppStore();
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [selectedModel, setSelectedModel] = useState<ModelInfo | null>(null);
  const [modelReady, setModelReady] = useState(false);

  // Hotkey
  const [recordingHotkey, setRecordingHotkey] = useState(false);
  const [hotkeyValue, setHotkeyValue] = useState('');
  const [hotkeySet, setHotkeySet] = useState(false);

  const recommendedModels = models.filter(m => RECOMMENDED_IDS.includes(m.id));

  // --- Auto-detect already completed steps on mount ---
  useEffect(() => {
    (async () => {
      const perms = await invoke<{ microphone: boolean; accessibility: boolean }>('check_permissions').catch(() => null);
      if (perms) {
        setMicOk(perms.microphone);
        setAccOk(perms.accessibility);
      }
      await fetchModels();
      await fetchSettings();
      const s = useAppStore.getState();
      const modelId = typeof s.settings.selected_model_id === 'string' ? s.settings.selected_model_id : '';
      const model = modelId ? s.models.find(m => m.id === modelId) : null;
      if (model && model.is_downloaded) {
        setModelReady(true);
        setSelectedModel(model);
      }
      const hotkey = typeof s.settings.hotkey === 'string' ? s.settings.hotkey : '';
      if (hotkey) {
        setHotkeyValue(hotkey);
        setHotkeySet(true);
      }
      setReady(true);
    })();
  }, [fetchModels, fetchSettings]);

  // --- Permission helpers ---
  const checkPermissions = useCallback(async () => {
    setCheckingPerm(true);
    try {
      const perms = await invoke<{ microphone: boolean; accessibility: boolean }>('check_permissions');
      setMicOk(perms.microphone);
      setAccOk(perms.accessibility);
      return perms;
    } catch { return null; }
    finally { setCheckingPerm(false); }
  }, []);

  const requestPermission = async (type: string) => {
    await invoke('request_permission', { permissionType: type }).catch(() => {});
    logEvent('Onboarding', 'request_permission', { type });
    // Poll until granted
    if (permPollRef.current) clearInterval(permPollRef.current);
    permPollRef.current = setInterval(async () => {
      const perms = await checkPermissions();
      if (!perms) return;
      if (type === 'microphone' && perms.microphone) {
        clearInterval(permPollRef.current!);
        permPollRef.current = null;
      }
      if (type === 'accessibility' && perms.accessibility) {
        clearInterval(permPollRef.current!);
        permPollRef.current = null;
      }
    }, 1000);
  };

  // --- Model download ---
  const handleDownloadModel = async (model: ModelInfo) => {
    setSelectedModel(model);
    setDownloading(true);
    setDownloadProgress(0);
    logEvent('Onboarding', 'download_model', { model_id: model.id });

    const isMLX = ['mlxWhisper', 'mlxFunASR'].includes(model.provider);

    try {
      if (isMLX) {
        // MLX models: use daemon_load_model which handles download + load
        const unlistenSetup = await listen<{ stage: string }>('daemon-setup-status', (e) => {
          const stage = e.payload.stage;
          if (stage === 'installing_deps') setDownloadProgress(10);
          if (stage === 'starting_daemon') setDownloadProgress(20);
          if (stage === 'ready') setDownloadProgress(30);
        });
        const unlistenProgress = await listen<{ progress: number }>('model-download-progress', (e) => {
          setDownloadProgress(30 + Math.min(e.payload.progress * 70, 70));
        });

        await invoke('daemon_load_model', { modelRepo: model.model_repo });
        unlistenSetup();
        unlistenProgress();
      } else {
        // Local whisper models: use download_model
        const unlistenProgress = await listen<{ progress: number }>('model-download-progress', (e) => {
          setDownloadProgress(e.payload.progress * 100);
        });
        await invoke('download_model', { modelId: model.id });
        unlistenProgress();
      }

      setDownloadProgress(100);
      await updateSetting('selected_model_id', model.id);
      await fetchModels();
      setModelReady(true);
      setDownloading(false);
      logEvent('Onboarding', 'download_complete', { model_id: model.id });
    } catch (e) {
      setDownloading(false);
      message.error(formatError(e, '下载失败'));
      logEvent('Onboarding', 'download_error', { model_id: model.id, error: formatError(e, 'unknown') });
    }
  };

  // --- Hotkey ---
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
      setHotkeyValue(shortcut);
      setRecordingHotkey(false);
      (async () => {
        try {
          await invoke('register_hotkey', { shortcut });
          await updateSetting('hotkey', shortcut);
          setHotkeySet(true);
          logEvent('Onboarding', 'hotkey_set', { shortcut });
          message.success(`快捷键已设置: ${shortcut}`);
        } catch (e) {
          message.error(formatError(e, '注册失败'));
        }
      })();
    }
  };

  // --- Complete ---
  const handleComplete = async () => {
    await updateSetting('onboarding_completed', 'true');
    logEvent('Onboarding', 'completed');
    if (permPollRef.current) clearInterval(permPollRef.current);
    onComplete();
  };

  // --- Step content ---
  const stepContent = () => {
    switch (step) {
      case 0: // Welcome
        return (
          <Flex vertical align="center" gap={24} style={{ padding: '40px 0' }}>
            <Title level={2} style={{ margin: 0 }}>欢迎使用语墨</Title>
            <Paragraph style={{ fontSize: 16, textAlign: 'center', maxWidth: 400 }}>
              语音转文字工具，按下快捷键说话，松开后文字自动粘贴到光标位置。
            </Paragraph>
            <Paragraph type="secondary">接下来几步帮你完成初始设置。</Paragraph>
          </Flex>
        );

      case 1: // Microphone
        return (
          <Flex vertical gap={16} style={{ padding: '24px 0' }}>
            <Title level={4}>麦克风权限</Title>
            <Paragraph>语墨需要麦克风权限来录制你的语音。</Paragraph>
            {micOk ? (
              <Alert message="麦克风权限已授权" type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={12}>
                <Alert message="需要麦克风权限" type="warning" showIcon />
                <Button type="primary" onClick={() => requestPermission('microphone')} loading={checkingPerm}>
                  授予麦克风权限
                </Button>
              </Flex>
            )}
          </Flex>
        );

      case 2: // Accessibility
        return (
          <Flex vertical gap={16} style={{ padding: '24px 0' }}>
            <Title level={4}>辅助功能权限</Title>
            <Paragraph>语墨需要辅助功能权限来将转写文字自动粘贴到光标位置。</Paragraph>
            {accOk ? (
              <Alert message="辅助功能权限已授权" type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={12}>
                <Alert message="需要辅助功能权限" description="点击后会打开系统设置，在列表中找到语墨并勾选" type="warning" showIcon />
                <Button type="primary" onClick={() => requestPermission('accessibility')} loading={checkingPerm}>
                  前往系统设置
                </Button>
              </Flex>
            )}
          </Flex>
        );

      case 3: // Model
        return (
          <Flex vertical gap={16} style={{ padding: '24px 0' }}>
            <Title level={4}>选择语音模型</Title>
            <Paragraph>选择一个模型下载，下载完成后即可开始转写。</Paragraph>
            {modelReady ? (
              <Alert
                message={`${selectedModel?.name} 已就绪`}
                type="success"
                showIcon
                icon={<CheckCircleOutlined />}
              />
            ) : downloading ? (
              <Card>
                <Flex vertical gap={8}>
                  <Text>{selectedModel?.name} 下载中...</Text>
                  <Progress percent={Math.round(downloadProgress)} status="active" />
                </Flex>
              </Card>
            ) : (
              <Flex vertical gap={8}>
                {recommendedModels.map(m => (
                  <Card
                    key={m.id}
                    hoverable
                    size="small"
                    onClick={() => handleDownloadModel(m)}
                    style={{ cursor: 'pointer' }}
                  >
                    <Flex justify="space-between" align="center">
                      <Flex vertical gap={2}>
                        <Space>
                          <Text strong>{m.name}</Text>
                          {m.is_recommended && <Tag color="blue">推荐</Tag>}
                        </Space>
                        <Text type="secondary" style={{ fontSize: 12 }}>{m.description} · {m.size_mb >= 1000 ? `${(m.size_mb / 1024).toFixed(1)}GB` : `${m.size_mb}MB`}</Text>
                      </Flex>
                      <Button type="primary" size="small">下载</Button>
                    </Flex>
                  </Card>
                ))}
              </Flex>
            )}
          </Flex>
        );

      case 4: // Hotkey
        return (
          <Flex vertical gap={16} style={{ padding: '24px 0' }}>
            <Title level={4}>设置全局快捷键</Title>
            <Paragraph>设置一个快捷键，在任何应用中按下即可开始录音。</Paragraph>
            {hotkeySet ? (
              <Alert message={`快捷键已设置: ${hotkeyValue}`} type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={12}>
                <Input
                  readOnly
                  value={recordingHotkey ? '请按下快捷键组合...' : hotkeyValue}
                  placeholder="点击此处开始录制快捷键"
                  onFocus={() => setRecordingHotkey(true)}
                  onBlur={() => setRecordingHotkey(false)}
                  onKeyDown={recordingHotkey ? handleHotkeyKeyDown : undefined}
                  prefix={recordingHotkey ? <LoadingOutlined /> : <ThunderboltOutlined />}
                  style={{ maxWidth: 300 }}
                />
              </Flex>
            )}
          </Flex>
        );

      case 5: // Done
        return (
          <Flex vertical align="center" gap={24} style={{ padding: '40px 0' }}>
            <CheckCircleOutlined style={{ fontSize: 64, color: '#52c41a' }} />
            <Title level={2} style={{ margin: 0 }}>设置完成！</Title>
            <Paragraph style={{ fontSize: 16, textAlign: 'center', maxWidth: 400 }}>
              {hotkeyValue
                ? <>按下 <Tag color="blue">{hotkeyValue}</Tag> 开始录音，松开后自动转写并粘贴到光标位置。</>
                : '在首页点击录音按钮，或前往设置页配置快捷键。'
              }
            </Paragraph>
          </Flex>
        );
    }
  };

  const canProceed = () => {
    switch (step) {
      case 0: return true;
      case 1: return micOk;
      case 2: return accOk;
      case 3: return modelReady;
      case 4: return true; // skippable
      case 5: return true;
      default: return false;
    }
  };

  const nextLabel = () => {
    if (step === 0) return '开始设置';
    if (step === 4 && !hotkeySet) return '跳过';
    if (step === 5) return '开始使用';
    return '下一步';
  };

  const handleNext = () => {
    if (step === 5) {
      handleComplete();
    } else {
      const next = step + 1;
      setStep(next);
      // Auto-check permissions when entering permission steps
      if (next === 1 || next === 2) checkPermissions();
      // Fetch models when entering model step
      if (next === 3) fetchModels();
    }
  };

  const stepItems = [
    { title: '欢迎', icon: <SmileOutlined /> },
    { title: '麦克风', icon: <AudioOutlined /> },
    { title: '辅助功能', icon: <DesktopOutlined /> },
    { title: '模型', icon: <CloudDownloadOutlined /> },
    { title: '快捷键', icon: <ThunderboltOutlined /> },
    { title: '完成', icon: <CheckCircleOutlined /> },
  ];

  if (!ready) return null;

  return (
    <Modal
      open
      closable={false}
      footer={null}
      width="90vw"
      style={{ top: '5vh', maxWidth: 800 }}
      styles={{ body: { minHeight: '70vh', display: 'flex', flexDirection: 'column' } }}
    >
      <Flex style={{ flex: 1, minHeight: 0 }}>
        <div style={{ width: 180, borderRight: '1px solid #f0f0f0', paddingRight: 24, paddingTop: 16 }}>
          <Steps
            direction="vertical"
            current={step}
            items={stepItems}
            size="small"
          />
        </div>
        <div style={{ flex: 1, paddingLeft: 32, display: 'flex', flexDirection: 'column' }}>
          <div style={{ flex: 1 }}>
            {stepContent()}
          </div>
          <Flex justify="flex-end" style={{ paddingTop: 16, borderTop: '1px solid #f0f0f0' }}>
            <Button
              type={step === 5 ? 'primary' : canProceed() ? 'primary' : 'default'}
              size="large"
              onClick={handleNext}
              disabled={!canProceed() && step !== 4}
            >
              {nextLabel()}
            </Button>
          </Flex>
        </div>
      </Flex>
    </Modal>
  );
}
