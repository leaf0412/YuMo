import { useEffect, useRef, useMemo, useState } from 'react';
import {
  createHashRouter, RouterProvider, Outlet, useNavigate, useLocation,
} from 'react-router-dom';
import { ConfigProvider, Layout, Menu, Modal, Typography, Alert, Space } from 'antd';
import {
  DashboardOutlined,
  HistoryOutlined,
  CloudDownloadOutlined,
  BookOutlined,
  ThunderboltOutlined,
  SettingOutlined,
  SafetyCertificateOutlined,
  PictureOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import zhCN from 'antd/locale/zh_CN';
import enUS from 'antd/locale/en_US';
import i18n from './i18n';
import { listen } from './lib/events';
import yumoIcon from './assets/yumo-icon.svg';
import { invoke, logEvent } from './lib/logger';
import { broadcast } from './lib/broadcast';
import OnboardingWizard from './components/OnboardingWizard';
import useAppStore from './stores/useAppStore';

import Dashboard from './pages/Dashboard';
import History from './pages/History';
import Models from './pages/Models';
import Dictionary from './pages/Dictionary';
import Enhancement from './pages/Enhancement';
import Settings from './pages/Settings';
import Permissions from './pages/Permissions';
import Sprites from './pages/Sprites';

const { Sider, Content } = Layout;
const { Title, Text } = Typography;

/* ------------------------------------------------------------------ */
/*  PermissionBanner — renders at the top of the window               */
/* ------------------------------------------------------------------ */

function PermissionBanner() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { permissions } = useAppStore();

  const missing: { key: string; msg: string }[] = [];
  if (!permissions.microphone) missing.push({ key: 'mic', msg: t('banner.micPermission') });
  if (!permissions.accessibility) missing.push({ key: 'acc', msg: t('banner.accPermission') });

  if (missing.length === 0) return null;

  return (
    <div style={{ flexShrink: 0 }}>
      {missing.map(({ key, msg }) => (
        <Alert
          key={key}
          type="warning"
          showIcon
          banner
          message={
            <Space>
              <span>{msg}</span>
              <a onClick={() => navigate('/permissions')}>{t('banner.grant')}</a>
            </Space>
          }
        />
      ))}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  ModelStatus — sidebar bottom indicator                            */
/* ------------------------------------------------------------------ */

function ModelStatus() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { daemonStatus, models, settings } = useAppStore();

  const modelName = useMemo(() => {
    if (!daemonStatus.loaded_model) return null;
    const m = models.find((m) => m.id === daemonStatus.loaded_model);
    return m?.name || daemonStatus.loaded_model;
  }, [daemonStatus.loaded_model, models]);

  return (
    <div
      onClick={() => navigate('/models')}
      style={{ borderTop: '1px solid #f0f0f0', padding: '12px 16px', cursor: 'pointer' }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{ color: daemonStatus.running ? '#52c41a' : '#8c8c8c', fontSize: 10 }}>●</span>
        <Text strong ellipsis style={{ fontSize: 13, maxWidth: 140 }}>
          {daemonStatus.running
            ? (modelName || t('sidebar.modelRunning'))
            : t('sidebar.modelStopped')}
        </Text>
      </div>
      {daemonStatus.running && !modelName && (
        <Text type="secondary" style={{ fontSize: 11 }}>{t('sidebar.noModel')}</Text>
      )}
      {settings.hotkey ? (
        <Text type="secondary" style={{ fontSize: 11 }}>
          {String(settings.hotkey)}
        </Text>
      ) : null}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  AppLayout — grouped sidebar with 3 groups                         */
/* ------------------------------------------------------------------ */

function AppLayout() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();

  // Log page navigation (skip initial render)
  const isFirstRender = useRef(true);
  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false;
      return;
    }
    logEvent('App', 'page_navigate', { to: location.pathname });
  }, [location.pathname]);

  // Note: group items have built-in visual separation via their label.
  // If dividers cause double-separation, remove { type: 'divider' } entries.
  const menuItems = [
    {
      type: 'group' as const,
      label: t('menu.group.core'),
      children: [
        { key: '/', icon: <DashboardOutlined />, label: t('menu.dashboard') },
        { key: '/history', icon: <HistoryOutlined />, label: t('menu.history') },
      ],
    },
    { type: 'divider' as const },
    {
      type: 'group' as const,
      label: t('menu.group.configuration'),
      children: [
        { key: '/models', icon: <CloudDownloadOutlined />, label: t('menu.models') },
        { key: '/dictionary', icon: <BookOutlined />, label: t('menu.dictionary') },
        { key: '/enhancement', icon: <ThunderboltOutlined />, label: t('menu.enhancement') },
      ],
    },
    { type: 'divider' as const },
    {
      type: 'group' as const,
      label: t('menu.group.system'),
      children: [
        { key: '/permissions', icon: <SafetyCertificateOutlined />, label: t('menu.permissions') },
        { key: '/sprites', icon: <PictureOutlined />, label: t('menu.sprites') },
        { key: '/settings', icon: <SettingOutlined />, label: t('menu.settings') },
      ],
    },
  ];

  return (
    <div style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}>
      <PermissionBanner />
      <Layout style={{ flex: 1, overflow: 'hidden' }}>
        <Sider width={200} theme="light" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
          <div style={{ padding: '16px', textAlign: 'center', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
            <img src={yumoIcon} alt="语墨" style={{ width: 28, height: 28 }} />
            <Title level={4} style={{ margin: 0 }}>{t('app.name')}</Title>
          </div>
          <Menu
            mode="inline"
            selectedKeys={[location.pathname]}
            items={menuItems}
            onClick={({ key }) => navigate(key)}
            style={{ flex: 1, overflow: 'auto', borderRight: 0 }}
          />
          <ModelStatus />
        </Sider>
        <Content style={{ padding: '24px', overflow: 'auto' }}>
          <Outlet />
        </Content>
      </Layout>
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  Router                                                            */
/* ------------------------------------------------------------------ */

const router = createHashRouter([
  {
    path: '/',
    element: <AppLayout />,
    children: [
      { index: true, element: <Dashboard /> },
      { path: 'history', element: <History /> },
      { path: 'models', element: <Models /> },
      { path: 'dictionary', element: <Dictionary /> },
      { path: 'enhancement', element: <Enhancement /> },
      { path: 'settings', element: <Settings /> },
      { path: 'permissions', element: <Permissions /> },
      { path: 'sprites', element: <Sprites /> },
    ],
  },
]);

/* ------------------------------------------------------------------ */
/*  checkModelReady — uses router.navigate() (outside components)     */
/* ------------------------------------------------------------------ */

/** Check if a downloaded model is selected; if not, show warning and return false */
function checkModelReady(): boolean {
  const { settings, models } = useAppStore.getState();
  const t = i18n.t;
  const modelId = typeof settings.selected_model_id === 'string' ? settings.selected_model_id : '';
  if (!modelId) {
    Modal.warning({
      title: t('app.selectModelFirst'),
      content: t('app.selectModelFirstDesc'),
      okText: t('app.goToModels'),
      onOk: () => router.navigate('/models'),
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
      onOk: () => router.navigate('/models'),
    });
    logEvent('App', 'recording_blocked', { reason: 'model_not_downloaded', model_id: modelId });
    return false;
  }
  return true;
}

/* ------------------------------------------------------------------ */
/*  App root                                                          */
/* ------------------------------------------------------------------ */

export default function App() {
  const { i18n: i18nInstance } = useTranslation();
  const pipelineRef = useRef('idle');
  const { fetchSettings, fetchPermissions, fetchDaemonStatus, fetchModels } = useAppStore();
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [onboardingChecked, setOnboardingChecked] = useState(false);

  // Check onboarding status on mount + fetch sidebar data
  useEffect(() => {
    fetchSettings().then(() => {
      const s = useAppStore.getState().settings;
      if (s.onboarding_completed !== 'true') {
        setShowOnboarding(true);
      }
      setOnboardingChecked(true);
    });
    // Fire-and-forget: sidebar shows loading state until data arrives
    fetchPermissions();
    fetchDaemonStatus();
    fetchModels();
  }, [fetchSettings, fetchPermissions, fetchDaemonStatus, fetchModels]);

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
        broadcast('escape-hint', 'cancelled');
        lastEsc = 0;
      } else {
        lastEsc = now;
        import('antd').then(({ message }) => message.info(i18n.t('app.pressEscAgain')));
        broadcast('escape-hint', 'pressAgain');
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
          broadcast('escape-hint', 'cancelled');
          lastEsc = 0;
        } else {
          lastEsc = now;
          import('antd').then(({ message }) => message.info(i18n.t('app.pressEscAgain')));
          broadcast('escape-hint', 'pressAgain');
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
      <RouterProvider router={router} />
    </ConfigProvider>
  );
}
