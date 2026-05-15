/**
 * Generic model card. Used by all three provider tabs (local / cloud / custom)
 * and the YAML-parse-error variant.
 *
 * Callers translate their domain shape into a `ModelCardViewModel` and
 * pass it in; everything visual lives here.
 */
import { Alert, Button, Card, Progress, Space, Tag, Typography } from 'antd';
import { CheckCircleOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type {
  ModelBadge,
  ModelCardViewModel,
  ModelStatus,
} from './types';

interface Props {
  vm: ModelCardViewModel;
}

export function ModelCard({ vm }: Props) {
  const { t } = useTranslation();

  if (vm.kind === 'invalid') {
    return (
      <Card data-testid="model-card-invalid">
        <Alert
          type="error"
          message={vm.title}
          description={
            <>
              <div>
                <code>{vm.sourcePath}</code>
              </div>
              <pre style={{ whiteSpace: 'pre-wrap', margin: 0 }}>
                {vm.error}
              </pre>
            </>
          }
        />
      </Card>
    );
  }

  const isActive = vm.status.kind === 'active';
  const cardStyle = isActive ? { borderColor: '#52c41a' } : undefined;

  return (
    <Card
      style={cardStyle}
      styles={{ body: { padding: 16 } }}
      data-testid={vm.testId ?? `model-${vm.id}`}
    >
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        <Space style={{ width: '100%', justifyContent: 'space-between' }}>
          <Space size={8}>
            {vm.icon}
            <Typography.Text strong>{vm.name}</Typography.Text>
          </Space>
          <StatusTag status={vm.status} badge={vm.badge} t={t} />
        </Space>

        {vm.description && (
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {vm.description}
          </Typography.Text>
        )}

        {vm.meta.length > 0 && (
          <Space size={16} wrap>
            {vm.meta.map((m) => (
              <span key={m.label}>
                <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                  {m.label}{' '}
                </Typography.Text>
                <Typography.Text style={{ fontSize: 12 }}>{m.value}</Typography.Text>
              </span>
            ))}
          </Space>
        )}

        {vm.alert && (
          <Alert
            type={vm.alert.type}
            message={vm.alert.message}
            style={{ padding: '6px 10px' }}
          />
        )}

        {vm.status.kind === 'downloading' && (
          <>
            {vm.status.note && (
              <Typography.Text type="warning" style={{ fontSize: 12 }}>
                {vm.status.note}
              </Typography.Text>
            )}
            {vm.status.percent != null && (
              <Progress percent={vm.status.percent} size="small" status="active" />
            )}
          </>
        )}

        {vm.extras}

        {vm.actions.length > 0 && (
          <Space style={{ justifyContent: 'flex-end', width: '100%' }}>
            {vm.actions.map((a) => (
              <Button
                key={a.key}
                type={a.type ?? 'default'}
                danger={a.danger}
                loading={a.loading}
                disabled={a.disabled}
                size="small"
                onClick={a.onClick}
              >
                {a.label}
              </Button>
            ))}
          </Space>
        )}
      </Space>
    </Card>
  );
}

interface StatusTagProps {
  status: ModelStatus;
  badge?: ModelBadge;
  t: (key: string) => string;
}

/**
 * Right-aligned tag combining provider badge + lifecycle state.
 *
 * Rendering order, in priority:
 *   - active                → green "正在使用"
 *   - needsDeps             → red   "缺依赖"
 *   - notDownloaded         → grey  "未下载"
 *   - downloading           → blue  "下载中"
 *   - available + badge     → badge color (blue/cyan/purple)
 *   - available no badge    → blue  "可用"
 */
function StatusTag({ status, badge, t }: StatusTagProps) {
  if (status.kind === 'active') {
    return (
      <Tag color="green" icon={<CheckCircleOutlined />}>
        {t('models.status.active')}
      </Tag>
    );
  }
  if (status.kind === 'needsDeps') {
    return <Tag color="red">{t('models.status.needsDeps')}</Tag>;
  }
  if (status.kind === 'notDownloaded') {
    return <Tag>{t('models.status.notDownloaded')}</Tag>;
  }
  if (status.kind === 'downloading') {
    return <Tag color="blue">{t('models.status.downloading')}</Tag>;
  }
  // available
  if (badge) {
    return <Tag color={badge.color ?? 'blue'}>{badge.text}</Tag>;
  }
  return <Tag color="blue">{t('models.status.available')}</Tag>;
}

export default ModelCard;
