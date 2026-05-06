/**
 * Custom-model UI types: re-export bridge types + UI status discriminator.
 *
 * The hook collapses three IPC calls (scan / check-deps / is-downloaded)
 * into a single CustomModelStatus value per item, so the section component
 * can switch on `kind` without re-running async checks during render.
 */
export type {
  CustomModelSpec,
  CustomModelScanResult,
  CustomDepsCheckResult,
  CustomDepsInstallResult,
  CustomDownloadResult,
} from '../../bridge/types';

import type { CustomModelSpec } from '../../bridge/types';

export type CustomModelStatus =
  | { kind: 'invalid'; error: string; sourcePath: string }
  | { kind: 'depsMissing'; spec: CustomModelSpec; missing: string[] }
  | { kind: 'notDownloaded'; spec: CustomModelSpec }
  | { kind: 'ready'; spec: CustomModelSpec };
