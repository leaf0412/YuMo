import { render, screen } from '@testing-library/react';
import { describe, test, expect, vi } from 'vitest';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
}));

describe('RecorderFloat', () => {
  test('renders recording state', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    render(<RecorderFloat />);
    expect(screen.getByText('录音中')).toBeInTheDocument();
  });

  test('renders waveform canvas', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    const { container } = render(<RecorderFloat />);
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });

  test('renders timer starting at 0:00', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    render(<RecorderFloat />);
    expect(screen.getByText('0:00')).toBeInTheDocument();
  });
});
