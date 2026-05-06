import { Modal, Typography } from 'antd';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CustomModelSpec } from './types';
import { getElectronAPI } from './electronApi';

interface Props {
  spec: CustomModelSpec;
  open: boolean;
  onCancel: () => void;
  onTrust: () => void;
}

/**
 * First-time-use trust prompt for a custom model.
 *
 * Behaviour:
 *   - On open, queries `custom-is-trusted` for the spec id.
 *     - Already trusted → calls `onTrust()` immediately and renders nothing.
 *     - Not trusted     → renders the modal with the warning copy.
 *   - On confirm, writes the id to `~/.voiceink/custom_models/.trusted` via
 *     `custom-set-trusted` and calls `onTrust()`. Persistence failure does
 *     not block: we still allow this single use (so the user is never stuck),
 *     but the next time the dialog will re-prompt because the file write
 *     didn't take effect.
 *   - Trust state is keyed by `spec.id` (T16 IPC contract), not source path.
 */
export function TrustDialog({ spec, open, onCancel, onTrust }: Props) {
  const { t } = useTranslation();
  // null = checking; true = previously trusted (auto-skip); false = needs prompt.
  const [previouslyTrusted, setPreviouslyTrusted] = useState<boolean | null>(null);

  useEffect(() => {
    if (!open) {
      setPreviouslyTrusted(null);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const trusted = (await getElectronAPI().invoke(
          'custom-is-trusted',
          spec.id,
        )) as boolean;
        if (cancelled) return;
        setPreviouslyTrusted(trusted);
        if (trusted) {
          // Auto-trust path: skip the dialog and activate immediately.
          onTrust();
        }
      } catch {
        if (!cancelled) setPreviouslyTrusted(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, spec.id, onTrust]);

  // Don't render until we know the trust status; once previouslyTrusted is
  // true the effect already called onTrust() and the parent should close.
  if (!open || previouslyTrusted !== false) return null;

  return (
    <Modal
      open
      title={t('customModels.trustTitle')}
      okText={t('customModels.trustContinue')}
      cancelText={t('customModels.trustCancel')}
      onCancel={onCancel}
      onOk={async () => {
        try {
          await getElectronAPI().invoke('custom-set-trusted', spec.id);
        } catch {
          // Persistence failure is surfaced indirectly: next time we will
          // re-prompt. But do not block the current activation — the user
          // already clicked "Trust and Continue".
        }
        onTrust();
      }}
    >
      <Typography.Paragraph>
        {t('customModels.trustBody', { module: spec.pythonModule })}
      </Typography.Paragraph>
      <Typography.Text type="secondary">
        {t('customModels.trustWarning')}
      </Typography.Text>
    </Modal>
  );
}

export default TrustDialog;
