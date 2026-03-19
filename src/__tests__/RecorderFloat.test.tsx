import { render, screen, act } from '@testing-library/react';
import { describe, test, expect, vi, beforeEach } from 'vitest';
import {
  EVENT_RECORDING_STATE,
  PIPELINE_RECORDING,
  PIPELINE_TRANSCRIBING,
  PIPELINE_LABELS,
} from '../lib/pipeline';

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
    expect(container.firstChild).toBeNull();
  });

  test('renders recording state after event', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    await act(async () => { render(<RecorderFloat />); });

    await act(async () => {
      eventListeners[EVENT_RECORDING_STATE]?.({ payload: { state: PIPELINE_RECORDING } });
    });

    expect(screen.getByText(PIPELINE_LABELS[PIPELINE_RECORDING])).toBeInTheDocument();
  });

  test('renders timer when recording', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    await act(async () => { render(<RecorderFloat />); });

    await act(async () => {
      eventListeners[EVENT_RECORDING_STATE]?.({ payload: { state: PIPELINE_RECORDING } });
    });

    expect(screen.getByText('0:00')).toBeInTheDocument();
  });

  test('renders transcribing state', async () => {
    const RecorderFloat = (await import('../windows/RecorderFloat')).default;
    await act(async () => { render(<RecorderFloat />); });

    await act(async () => {
      eventListeners[EVENT_RECORDING_STATE]?.({ payload: { state: PIPELINE_TRANSCRIBING } });
    });

    expect(screen.getByText(PIPELINE_LABELS[PIPELINE_TRANSCRIBING])).toBeInTheDocument();
  });
});
