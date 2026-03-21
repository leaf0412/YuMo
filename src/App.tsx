import { useEffect, useRef, useState, useCallback, type ReactNode } from 'react';
import { ConfigProvider, Layout, Menu, Modal, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import zhCN from 'antd/locale/zh_CN';
import enUS from 'antd/locale/en_US';
import i18n from './i18n';
import {
  DashboardOutlined,
  HistoryOutlined,
  CloudDownloadOutlined,
  BookOutlined,
  ThunderboltOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { listen } from './lib/events';
import yumoIcon from './assets/yumo-icon.svg';
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

const pageEntries: { key: string; render: () => ReactNode }[] = [
  { key: '/', render: () => <Dashboard /> },
  { key: '/history', render: () => <History /> },
  { key: '/models', render: () => <Models /> },
  { key: '/dictionary', render: () => <Dictionary /> },
  { key: '/enhancement', render: () => <Enhancement /> },
  { key: '/settings', render: () => <Settings /> },
];

function AppLayout() {
  const { t } = useTranslation();
  const { activeKey, setActiveKey } = useAppStore();

  const menuItems = [
    { key: '/', icon: <DashboardOutlined />, label: t('menu.dashboard') },
    { key: '/history', icon: <HistoryOutlined />, label: t('menu.history') },
    { key: '/models', icon: <CloudDownloadOutlined />, label: t('menu.models') },
    { key: '/dictionary', icon: <BookOutlined />, label: t('menu.dictionary') },
    { key: '/enhancement', icon: <ThunderboltOutlined />, label: t('menu.enhancement') },
    { key: '/settings', icon: <SettingOutlined />, label: t('menu.settings') },
  ];
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
        <div style={{ padding: '16px', textAlign: 'center', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
          <img src={yumoIcon} alt="语墨" style={{ width: 28, height: 28 }} />
          <Title level={4} style={{ margin: 0 }}>{t('app.name')}</Title>
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
  const t = i18n.t;
  const modelId = typeof settings.selected_model_id === 'string' ? settings.selected_model_id : '';
  if (!modelId) {
    Modal.warning({
      title: t('app.selectModelFirst'),
      content: t('app.selectModelFirstDesc'),
      okText: t('app.goToModels'),
      onOk: () => setActiveKey('/models'),
    });
    logEvent('App', 'recording_blocked', { reason: 'no_model_selected' });
    return false;
  }
  const model = models.find(m => m.id === modelId);
  if (model && !model.is_downloaded && ['local', 'mlxWhisper', 'mlxFunASR'].includes(model.provider)) {
    Modal.warning({
      title: t('app.modelNotDownloaded'),
      content: t('app.modelNotDownloadedDesc', { name: model.name }),
      okText: t('app.goToModels'),
      onOk: () => setActiveKey('/models'),
    });
    logEvent('App', 'recording_blocked', { reason: 'model_not_downloaded', model_id: modelId });
    return false;
  }
  return true;
}

export default function App() {
  const { i18n: i18nInstance } = useTranslation();
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

    // Global ESC double-press to cancel recording (works even without window focus)
    let lastEsc = 0;
    const unlistenEscape = listen('escape-pressed', () => {
      if (pipelineRef.current === 'idle') return;
      const now = Date.now();
      if (now - lastEsc < 500) {
        logEvent('App', 'hotkey_cancel', { current_state: pipelineRef.current });
        invoke('cancel_recording').catch(() => {});
        import('antd').then(({ message }) => message.info(i18n.t('app.recordingCancelled')));
        lastEsc = 0;
      } else {
        lastEsc = now;
        import('antd').then(({ message }) => message.info(i18n.t('app.pressEscAgain')));
      }
    });

    // Fallback: in-window ESC for when global shortcut isn't registered
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && pipelineRef.current !== 'idle') {
        const now = Date.now();
        if (now - lastEsc < 500) {
          logEvent('App', 'hotkey_cancel', { current_state: pipelineRef.current });
          invoke('cancel_recording').catch(() => {});
          import('antd').then(({ message }) => message.info(i18n.t('app.recordingCancelled')));
          lastEsc = 0;
        } else {
          lastEsc = now;
          import('antd').then(({ message }) => message.info(i18n.t('app.pressEscAgain')));
        }
      }
    };
    window.addEventListener('keydown', onKeyDown);

    return () => {
      unlistenToggle.then((fn) => fn());
      unlistenState.then((fn) => fn());
      unlistenEscape.then((fn) => fn());
      window.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  const antdLocale = i18nInstance.language === 'zh-CN' ? zhCN : enUS;

  if (!onboardingChecked) return null; // Wait for settings to load

  return (
    <ConfigProvider locale={antdLocale}>
      {showOnboarding && (
        <OnboardingWizard onComplete={() => setShowOnboarding(false)} />
      )}
      <AppLayout />
    </ConfigProvider>
  );
}
