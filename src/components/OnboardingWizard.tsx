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
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation();
  const [step, setStep] = useState(0);
  const [ready, setReady] = useState(false);

  // Permissions
  const [micOk, setMicOk] = useState(false);
  const [accOk, setAccOk] = useState(false);
  const [checkingPerm, setCheckingPerm] = useState(false);
  const permPollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Model
  const { models, fetchModels, updateSetting } = useAppStore();
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
  // Settings already loaded by App.tsx; only fetch models here.
  useEffect(() => {
    (async () => {
      const perms = await invoke<{ microphone: boolean; accessibility: boolean }>('check_permissions').catch(() => null);
      if (perms) {
        setMicOk(perms.microphone);
        setAccOk(perms.accessibility);
      }
      await fetchModels();
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
  }, [fetchModels]);

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
      message.error(formatError(e, t('onboarding.modelDownloadFailed')));
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
          message.success(t('onboarding.hotkeySet', { shortcut }));
        } catch (e) {
          message.error(formatError(e, t('onboarding.hotkeyRegisterFailed')));
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
            <Title level={2} style={{ margin: 0 }}>{t('onboarding.welcomeTitle')}</Title>
            <Paragraph style={{ fontSize: 16, textAlign: 'center', maxWidth: 400 }}>
              {t('onboarding.welcomeDesc')}
            </Paragraph>
            <Paragraph type="secondary">{t('onboarding.welcomeHint')}</Paragraph>
          </Flex>
        );

      case 1: // Microphone
        return (
          <Flex vertical gap={16} style={{ padding: '24px 0' }}>
            <Title level={4}>{t('onboarding.micTitle')}</Title>
            <Paragraph>{t('onboarding.micDesc')}</Paragraph>
            {micOk ? (
              <Alert message={t('onboarding.micGranted')} type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={12}>
                <Alert message={t('onboarding.micNeeded')} type="warning" showIcon />
                <Button type="primary" onClick={() => requestPermission('microphone')} loading={checkingPerm}>
                  {t('onboarding.micGrant')}
                </Button>
              </Flex>
            )}
          </Flex>
        );

      case 2: // Accessibility
        return (
          <Flex vertical gap={16} style={{ padding: '24px 0' }}>
            <Title level={4}>{t('onboarding.accTitle')}</Title>
            <Paragraph>{t('onboarding.accDesc')}</Paragraph>
            {accOk ? (
              <Alert message={t('onboarding.accGranted')} type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={12}>
                <Alert message={t('onboarding.accNeeded')} description={t('onboarding.accNeededDesc')} type="warning" showIcon />
                <Button type="primary" onClick={() => requestPermission('accessibility')} loading={checkingPerm}>
                  {t('onboarding.accGrant')}
                </Button>
              </Flex>
            )}
          </Flex>
        );

      case 3: // Model
        return (
          <Flex vertical gap={16} style={{ padding: '24px 0' }}>
            <Title level={4}>{t('onboarding.modelTitle')}</Title>
            <Paragraph>{t('onboarding.modelDesc')}</Paragraph>
            {modelReady ? (
              <Alert
                message={t('onboarding.modelReady', { name: selectedModel?.name })}
                type="success"
                showIcon
                icon={<CheckCircleOutlined />}
              />
            ) : downloading ? (
              <Card>
                <Flex vertical gap={8}>
                  <Text>{t('onboarding.modelDownloading', { name: selectedModel?.name })}</Text>
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
                    data-testid={`model-${m.id}`}
                  >
                    <Flex justify="space-between" align="center">
                      <Flex vertical gap={2}>
                        <Space>
                          <Text strong>{m.name}</Text>
                          {m.is_recommended && <Tag color="blue">{t('onboarding.modelRecommended')}</Tag>}
                        </Space>
                        <Text type="secondary" style={{ fontSize: 12 }}>{m.description} · {m.size_mb >= 1000 ? `${(m.size_mb / 1024).toFixed(1)}GB` : `${m.size_mb}MB`}</Text>
                      </Flex>
                      <Button type="primary" size="small">{t('onboarding.modelDownload')}</Button>
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
            <Title level={4}>{t('onboarding.hotkeyTitle')}</Title>
            <Paragraph>{t('onboarding.hotkeyDesc')}</Paragraph>
            {hotkeySet ? (
              <Alert message={t('onboarding.hotkeySet', { shortcut: hotkeyValue })} type="success" showIcon icon={<CheckCircleOutlined />} />
            ) : (
              <Flex vertical gap={12}>
                <Input
                  readOnly
                  value={recordingHotkey ? t('onboarding.hotkeyRecording') : hotkeyValue}
                  placeholder={t('onboarding.hotkeyPlaceholder')}
                  onFocus={() => setRecordingHotkey(true)}
                  onBlur={() => setRecordingHotkey(false)}
                  onKeyDown={recordingHotkey ? handleHotkeyKeyDown : undefined}
                  prefix={recordingHotkey ? <LoadingOutlined /> : <ThunderboltOutlined />}
                  style={{ maxWidth: 300 }}
                  data-testid="hotkey-record-btn"
                />
              </Flex>
            )}
          </Flex>
        );

      case 5: // Done
        return (
          <Flex vertical align="center" gap={24} style={{ padding: '40px 0' }}>
            <CheckCircleOutlined style={{ fontSize: 64, color: '#52c41a' }} />
            <Title level={2} style={{ margin: 0 }}>{t('onboarding.doneTitle')}</Title>
            <Paragraph style={{ fontSize: 16, textAlign: 'center', maxWidth: 400 }}>
              {hotkeyValue
                ? <>{t('onboarding.doneHotkeyPrefix')} <Tag color="blue">{hotkeyValue}</Tag> {t('onboarding.doneHotkeySuffix')}</>
                : t('onboarding.doneNoHotkey')
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
    if (step === 0) return t('onboarding.startSetup');
    if (step === 4 && !hotkeySet) return t('onboarding.skip');
    if (step === 5) return t('onboarding.startUsing');
    return t('onboarding.next');
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
    { title: t('onboarding.stepWelcome'), icon: <SmileOutlined /> },
    { title: t('onboarding.stepMic'), icon: <AudioOutlined /> },
    { title: t('onboarding.stepAcc'), icon: <DesktopOutlined /> },
    { title: t('onboarding.stepModel'), icon: <CloudDownloadOutlined /> },
    { title: t('onboarding.stepHotkey'), icon: <ThunderboltOutlined /> },
    { title: t('onboarding.stepDone'), icon: <CheckCircleOutlined /> },
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
              data-testid={step === 0 ? 'onboarding-start' : step === 5 ? 'onboarding-done' : undefined}
            >
              {nextLabel()}
            </Button>
          </Flex>
        </div>
      </Flex>
    </Modal>
  );
}
