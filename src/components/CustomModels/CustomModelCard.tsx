import { Alert, Button, Card, Modal, Space, Tag, Typography, message } from 'antd';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CustomModelStatus } from './types';
import { getElectronAPI } from './electronApi';
import { TrustDialog } from './TrustDialog';

interface Props {
  status: CustomModelStatus;
  onChange: () => void;
}

type InstallResult = {
  success: boolean;
  stdout?: string;
  stderr?: string;
  error?: string | null;
};

type DownloadResult = {
  success: boolean;
  paths?: Record<string, string>;
  error?: string;
};

/**
 * Renders one custom-model entry in four discriminated states:
 *   - invalid       → YAML parse error display (no spec available)
 *   - depsMissing   → Alert + "Install Dependencies" action
 *   - notDownloaded → "Download Model" action
 *   - ready         → "Use" action (T20 wires this to TrustDialog)
 *
 * Mutating actions invoke IPC then call `onChange()` so the parent can
 * re-scan. Failures surface via Modal/message — no silent fallback.
 */
export function CustomModelCard({ status, onChange }: Props) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const [trustOpen, setTrustOpen] = useState(false);

  // ---- invalid: no spec, render error-only card -------------------------
  if (status.kind === 'invalid') {
    return (
      <Card>
        <Alert
          type="error"
          message={t('customModels.invalidYaml')}
          description={
            <>
              <div>
                <code>{status.sourcePath}</code>
              </div>
              <pre style={{ whiteSpace: 'pre-wrap', margin: 0 }}>
                {status.error}
              </pre>
            </>
          }
        />
      </Card>
    );
  }

  const spec = status.spec;
  const langs = Object.keys(spec.languages).join(', ');

  const handleInstall = async () => {
    setBusy(true);
    try {
      const res = (await getElectronAPI().invoke(
        'custom-install-deps',
        spec.sourcePath,
      )) as InstallResult;
      if (res.success) {
        message.success(t('customModels.installSuccess'));
        onChange();
      } else {
        Modal.error({
          title: t('customModels.installFailed'),
          content: (
            <pre style={{ whiteSpace: 'pre-wrap' }}>
              {res.stderr || res.error || 'unknown'}
            </pre>
          ),
          width: 700,
        });
      }
    } catch (err) {
      Modal.error({
        title: t('customModels.installFailed'),
        content: String(err),
      });
    } finally {
      setBusy(false);
    }
  };

  const handleDownload = async () => {
    setBusy(true);
    try {
      const res = (await getElectronAPI().invoke(
        'custom-download',
        spec.sourcePath,
      )) as DownloadResult;
      if (res.success) {
        message.success(t('customModels.downloadSuccess'));
        onChange();
      } else {
        Modal.error({
          title: t('customModels.downloadFailed'),
          content: res.error || 'unknown',
        });
      }
    } catch (err) {
      Modal.error({
        title: t('customModels.downloadFailed'),
        content: String(err),
      });
    } finally {
      setBusy(false);
    }
  };

  const handleUse = () => {
    setTrustOpen(true);
  };

  // Activation: mark this model as the selected one via the existing
  // `select-model` IPC channel. This mirrors what the MLX path does after
  // loading (it calls Tauri's `select_model` which sets `selected_model_id`).
  //
  // KNOWN GAP (deferred): the daemon `load` action accepts a `provider`
  // parameter (T13) but the napi `daemon_load_model` bridge does not yet
  // forward it — see `napi/src/lib.rs::daemon_load_model`. Until that's
  // wired, we don't trigger a daemon-side load for custom models here;
  // selection-only activation is sufficient for the trust-dialog milestone.
  const handleTrust = useCallback(async () => {
    setTrustOpen(false);
    try {
      await getElectronAPI().invoke('select-model', { modelId: spec.id });
      message.success(t('customModels.activated', { name: spec.name }));
      onChange();
    } catch (err) {
      Modal.error({
        title: t('customModels.activateFailed'),
        content: String(err),
      });
    }
  }, [spec.id, spec.name, onChange, t]);

  const handleRemove = () => {
    Modal.confirm({
      title: t('customModels.removeConfirmTitle'),
      content: t('customModels.removeConfirmContent', { size: spec.sizeMb }),
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await getElectronAPI().invoke('custom-remove', spec.sourcePath);
          message.success(t('customModels.removeSuccess'));
          onChange();
        } catch (err) {
          Modal.error({
            title: t('customModels.removeFailed'),
            content: String(err),
          });
        }
      },
    });
  };

  return (
    <>
      <Card>
      <Space direction="vertical" style={{ width: '100%' }} size="small">
        <Space style={{ justifyContent: 'space-between', width: '100%' }}>
          <Typography.Text strong>{spec.name}</Typography.Text>
          <Tag>{t('customModels.customBadge')}</Tag>
        </Space>

        {spec.description && (
          <Typography.Text type="secondary">{spec.description}</Typography.Text>
        )}

        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          {t('customModels.metaLine', {
            langs,
            size: (spec.sizeMb / 1024).toFixed(1),
            speed: spec.speed,
            accuracy: spec.accuracy,
          })}
        </Typography.Text>

        {status.kind === 'depsMissing' && (
          <Alert
            type="warning"
            message={t('customModels.depsMissing', {
              pkgs: status.missing.join(', '),
            })}
          />
        )}

        <Space>
          {status.kind === 'depsMissing' && (
            <Button type="primary" loading={busy} onClick={handleInstall}>
              {t('customModels.installDeps')}
            </Button>
          )}
          {status.kind === 'notDownloaded' && (
            <Button type="primary" loading={busy} onClick={handleDownload}>
              {t('customModels.downloadModel')}
            </Button>
          )}
          {status.kind === 'ready' && (
            <Button type="primary" onClick={handleUse}>
              {t('customModels.use')}
            </Button>
          )}
          <Button danger onClick={handleRemove}>
            {t('customModels.remove')}
          </Button>
        </Space>
      </Space>
    </Card>
      <TrustDialog
        spec={spec}
        open={trustOpen}
        onCancel={() => setTrustOpen(false)}
        onTrust={handleTrust}
      />
    </>
  );
}

export default CustomModelCard;
