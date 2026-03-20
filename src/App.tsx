import { useEffect, useRef, useState, useCallback, type ReactNode } from 'react';
import { Layout, Menu, Modal, Typography } from 'antd';
import {
  DashboardOutlined,
  HistoryOutlined,
  CloudDownloadOutlined,
  BookOutlined,
  ThunderboltOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { listen } from '@tauri-apps/api/event';
import { invoke, logEvent } from './lib/logger';
import { broadcast } from './lib/broadcast';
import Dashboard from './pages/Dashboard';
import History from './pages/History';
import Models from './pages/Models';
import Dictionary from './pages/Dictionary';
import Enhancement from './pages/Enhancement';
import Settings from './pages/Settings';
import OnboardingWizard from './components/OnboardingWizard';
import useAppStore from './stores/useAppStore';

const { Sider, Content } = Layout;
const { Title } = Typography;

const menuItems = [
  { key: '/', icon: <DashboardOutlined />, label: '仪表盘' },
  { key: '/history', icon: <HistoryOutlined />, label: '转录历史' },
  { key: '/models', icon: <CloudDownloadOutlined />, label: '模型管理' },
  { key: '/dictionary', icon: <BookOutlined />, label: '词典' },
  { key: '/enhancement', icon: <ThunderboltOutlined />, label: 'AI 增强' },
  { key: '/settings', icon: <SettingOutlined />, label: '设置' },
];

const pageEntries: { key: string; render: () => ReactNode }[] = [
  { key: '/', render: () => <Dashboard /> },
  { key: '/history', render: () => <History /> },
  { key: '/models', render: () => <Models /> },
  { key: '/dictionary', render: () => <Dictionary /> },
  { key: '/enhancement', render: () => <Enhancement /> },
  { key: '/settings', render: () => <Settings /> },
];

function AppLayout() {
  const { activeKey, setActiveKey } = useAppStore();
  // Track which pages have been visited — only render after first visit
  const [mounted, setMounted] = useState<Set<string>>(() => new Set(['/']));

  const handleMenuClick = useCallback(({ key }: { key: string }) => {
    logEvent('App', 'page_navigate', { to: key });
    setActiveKey(key);
    setMounted((prev) => (prev.has(key) ? prev : new Set(prev).add(key)));
  }, [setActiveKey]);

  // Sync mounted set when activeKey changes from store (e.g. from Dashboard link)
  useEffect(() => {
    setMounted((prev) => (prev.has(activeKey) ? prev : new Set(prev).add(activeKey)));
  }, [activeKey]);

  return (
    <Layout style={{ height: '100vh' }}>
      <Sider width={200} theme="light" style={{ overflow: 'auto' }}>
        <div style={{ padding: '16px', textAlign: 'center' }}>
          <Title level={4} style={{ margin: 0 }}>语墨</Title>
        </div>
        <Menu
          mode="inline"
          selectedKeys={[activeKey]}
          items={menuItems}
          onClick={handleMenuClick}
        />
      </Sider>
      <Content style={{ padding: '24px', overflow: 'auto' }}>
        {pageEntries.map(({ key, render }) =>
          mounted.has(key) ? (
            <div key={key} style={{ display: activeKey === key ? 'block' : 'none' }}>
              {render()}
            </div>
          ) : null
        )}
      </Content>
    </Layout>
  );
}

/** Check if a downloaded model is selected; if not, show warning and return false */
function checkModelReady(): boolean {
  const { settings, models, setActiveKey } = useAppStore.getState();
  const modelId = typeof settings.selected_model_id === 'string' ? settings.selected_model_id : '';
  if (!modelId) {
    Modal.warning({
      title: '请先选择模型',
      content: '录音需要一个已下载的语音识别模型。',
      okText: '前往模型页',
      onOk: () => setActiveKey('/models'),
    });
    logEvent('App', 'recording_blocked', { reason: 'no_model_selected' });
    return false;
  }
  const model = models.find(m => m.id === modelId);
  if (model && !model.is_downloaded && ['local', 'mlxWhisper', 'mlxFunASR'].includes(model.provider)) {
    Modal.warning({
      title: '模型未下载',
      content: `模型 "${model.name}" 尚未下载，请先下载。`,
      okText: '前往模型页',
      onOk: () => setActiveKey('/models'),
    });
    logEvent('App', 'recording_blocked', { reason: 'model_not_downloaded', model_id: modelId });
    return false;
  }
  return true;
}

export default function App() {
  const pipelineRef = useRef('idle');
  const { fetchSettings } = useAppStore();
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [onboardingChecked, setOnboardingChecked] = useState(false);

  // Check onboarding status on mount
  useEffect(() => {
    fetchSettings().then(() => {
      const s = useAppStore.getState().settings;
      if (s.onboarding_completed !== 'true') {
        setShowOnboarding(true);
      }
      setOnboardingChecked(true);
    });
  }, [fetchSettings]);

  // Global hotkey listener — works on any page
  useEffect(() => {
    const unlistenToggle = listen('toggle-recording', async () => {
      const s = pipelineRef.current;
      logEvent('App', 'hotkey_toggle', { current_state: s });
      if (s === 'recording') {
        await invoke('stop_recording').catch(() => {});
      } else if (s === 'idle') {
        if (!checkModelReady()) return;
        await invoke('start_recording').catch(() => {});
      } else {
        // transcribing/enhancing/pasting — force cancel to recover
        await invoke('cancel_recording').catch(() => {});
      }
    });
    const unlistenState = listen<{ state: string }>('recording-state', (event) => {
      const { state } = event.payload;
      logEvent('App', 'recording_state_changed', { state });
      pipelineRef.current = state;
      broadcast('pipeline-state', state);
    });

    // Double ESC to cancel recording
    let lastEsc = 0;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && pipelineRef.current !== 'idle') {
        const now = Date.now();
        if (now - lastEsc < 500) {
          logEvent('App', 'hotkey_cancel', { current_state: pipelineRef.current });
          invoke('cancel_recording').catch(() => {});
          lastEsc = 0;
        } else {
          lastEsc = now;
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);

    return () => {
      unlistenToggle.then((fn) => fn());
      unlistenState.then((fn) => fn());
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  if (!onboardingChecked) return null; // Wait for settings to load

  return (
    <>
      {showOnboarding && (
        <OnboardingWizard onComplete={() => setShowOnboarding(false)} />
      )}
      <AppLayout />
    </>
  );
}
