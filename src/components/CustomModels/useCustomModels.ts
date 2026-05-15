import { useCallback, useEffect, useState } from 'react';
import type {
  CustomModelStatus,
  CustomModelScanResult,
  CustomDepsCheckResult,
} from './types';
import { getCustomBridge } from './bridge';

/**
 * Aggregates list-custom-models + custom-check-deps + custom-is-downloaded
 * into a CustomModelStatus[] for the settings section.
 *
 * Each scan-error becomes an `invalid` entry; each ok spec is then probed
 * for deps and download state in order. Errors during the per-spec probes
 * are surfaced as failures (no silent fallback) — `refresh()` rejects so
 * callers can `.catch()` and toast, and `onError` is invoked for the
 * auto-refresh on mount so the initial scan failure is also visible.
 */
export interface UseCustomModelsOptions {
  /** Called when the auto-refresh on mount throws. Manual refresh() callers
   *  should attach their own .catch — this only covers the initial scan. */
  onError?: (err: unknown) => void;
}

export function useCustomModels(options: UseCustomModelsOptions = {}) {
  const { onError } = options;
  const [items, setItems] = useState<CustomModelStatus[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const api = getCustomBridge();
      const scan = (await api.invoke('list-custom-models')) as CustomModelScanResult;
      const next: CustomModelStatus[] = [];

      for (const e of scan.errors) {
        next.push({ kind: 'invalid', error: e.error, sourcePath: e.path });
      }

      for (const spec of scan.ok) {
        const deps = (await api.invoke(
          'custom-check-deps',
          spec.sourcePath,
        )) as CustomDepsCheckResult;
        if (!deps.allInstalled) {
          next.push({ kind: 'depsMissing', spec, missing: deps.missing });
          continue;
        }
        const downloaded = (await api.invoke(
          'custom-is-downloaded',
          spec.id,
        )) as boolean;
        next.push(
          downloaded ? { kind: 'ready', spec } : { kind: 'notDownloaded', spec },
        );
      }

      setItems(next);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh().catch((err) => {
      if (onError) {
        onError(err);
      }
    });
  }, [refresh, onError]);

  return { items, loading, refresh };
}
