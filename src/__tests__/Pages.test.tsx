import { render, screen } from '@testing-library/react';
import { describe, test, expect } from 'vitest';

// Import test utils for mocking
import './test-utils';

describe('Dashboard', () => {
  test('renders record button and page title', async () => {
    const Dashboard = (await import('../pages/Dashboard')).default;
    render(<Dashboard />);
    expect(screen.getByText('仪表盘')).toBeInTheDocument();
    expect(screen.getByLabelText('开始录音')).toBeInTheDocument();
  });

  test('renders permission cards', async () => {
    const Dashboard = (await import('../pages/Dashboard')).default;
    render(<Dashboard />);
    expect(screen.getByText('麦克风权限')).toBeInTheDocument();
    expect(screen.getByText('辅助功能权限')).toBeInTheDocument();
  });

  test('renders recent transcriptions section', async () => {
    const Dashboard = (await import('../pages/Dashboard')).default;
    render(<Dashboard />);
    expect(screen.getByText('最近转录')).toBeInTheDocument();
  });
});

describe('History', () => {
  test('renders search input and clear button', async () => {
    const History = (await import('../pages/History')).default;
    render(<History />);
    expect(screen.getByPlaceholderText('搜索转录内容...')).toBeInTheDocument();
    expect(screen.getByText('清空全部')).toBeInTheDocument();
  });
});

describe('Models', () => {
  test('renders page title and import button', async () => {
    const Models = (await import('../pages/Models')).default;
    render(<Models />);
    expect(screen.getByText('模型管理')).toBeInTheDocument();
    expect(screen.getByText('导入模型')).toBeInTheDocument();
  });

  test('renders local and cloud model sections', async () => {
    const Models = (await import('../pages/Models')).default;
    render(<Models />);
    expect(screen.getByText('本地模型')).toBeInTheDocument();
    expect(screen.getByText('云端模型')).toBeInTheDocument();
  });

  test('renders API key input', async () => {
    const Models = (await import('../pages/Models')).default;
    render(<Models />);
    expect(screen.getByPlaceholderText('输入 API Key')).toBeInTheDocument();
  });
});

describe('Dictionary', () => {
  test('renders tabs for vocabulary and replacements', async () => {
    const Dictionary = (await import('../pages/Dictionary')).default;
    render(<Dictionary />);
    expect(screen.getByText('词典')).toBeInTheDocument();
    expect(screen.getByText('词汇表')).toBeInTheDocument();
    expect(screen.getByText('替换规则')).toBeInTheDocument();
  });

  test('renders add vocabulary input', async () => {
    const Dictionary = (await import('../pages/Dictionary')).default;
    render(<Dictionary />);
    expect(screen.getByPlaceholderText('添加新词汇...')).toBeInTheDocument();
  });
});

describe('Enhancement', () => {
  test('renders AI enhancement toggle and title', async () => {
    const Enhancement = (await import('../pages/Enhancement')).default;
    render(<Enhancement />);
    expect(screen.getByText('AI 增强')).toBeInTheDocument();
    expect(screen.getByText('启用 AI 增强')).toBeInTheDocument();
  });

  test('renders prompt management section', async () => {
    const Enhancement = (await import('../pages/Enhancement')).default;
    render(<Enhancement />);
    expect(screen.getByText('Prompt 管理')).toBeInTheDocument();
    expect(screen.getByText('新建 Prompt')).toBeInTheDocument();
  });

  test('renders provider and model selects', async () => {
    const Enhancement = (await import('../pages/Enhancement')).default;
    render(<Enhancement />);
    expect(screen.getByText('LLM 服务商')).toBeInTheDocument();
    expect(screen.getByText('模型')).toBeInTheDocument();
  });
});

describe('Settings', () => {
  test('renders settings title and collapse panels', async () => {
    const Settings = (await import('../pages/Settings')).default;
    render(<Settings />);
    expect(screen.getByText('设置')).toBeInTheDocument();
  });

  test('renders audio and hotkey sections expanded by default', async () => {
    const Settings = (await import('../pages/Settings')).default;
    render(<Settings />);
    // These are default expanded panels
    expect(screen.getByText('音频设备')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('点击"录制"后按下快捷键')).toBeInTheDocument();
  });

  test('renders general settings section', async () => {
    const Settings = (await import('../pages/Settings')).default;
    render(<Settings />);
    // Collapse headers should be visible even when collapsed
    expect(screen.getByText('通用')).toBeInTheDocument();
    expect(screen.getByText('历史管理')).toBeInTheDocument();
  });
});
