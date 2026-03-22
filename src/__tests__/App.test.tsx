import { render, screen, waitFor } from '@testing-library/react';
import { describe, test, expect, vi } from 'vitest';
import App from '../App';

// Mock Tauri API
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    onFocusChanged: vi.fn(() => Promise.resolve(() => {})),
  })),
}));

describe('App Shell', () => {
  test('renders sidebar with all navigation items', async () => {
    render(<App />);
    // App returns null until fetchSettings resolves; wait for the sidebar to appear.
    // i18n is initialized with zh-CN in test-setup.ts.
    await waitFor(() => {
      expect(screen.getByText('转录历史')).toBeInTheDocument();
    });
    expect(screen.getByText('模型管理')).toBeInTheDocument();
    expect(screen.getByText('词典')).toBeInTheDocument();
    expect(screen.getByText('AI 增强')).toBeInTheDocument();
    expect(screen.getByText('设置')).toBeInTheDocument();
  });

  test('renders app title in sidebar', async () => {
    render(<App />);
    await waitFor(() => {
      // app.name is "语墨" in zh-CN locale
      expect(screen.getByText('语墨')).toBeInTheDocument();
    });
  });
});
