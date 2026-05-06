import { Alert, Button, Card, Modal, Space, Tag, Typography, message } from 'antd';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CustomModelStatus } from './types';
import { getElectronAPI } from './electronApi';

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
    // T20 will replace this stub with a real TrustDialog flow that
    // persists the user's trust decision and activates the model in the
    // global model-selection store.
    message.info(t('customModels.useTodoT20', { name: spec.name }));
  };

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
  );
}

export default CustomModelCard;
