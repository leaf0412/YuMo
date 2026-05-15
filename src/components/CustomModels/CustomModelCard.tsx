/**
 * Custom-model card. Translates a `CustomModelStatus` (4-way union) into
 * the unified `ModelCardViewModel` and delegates rendering to
 * <ModelCard />, so custom models look and behave like the local/cloud
 * tabs. The TrustDialog stays here since it's specific to custom models.
 */
import { Modal, message } from 'antd';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CustomModelStatus } from './types';
import { getCustomBridge } from './bridge';
import { invoke, formatError } from '../../lib/logger';
import { TrustDialog } from './TrustDialog';
import { ModelCard } from '../ModelCard';
import useAppStore from '../../stores/useAppStore';
import type {
  ModelAction,
  ModelCardViewModel,
  ModelStatus,
} from '../ModelCard/types';

interface Props {
  status: CustomModelStatus;
  /** True when this model is the currently selected one in app settings. */
  isActive: boolean;
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

export function CustomModelCard({ status, isActive, onChange }: Props) {
  const { t } = useTranslation();
  const selectModel = useAppStore((s) => s.selectModel);
  const [busy, setBusy] = useState(false);
  const [trustOpen, setTrustOpen] = useState(false);

  // ---- invalid: no spec, render error-only card via ModelCard --------------
  if (status.kind === 'invalid') {
    const vm: ModelCardViewModel = {
      kind: 'invalid',
      sourcePath: status.sourcePath,
      title: t('customModels.invalidYaml'),
      error: status.error,
    };
    return <ModelCard vm={vm} />;
  }

  const spec = status.spec;
  const langs = Object.keys(spec.languages).join(', ');

  const handleInstall = async () => {
    setBusy(true);
    try {
      const res = (await getCustomBridge().invoke(
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
        content: formatError(err, t('customModels.installFailed')),
      });
    } finally {
      setBusy(false);
    }
  };

  const handleDownload = async () => {
    setBusy(true);
    try {
      const res = (await getCustomBridge().invoke(
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
        content: formatError(err, t('customModels.downloadFailed')),
      });
    } finally {
      setBusy(false);
    }
  };

  const handleUse = () => {
    setTrustOpen(true);
  };

  // Activation: daemon_load_model swaps the in-memory model (build_load_command
  // on the Rust side detects the .yaml suffix and adds provider=custom + dirs);
  // select_model persists the selection across restarts. Without step 1 the
  // daemon would keep serving whatever model was loaded before.
  const handleTrust = useCallback(async () => {
    setTrustOpen(false);
    setBusy(true);
    try {
      await invoke('daemon_load_model', { modelId: spec.id });
      await selectModel(spec.id);
      message.success(t('customModels.activated', { name: spec.name }));
      onChange();
    } catch (err) {
      Modal.error({
        title: t('customModels.activateFailed'),
        content: formatError(err, t('customModels.activateFailed')),
      });
    } finally {
      setBusy(false);
    }
  }, [spec.id, spec.sourcePath, spec.name, onChange, selectModel, t]);

  const handleRemove = () => {
    Modal.confirm({
      title: t('customModels.removeConfirmTitle'),
      content: t('customModels.removeConfirmContent', { size: spec.sizeMb }),
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await getCustomBridge().invoke('custom-remove', spec.sourcePath);
          message.success(t('customModels.removeSuccess'));
          onChange();
        } catch (err) {
          Modal.error({
            title: t('customModels.removeFailed'),
            content: formatError(err, t('customModels.removeFailed')),
          });
        }
      },
    });
  };

  // ---- Build status + actions for the unified card -----------------------
  let cardStatus: ModelStatus;
  const actions: ModelAction[] = [];

  if (status.kind === 'depsMissing') {
    cardStatus = { kind: 'needsDeps', missing: status.missing };
    actions.push({
      key: 'install',
      label: t('models.action.installDeps'),
      type: 'primary',
      loading: busy,
      onClick: handleInstall,
    });
  } else if (status.kind === 'notDownloaded') {
    cardStatus = { kind: 'notDownloaded' };
    actions.push({
      key: 'download',
      label: t('models.action.download'),
      type: 'primary',
      loading: busy,
      onClick: handleDownload,
    });
  } else {
    // status.kind === 'ready'
    cardStatus = isActive ? { kind: 'active' } : { kind: 'available' };
    if (!isActive) {
      actions.push({
        key: 'use',
        label: t('models.action.use'),
        type: 'primary',
        loading: busy,
        onClick: handleUse,
      });
    }
  }
  actions.push({
    key: 'delete',
    label: t('models.action.delete'),
    danger: true,
    disabled: busy,
    onClick: handleRemove,
  });

  const vm: ModelCardViewModel = {
    kind: 'normal',
    id: spec.id,
    name: spec.name,
    description: spec.description ?? undefined,
    badge: { text: t('models.badge.custom'), color: 'purple' },
    meta: [
      { label: t('models.label.size'), value: `${(spec.sizeMb / 1024).toFixed(1)} GB` },
      { label: t('models.label.language'), value: langs },
    ],
    status: cardStatus,
    actions,
    alert:
      status.kind === 'depsMissing'
        ? {
            type: 'warning',
            message: t('customModels.depsMissing', {
              pkgs: status.missing.join(', '),
            }),
          }
        : undefined,
    testId: `model-${spec.id}`,
  };

  return (
    <>
      <ModelCard vm={vm} />
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
