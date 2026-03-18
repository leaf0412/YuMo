import { render, screen, act } from '@testing-library/react';
import { describe, test, expect, vi, beforeEach } from 'vitest';

// Store listener callbacks so we can fire events in tests
const eventListeners: Record<string, (e: unknown) => void> = {};

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((eventName: string, handler: (e: unknown) => void) => {
    eventListeners[eventName] = handler;
    return Promise.resolve(() => {});
  }),
}));

beforeEach(() => {
  Object.keys(eventListeners).forEach((k) => delete eventListeners[k]);
});

describe('RecorderFloat', () => {
  test('renders null when idle (default state)', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    const { container } = render(<RecorderFloat />);
    // Component returns null when state is idle
    expect(container.firstChild).toBeNull();
  });

  test('renders recording state after event', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    render(<RecorderFloat />);

    // Simulate recording-state event
    await act(async () => {
      eventListeners['recording-state']?.({ payload: { state: 'recording' } });
    });

    expect(screen.getByText('录音中')).toBeInTheDocument();
  });

  test('renders timer when recording', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    render(<RecorderFloat />);

    await act(async () => {
      eventListeners['recording-state']?.({ payload: { state: 'recording' } });
    });

    expect(screen.getByText('0:00')).toBeInTheDocument();
  });

  test('renders transcribing state', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    render(<RecorderFloat />);

    await act(async () => {
      eventListeners['recording-state']?.({ payload: { state: 'transcribing' } });
    });

    expect(screen.getByText('转录中...')).toBeInTheDocument();
  });
});
