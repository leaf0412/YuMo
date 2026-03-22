import { useEffect, useState, useCallback } from 'react';
import { Flex, Space, Button, Typography, message } from 'antd';
import { CheckCircleOutlined, CloseCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { invoke, formatError } from '../lib/logger';

const { Text } = Typography;

interface PermissionState {
  microphone: boolean;
  accessibility: boolean;
}

export default function Permissions() {
  const { t } = useTranslation();
  const [permissions, setPermissions] = useState<PermissionState>({ microphone: false, accessibility: false });

  const fetchPermissions = useCallback(async () => {
    try {
      const result = await invoke<PermissionState>('check_permissions');
      setPermissions(result);
    } catch { /* ignore */ }
  }, []);

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
        const result = await invoke<PermissionState>('check_permissions');
        setPermissions(result);
        if ((type === 'microphone' && result.microphone) || (type === 'accessibility' && result.accessibility)) {
          clearInterval(interval);
        }
      }, 1000);
      setTimeout(() => clearInterval(interval), 30000);
    } catch (e) {
      message.error(formatError(e, t('settings.permissionRequestFailed')));
    }
  }, [t]);

  const settingRow = (label: string, control: React.ReactNode) => (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0' }}>
      <Text>{label}</Text>
      {control}
    </div>
  );

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
            {!permissions.microphone && (
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
            {!permissions.accessibility && (
              <Button size="small" type="link" onClick={() => handleRequestPermission('accessibility')}>
                {t('settings.permGrant')}
              </Button>
            )}
          </Space>,
        )}
        <Button size="small" style={{ alignSelf: 'flex-start' }} onClick={fetchPermissions}>
          {t('settings.permRefresh')}
        </Button>
      </Flex>
    </Flex>
  );
}
