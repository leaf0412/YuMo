import { useCallback } from 'react';
import { Button, Empty, Space, Spin, Typography, message } from 'antd';
import { FolderOpenOutlined, ImportOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { formatError } from '../../lib/logger';
import { useCustomModels } from './useCustomModels';
import { getCustomBridge } from './bridge';
import { CustomModelCard } from './CustomModelCard';

/**
 * Settings page section for custom YAML-defined models.
 *
 * Aggregates list scan into per-item status (via useCustomModels) and
 * delegates rendering to <CustomModelCard /> for the four states.
 * Surface scan errors via toast — silent failures hide misconfigured
 * YAML files from the user.
 */
export function CustomModelsSection() {
  const { t } = useTranslation();

  const reportScanError = useCallback(
    (err: unknown) => {
      const msg = formatError(err, 'unknown error');
      message.error(t('customModels.scanFailed', { msg }));
    },
    [t],
  );

  const { items, loading, refresh } = useCustomModels({ onError: reportScanError });

  const safeRefresh = useCallback(() => {
    refresh().catch(reportScanError);
  }, [refresh, reportScanError]);

  const handleImportExample = async () => {
    try {
      await getCustomBridge().invoke('custom-import-example', 'mimo.yaml');
      message.success(t('customModels.importSuccess'));
      safeRefresh();
    } catch (e) {
      message.error(formatError(e, t('customModels.importFailed')));
    }
  };

  const handleOpenFolder = async () => {
    try {
      await getCustomBridge().invoke('custom-open-dir');
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
          {items.map((item) => (
            <CustomModelCard
              key={item.kind === 'invalid' ? item.sourcePath : item.spec.id}
              status={item}
              onChange={safeRefresh}
            />
          ))}
        </Space>
      )}
    </section>
  );
}

export default CustomModelsSection;
