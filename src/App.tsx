import { useEffect, useRef, useState, useCallback, type ReactNode } from 'react';
import { Layout, Menu, Typography } from 'antd';
import {
  DashboardOutlined,
  HistoryOutlined,
  CloudDownloadOutlined,
  BookOutlined,
  ThunderboltOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import { listen } from '@tauri-apps/api/event';
import { invoke } from './lib/logger';
import Dashboard from './pages/Dashboard';
import History from './pages/History';
import Models from './pages/Models';
import Dictionary from './pages/Dictionary';
import Enhancement from './pages/Enhancement';
import Settings from './pages/Settings';

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
  const [activeKey, setActiveKey] = useState('/');
  // Track which pages have been visited — only render after first visit
  const [mounted, setMounted] = useState<Set<string>>(() => new Set(['/']));

  const handleMenuClick = useCallback(({ key }: { key: string }) => {
    setActiveKey(key);
    setMounted((prev) => (prev.has(key) ? prev : new Set(prev).add(key)));
  }, []);

  return (
    <Layout style={{ height: '100vh' }}>
      <Sider width={200} theme="light" style={{ overflow: 'auto' }}>
        <div style={{ padding: '16px', textAlign: 'center' }}>
          <Title level={4} style={{ margin: 0 }}>VoiceInk</Title>
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

export default function App() {
  const pipelineRef = useRef('idle');

  // Global hotkey listener — works on any page
  useEffect(() => {
    const unlistenToggle = listen('toggle-recording', async () => {
      const s = pipelineRef.current;
      if (s === 'recording') {
        await invoke('stop_recording').catch(() => {});
      } else if (s === 'idle') {
        await invoke('start_recording').catch(() => {});
      } else {
        // transcribing/enhancing/pasting — force cancel to recover
        await invoke('cancel_recording').catch(() => {});
      }
    });
    const unlistenState = listen<{ state: string }>('recording-state', (event) => {
      pipelineRef.current = event.payload.state;
    });
    return () => {
      unlistenToggle.then((fn) => fn());
      unlistenState.then((fn) => fn());
    };
  }, []);

  return <AppLayout />;
}
