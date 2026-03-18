import { BrowserRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, Typography } from 'antd';
import {
  DashboardOutlined,
  HistoryOutlined,
  CloudDownloadOutlined,
  BookOutlined,
  ThunderboltOutlined,
  SettingOutlined,
} from '@ant-design/icons';
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

function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider width={200} theme="light">
        <div style={{ padding: '16px', textAlign: 'center' }}>
          <Title level={4} style={{ margin: 0 }}>VoiceInk</Title>
        </div>
        <Menu
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Content style={{ padding: '24px' }}>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/history" element={<History />} />
          <Route path="/models" element={<Models />} />
          <Route path="/dictionary" element={<Dictionary />} />
          <Route path="/enhancement" element={<Enhancement />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </Content>
    </Layout>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppLayout />
    </BrowserRouter>
  );
}
