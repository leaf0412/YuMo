/**
 * Pipeline state constants shared across all frontend windows/pages.
 */

// --- State values -----------------------------------------------------------

export const PIPELINE_IDLE = 'idle' as const;
export const PIPELINE_RECORDING = 'recording' as const;
export const PIPELINE_TRANSCRIBING = 'transcribing' as const;
export const PIPELINE_ENHANCING = 'enhancing' as const;
export const PIPELINE_PASTING = 'pasting' as const;

export type PipelineState =
  | typeof PIPELINE_IDLE
  | typeof PIPELINE_RECORDING
  | typeof PIPELINE_TRANSCRIBING
  | typeof PIPELINE_ENHANCING
  | typeof PIPELINE_PASTING;

// --- Tauri event / command names --------------------------------------------

export const EVENT_RECORDING_STATE = 'recording-state';
export const CMD_GET_PIPELINE_STATE = 'get_pipeline_state';

// --- Display labels (strategy map) ------------------------------------------

export const PIPELINE_LABELS: Record<PipelineState, string> = {
  [PIPELINE_RECORDING]: '录音中',
  [PIPELINE_TRANSCRIBING]: '转录中...',
  [PIPELINE_ENHANCING]: '增强中...',
  [PIPELINE_PASTING]: '粘贴中...',
  [PIPELINE_IDLE]: '',
};

// --- UI style tokens (strategy map) -----------------------------------------

export const COLOR_ACTIVE = '#ff4d4f';
export const COLOR_PROCESSING = '#1890ff';

export const PIPELINE_COLORS: Record<PipelineState, string> = {
  [PIPELINE_RECORDING]: COLOR_ACTIVE,
  [PIPELINE_TRANSCRIBING]: COLOR_PROCESSING,
  [PIPELINE_ENHANCING]: COLOR_PROCESSING,
  [PIPELINE_PASTING]: COLOR_PROCESSING,
  [PIPELINE_IDLE]: COLOR_PROCESSING,
};

export const PIPELINE_ANIMATIONS: Record<PipelineState, string> = {
  [PIPELINE_RECORDING]: 'pulse 1.5s infinite',
  [PIPELINE_TRANSCRIBING]: 'none',
  [PIPELINE_ENHANCING]: 'none',
  [PIPELINE_PASTING]: 'none',
  [PIPELINE_IDLE]: 'none',
};

// --- Helpers ----------------------------------------------------------------

/** Parse a raw state string from backend, default to IDLE if unknown. */
export function parsePipelineState(raw: string | undefined | null): PipelineState {
  const valid: ReadonlySet<string> = new Set([
    PIPELINE_IDLE,
    PIPELINE_RECORDING,
    PIPELINE_TRANSCRIBING,
    PIPELINE_ENHANCING,
    PIPELINE_PASTING,
  ]);
  return valid.has(raw ?? '') ? (raw as PipelineState) : PIPELINE_IDLE;
}
