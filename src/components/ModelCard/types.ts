/**
 * Unified view model for all three kinds of models (local, cloud, custom).
 *
 * The card UI itself stays implementation-agnostic — each provider page
 * (or custom-models module) is responsible for translating its own
 * domain shape into this view model and feeding it to <ModelCard />.
 */
import type { ReactNode } from 'react';

/** Lifecycle state visible to the user. Internal details (daemon-loaded
 *  vs cached, cloud-credentials-configured-but-untested, etc.) collapse
 *  into these five states; the underlying implementation keeps that
 *  knowledge to itself. */
export type ModelStatus =
  | { kind: 'needsDeps'; missing: string[] }
  | { kind: 'notDownloaded' }
  | { kind: 'downloading'; percent?: number; note?: ReactNode }
  | { kind: 'available' }
  | { kind: 'active' };

export interface ModelMetaItem {
  /** Localised label, e.g. "大小" / "Size". */
  label: string;
  /** Pre-formatted value (string or pre-rendered tags). */
  value: ReactNode;
}

export interface ModelAction {
  /** Stable key for React reconciliation. */
  key: string;
  label: ReactNode;
  type?: 'primary' | 'default';
  danger?: boolean;
  loading?: boolean;
  disabled?: boolean;
  onClick: () => void | Promise<void>;
}

export interface ModelBadge {
  text: string;
  /** antd Tag color string, e.g. "blue" / "geekblue" / "purple". */
  color?: string;
}

export interface ModelAlert {
  type: 'warning' | 'error';
  message: ReactNode;
}

/** A normal, parse-successful model card. */
export interface NormalModelViewModel {
  kind: 'normal';
  id: string;
  name: string;
  description?: ReactNode;
  meta: ModelMetaItem[];
  badge?: ModelBadge;
  icon?: ReactNode;
  status: ModelStatus;
  actions: ModelAction[];
  /** Optional warning displayed between meta and actions
   *  (e.g. dependency missing alert for custom YAML models). */
  alert?: ModelAlert;
  /** Slot for provider-specific controls (temperature / max_tokens
   *  sliders for daemon-backed models). Rendered above the action row. */
  extras?: ReactNode;
  /** data-testid passed through to the card root. */
  testId?: string;
}

/** YAML parse failure for a custom-model spec — has no id/name to show. */
export interface InvalidModelViewModel {
  kind: 'invalid';
  /** Path to the YAML file that failed to parse — used as React key. */
  sourcePath: string;
  /** Localised header (e.g. "YAML 解析失败"). */
  title: string;
  /** Raw parse error from yaml.safe_load / serde_yaml. */
  error: string;
}

export type ModelCardViewModel = NormalModelViewModel | InvalidModelViewModel;
