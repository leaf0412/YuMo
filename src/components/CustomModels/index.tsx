import { Button, Empty, Space, Spin, Typography, message } from 'antd';
import { FolderOpenOutlined, ImportOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { formatError } from '../../lib/logger';
import { useCustomModels } from './useCustomModels';

type ElectronAPI = {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
};

function getElectronAPI(): ElectronAPI {
  const api = (window as unknown as { electronAPI?: ElectronAPI }).electronAPI;
  if (!api) {
    throw new Error('window.electronAPI is unavailable — custom models require the Electron host');
  }
  return api;
}

/**
 * Settings page section for custom YAML-defined models.
 *
 * Renders a placeholder per-item; T19 replaces the placeholder with
 * <CustomModelCard /> wired to deps-install / download / trust flows.
 */
export function CustomModelsSection() {
  const { items, loading, refresh } = useCustomModels();
  const { t } = useTranslation();

  const handleImportExample = async () => {
    try {
      await getElectronAPI().invoke('custom-import-example', 'mimo.yaml');
      message.success(t('customModels.importSuccess'));
      refresh();
    } catch (e) {
      message.error(formatError(e, t('customModels.importFailed')));
    }
  };

  const handleOpenFolder = async () => {
    try {
      await getElectronAPI().invoke('custom-open-dir');
    } catch (e) {
      message.error(formatError(e, t('customModels.openFolderFailed')));
    }
  };

  return (
    <section style={{ marginTop: 32 }} data-testid="custom-models-section">
      <Space
        style={{
          width: '100%',
          justifyContent: 'space-between',
          marginBottom: 12,
        }}
      >
        <Typography.Title level={5} style={{ margin: 0 }}>
          {t('customModels.title')}
        </Typography.Title>
        <Space>
          <Button icon={<ImportOutlined />} onClick={handleImportExample}>
            {t('customModels.importExample')}
          </Button>
          <Button icon={<FolderOpenOutlined />} onClick={handleOpenFolder}>
            {t('customModels.openFolder')}
          </Button>
        </Space>
      </Space>

      {loading ? (
        <Spin />
      ) : items.length === 0 ? (
        <Empty description={t('customModels.empty')} />
      ) : (
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          {items.map((item, i) => (
            // T19 will replace this placeholder with <CustomModelCard />.
            <div
              key={i}
              style={{
                padding: 8,
                border: '1px solid #ddd',
                borderRadius: 4,
              }}
              data-testid={`custom-model-item-${i}`}
            >
              {item.kind === 'invalid'
                ? `Invalid: ${item.sourcePath} — ${item.error}`
                : `[${item.kind}] ${item.spec.name} (${item.spec.id})`}
            </div>
          ))}
        </Space>
      )}
    </section>
  );
}

export default CustomModelsSection;
