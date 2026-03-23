import { useEffect, useCallback } from 'react';
import { Flex, Space, Button, Typography, Alert, message } from 'antd';
import { CheckCircleOutlined, CloseCircleOutlined, InfoCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { invoke, formatError } from '../lib/logger';
import useAppStore from '../stores/useAppStore';

const { Text } = Typography;

const isMacOS = navigator.userAgent.includes('Macintosh');
const isLinux = navigator.userAgent.includes('Linux');

export default function Permissions() {
  const { t } = useTranslation();
  const { permissions, fetchPermissions } = useAppStore();

  useEffect(() => {
    fetchPermissions();
  }, [fetchPermissions]);

  const permissionIcon = (granted: boolean) =>
    granted
      ? <CheckCircleOutlined style={{ color: '#52c41a' }} />
      : <CloseCircleOutlined style={{ color: '#ff4d4f' }} />;

  const handleRequestPermission = useCallback(async (type: string) => {
    try {
      await invoke('request_permission', { permissionType: type });
      const interval = setInterval(async () => {
        await fetchPermissions();
        const current = useAppStore.getState().permissions;
        if ((type === 'microphone' && current.microphone) || (type === 'accessibility' && current.accessibility)) {
          clearInterval(interval);
        }
      }, 1000);
      setTimeout(() => clearInterval(interval), 30000);
    } catch (e) {
      message.error(formatError(e, t('settings.permissionRequestFailed')));
    }
  }, [fetchPermissions, t]);

  const settingRow = (label: string, control: React.ReactNode) => (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0' }}>
      <Text>{label}</Text>
      {control}
    </div>
  );

  // Linux: clipboard-only mode, no permissions needed
  if (isLinux) {
    return (
      <Flex vertical gap="large" style={{ width: '100%' }}>
        <Typography.Title level={3}>{t('permissions.linuxTitle')}</Typography.Title>
        <Alert
          type="info"
          showIcon
          icon={<InfoCircleOutlined />}
          message={t('permissions.linuxClipboardMode')}
          description={t('permissions.linuxClipboardDesc')}
        />
      </Flex>
    );
  }

  // macOS: show microphone & accessibility permissions
  // Windows: same UI (always granted, mostly informational)
  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Typography.Title level={3}>{t('permissions.title')}</Typography.Title>
      <Flex vertical gap={8} style={{ width: '100%' }}>
        {settingRow(
          t('settings.permMicrophone'),
          <Space>
            {permissionIcon(permissions.microphone)}
            <Text type={permissions.microphone ? 'success' : 'danger'}>
              {permissions.microphone ? t('settings.permGranted') : t('settings.permDenied')}
            </Text>
            {!permissions.microphone && isMacOS && (
              <Button size="small" type="link" onClick={() => handleRequestPermission('microphone')}>
                {t('settings.permGrant')}
              </Button>
            )}
          </Space>,
        )}
        {settingRow(
          t('settings.permAccessibility'),
          <Space>
            {permissionIcon(permissions.accessibility)}
            <Text type={permissions.accessibility ? 'success' : 'danger'}>
              {permissions.accessibility ? t('settings.permGranted') : t('settings.permDenied')}
            </Text>
            {!permissions.accessibility && isMacOS && (
              <Button size="small" type="link" onClick={() => handleRequestPermission('accessibility')}>
                {t('settings.permGrant')}
              </Button>
            )}
          </Space>,
        )}
        <Button size="small" style={{ alignSelf: 'flex-start' }} onClick={() => fetchPermissions()}>
          {t('settings.permRefresh')}
        </Button>
      </Flex>
    </Flex>
  );
}
