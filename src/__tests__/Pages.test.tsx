import { render, screen, waitFor } from '@testing-library/react';
import { describe, test, expect, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';

// Import test utils for mocking
import './test-utils';

// Dashboard now shows a statistics page. When get_statistics returns
// { total_sessions: 0 }, the empty-state ("暂无录音数据") is rendered.
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn((cmd: string) => {
    if (cmd === 'get_statistics') return Promise.resolve({ total_sessions: 0 });
    return Promise.resolve({});
  }),
}));

describe('Dashboard', () => {
  test('renders statistics empty state', async () => {
    const Dashboard = (await import('../pages/Dashboard')).default;
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );
    // Empty state shown when no sessions recorded yet
    await waitFor(() => {
      expect(screen.getByText('暂无录音数据')).toBeInTheDocument();
    });
  });

  test('renders empty state hint', async () => {
    const Dashboard = (await import('../pages/Dashboard')).default;
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByText('开始你的第一次语音录入吧')).toBeInTheDocument();
    });
  });
});

describe('History', () => {
  test('renders search input and clear button', async () => {
    const History = (await import('../pages/History')).default;
    render(
      <MemoryRouter>
        <History />
      </MemoryRouter>,
    );
    expect(screen.getByPlaceholderText('搜索转录内容...')).toBeInTheDocument();
    expect(screen.getByText('清空全部')).toBeInTheDocument();
  });
});

describe('Models', () => {
  test('renders page title and import button', async () => {
    const Models = (await import('../pages/Models')).default;
    render(
      <MemoryRouter>
        <Models />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByText('模型管理')).toBeInTheDocument();
    });
    expect(screen.getByText('导入模型')).toBeInTheDocument();
  });

  test('renders cloud model tab', async () => {
    const Models = (await import('../pages/Models')).default;
    render(
      <MemoryRouter>
        <Models />
      </MemoryRouter>,
    );
    await waitFor(() => {
      // Cloud tab is always rendered (MLX tab only appears when mlxModels.length > 0)
      expect(screen.getByTestId('cloud-models-tab')).toBeInTheDocument();
    });
  });

  test('renders API key input', async () => {
    const Models = (await import('../pages/Models')).default;
    render(
      <MemoryRouter>
        <Models />
      </MemoryRouter>,
    );
    await waitFor(() => {
      expect(screen.getByPlaceholderText('输入 API Key')).toBeInTheDocument();
    });
  });
});

describe('Dictionary', () => {
  test('renders tabs for vocabulary and replacements', async () => {
    const Dictionary = (await import('../pages/Dictionary')).default;
    render(
      <MemoryRouter>
        <Dictionary />
      </MemoryRouter>,
    );
    expect(screen.getByText('词典')).toBeInTheDocument();
    expect(screen.getByText('词汇表')).toBeInTheDocument();
    expect(screen.getByText('替换规则')).toBeInTheDocument();
  });

  test('renders add vocabulary input', async () => {
    const Dictionary = (await import('../pages/Dictionary')).default;
    render(
      <MemoryRouter>
        <Dictionary />
      </MemoryRouter>,
    );
    expect(screen.getByPlaceholderText('添加新词汇...')).toBeInTheDocument();
  });
});

describe('Enhancement', () => {
  test('renders AI enhancement toggle and title', async () => {
    const Enhancement = (await import('../pages/Enhancement')).default;
    render(
      <MemoryRouter>
        <Enhancement />
      </MemoryRouter>,
    );
    expect(screen.getByText('AI 增强')).toBeInTheDocument();
    expect(screen.getByText('启用 AI 增强')).toBeInTheDocument();
  });

  test('renders prompt management section', async () => {
    const Enhancement = (await import('../pages/Enhancement')).default;
    render(
      <MemoryRouter>
        <Enhancement />
      </MemoryRouter>,
    );
    expect(screen.getByText('Prompt 管理')).toBeInTheDocument();
    expect(screen.getByText('新建 Prompt')).toBeInTheDocument();
  });

  test('renders provider and model selects', async () => {
    const Enhancement = (await import('../pages/Enhancement')).default;
    render(
      <MemoryRouter>
        <Enhancement />
      </MemoryRouter>,
    );
    expect(screen.getByText('LLM 服务商')).toBeInTheDocument();
    expect(screen.getByText('模型')).toBeInTheDocument();
  });
});

describe('Settings', () => {
  test('renders settings title', async () => {
    const Settings = (await import('../pages/Settings')).default;
    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>,
    );
    expect(screen.getByText('设置')).toBeInTheDocument();
  });

  test('renders audio and hotkey sections expanded by default', async () => {
    const Settings = (await import('../pages/Settings')).default;
    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>,
    );
    // These are default expanded panels
    expect(screen.getByText('音频设备')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('点击"录制"后按下快捷键')).toBeInTheDocument();
  });

  test('renders general settings section', async () => {
    const Settings = (await import('../pages/Settings')).default;
    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>,
    );
    // Collapse headers should be visible even when collapsed
    expect(screen.getByText('通用')).toBeInTheDocument();
    expect(screen.getByText('历史管理')).toBeInTheDocument();
  });
});
