import { useCallback } from 'react';
import { Button, Col, Empty, Row, Space, Spin, message } from 'antd';
import { FolderOpenOutlined, ImportOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { formatError } from '../../lib/logger';
import useAppStore from '../../stores/useAppStore';
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
  const selectedModelId = useAppStore((s) => s.settings.selected_model_id);

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
    <section data-testid="custom-models-section">
      <Space
        style={{
          width: '100%',
          justifyContent: 'flex-end',
          marginBottom: 12,
        }}
      >
        <Button icon={<ImportOutlined />} onClick={handleImportExample}>
          {t('customModels.importExample')}
        </Button>
        <Button icon={<FolderOpenOutlined />} onClick={handleOpenFolder}>
          {t('customModels.openFolder')}
        </Button>
      </Space>

      {loading ? (
        <Spin />
      ) : items.length === 0 ? (
        <Empty description={t('customModels.empty')} />
      ) : (
        <Row gutter={[16, 16]}>
          {items.map((item) => {
            const key = item.kind === 'invalid' ? item.sourcePath : item.spec.id;
            const isActive =
              item.kind !== 'invalid' && selectedModelId === item.spec.id;
            return (
              <Col xs={24} sm={12} md={8} key={key}>
                <CustomModelCard
                  status={item}
                  isActive={isActive}
                  onChange={safeRefresh}
                />
              </Col>
            );
          })}
        </Row>
      )}
    </section>
  );
}

export default CustomModelsSection;
