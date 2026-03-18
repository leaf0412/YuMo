import { render, screen } from '@testing-library/react';
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
  test('renders sidebar with all navigation items', () => {
    render(<App />);
    // '仪表盘' appears in both menu and page content (default route)
    expect(screen.getAllByText('仪表盘').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('转录历史')).toBeInTheDocument();
    expect(screen.getByText('模型管理')).toBeInTheDocument();
    expect(screen.getByText('词典')).toBeInTheDocument();
    expect(screen.getByText('AI 增强')).toBeInTheDocument();
    expect(screen.getByText('设置')).toBeInTheDocument();
  });

  test('renders app title in sidebar', () => {
    render(<App />);
    expect(screen.getByText('VoiceInk')).toBeInTheDocument();
  });
});
