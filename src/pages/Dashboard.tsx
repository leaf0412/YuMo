import { useEffect, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import { Card, Flex, Typography, Row, Col, Segmented, Spin, Empty, Tooltip } from 'antd';
import { InfoCircleOutlined } from '@ant-design/icons';
import {
  AudioOutlined,
  EditOutlined,
  DashboardOutlined,
  FieldTimeOutlined,
} from '@ant-design/icons';
import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip as RechartsTooltip,
} from 'recharts';
import { invoke } from '../lib/logger';

const { Title, Text } = Typography;

interface DailyWpm {
  date: string;
  wpm: number;
  session_count: number;
}

interface WpmStats {
  avg: number;
  max: number;
  min: number;
}

interface Statistics {
  total_sessions: number;
  total_words: number;
  total_duration_seconds: number;
  total_keystrokes_saved: number;
  time_saved_minutes: number;
  avg_wpm: number;
  daily_wpm: DailyWpm[];
  wpm_stats: WpmStats;
}

function formatDate(dateStr: string): string {
  const [, m, d] = dateStr.split('-');
  return `${Number(m)}/${Number(d)}`;
}

function formatNumber(n: number): string {
  return n.toLocaleString();
}

export default function Dashboard() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<Statistics | null>(null);
  const [loading, setLoading] = useState(true);
  const [days, setDays] = useState<number>(30);

  const timeRanges = [
    { label: t('dashboard.range7d'), value: 7 },
    { label: t('dashboard.range30d'), value: 30 },
    { label: t('dashboard.range90d'), value: 90 },
    { label: t('dashboard.rangeAll'), value: 0 },
  ];

  const formatTimeSaved = (minutes: number): string => {
    if (minutes < 1) return t('dashboard.timeLessThanMin');
    if (minutes < 60) return t('dashboard.timeMinutes', { count: Math.round(minutes) });
    const hours = Math.floor(minutes / 60);
    const mins = Math.round(minutes % 60);
    return mins > 0
      ? t('dashboard.timeHoursMinutes', { hours, minutes: mins })
      : t('dashboard.timeHours', { hours });
  };

  const loadStats = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<Statistics>('get_statistics', {
        days: days === 0 ? null : days,
      });
      setStats(result);
    } catch { /* logged */ }
    setLoading(false);
  }, [days]);

  useEffect(() => { loadStats(); }, [loadStats]);

  // Auto-refresh stats when a recording completes or data changes
  useEffect(() => {
    const unlistenRecording = listen<{ state: string }>('recording-state', (event) => {
      if (event.payload.state === 'idle') loadStats();
    });
    const unlistenStats = listen('stats-updated', () => loadStats());
    return () => {
      unlistenRecording.then((fn) => fn());
      unlistenStats.then((fn) => fn());
    };
  }, [loadStats]);

  if (loading && !stats) {
    return <Flex justify="center" align="center" style={{ height: 300 }}><Spin size="large" /></Flex>;
  }

  if (!stats || stats.total_sessions === 0) {
    return (
      <Flex vertical align="center" justify="center" gap="middle" style={{ height: 400 }}>
        <Empty description={t('dashboard.noData')} />
        <Text type="secondary">{t('dashboard.noDataHint')}</Text>
      </Flex>
    );
  }

  const { total_sessions, total_words, avg_wpm, total_keystrokes_saved, time_saved_minutes, daily_wpm, wpm_stats } = stats;

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      {/* Top Banner */}
      <div style={{
        background: 'linear-gradient(135deg, #1a3a8a 0%, #2563eb 50%, #3b82f6 100%)',
        borderRadius: 12,
        padding: '28px 32px',
        textAlign: 'center',
      }}>
        <Title level={4} style={{ color: '#fff', margin: 0 }}>
          {t('dashboard.timeSavedBanner', { time: formatTimeSaved(time_saved_minutes) })}{' '}
          <Tooltip title={t('dashboard.timeSavedTooltip')}>
            <InfoCircleOutlined style={{ fontSize: 14, color: 'rgba(255,255,255,0.6)', cursor: 'help' }} />
          </Tooltip>
        </Title>
        <Text style={{ color: 'rgba(255,255,255,0.8)', fontSize: 14 }}>
          {t('dashboard.totalSummary', { words: formatNumber(total_words), sessions: formatNumber(total_sessions) })}
        </Text>
      </div>

      {/* Stat Cards */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <AudioOutlined style={{ fontSize: 16, color: '#f5222d' }} />
              <Text type="secondary">{t('dashboard.recordings')}</Text>
              <Tooltip title={t('dashboard.recordingsTooltip')}>
                <InfoCircleOutlined style={{ fontSize: 12, color: 'rgba(128,128,128,0.45)', cursor: 'help' }} />
              </Tooltip>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>{formatNumber(total_sessions)}</Title>
            <Text type="secondary" style={{ fontSize: 12 }}>{t('dashboard.recordingsFooter')}</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <EditOutlined style={{ fontSize: 16, color: '#1890ff' }} />
              <Text type="secondary">{t('dashboard.words')}</Text>
              <Tooltip title={t('dashboard.wordsTooltip')}>
                <InfoCircleOutlined style={{ fontSize: 12, color: 'rgba(128,128,128,0.45)', cursor: 'help' }} />
              </Tooltip>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>{formatNumber(total_words)}</Title>
            <Text type="secondary" style={{ fontSize: 12 }}>{t('dashboard.wordsFooter')}</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <DashboardOutlined style={{ fontSize: 16, color: '#52c41a' }} />
              <Text type="secondary">{t('dashboard.wpm')}</Text>
              <Tooltip title={t('dashboard.wpmTooltip')}>
                <InfoCircleOutlined style={{ fontSize: 12, color: 'rgba(128,128,128,0.45)', cursor: 'help' }} />
              </Tooltip>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>
              {avg_wpm > 0 ? avg_wpm.toFixed(1) : 'N/A'}
            </Title>
            <Text type="secondary" style={{ fontSize: 12 }}>{t('dashboard.wpmFooter')}</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <FieldTimeOutlined style={{ fontSize: 16, color: '#fa8c16' }} />
              <Text type="secondary">{t('dashboard.keystrokes')}</Text>
              <Tooltip title={t('dashboard.keystrokesTooltip')}>
                <InfoCircleOutlined style={{ fontSize: 12, color: 'rgba(128,128,128,0.45)', cursor: 'help' }} />
              </Tooltip>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>{formatNumber(total_keystrokes_saved)}</Title>
            <Text type="secondary" style={{ fontSize: 12 }}>{t('dashboard.keystrokesFooter')}</Text>
          </Card>
        </Col>
      </Row>

      {/* WPM History Chart */}
      <Card
        size="small"
        style={{ borderRadius: 10 }}
        title={
          <Flex align="center" gap={8}>
            <DashboardOutlined />
            <span>{t('dashboard.wpmHistory')}</span>
          </Flex>
        }
        extra={
          <Flex align="center" gap={12}>
            <Text type="secondary" style={{ fontSize: 12 }}>{t('dashboard.records', { count: daily_wpm.length })}</Text>
            <Segmented
              size="small"
              options={timeRanges.map(r => ({ label: r.label, value: r.value }))}
              value={days}
              onChange={(v) => setDays(v as number)}
            />
          </Flex>
        }
      >
        <div style={{ width: '100%', height: 240 }}>
          <ResponsiveContainer>
            <AreaChart data={daily_wpm} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="wpmGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#fa8c16" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="#fa8c16" stopOpacity={0} />
                </linearGradient>
              </defs>
              <XAxis
                dataKey="date"
                tickFormatter={formatDate}
                tick={{ fontSize: 11 }}
                axisLine={false}
                tickLine={false}
              />
              <YAxis
                tick={{ fontSize: 11 }}
                axisLine={false}
                tickLine={false}
                width={40}
              />
              <RechartsTooltip
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                formatter={(value: any) => [`${Number(value).toFixed(1)} WPM`, t('dashboard.wpmLabel')] as any}
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                labelFormatter={(label: any) => String(label ?? '') as any}
              />
              <Area
                type="monotone"
                dataKey="wpm"
                stroke="#fa8c16"
                strokeWidth={2}
                fill="url(#wpmGradient)"
                dot={false}
                activeDot={{ r: 4 }}
              />
            </AreaChart>
          </ResponsiveContainer>
        </div>

        {/* Footer stats */}
        <Flex justify="flex-start" gap={40} style={{ marginTop: 16, paddingTop: 12, borderTop: '1px solid rgba(128,128,128,0.15)' }}>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>{t('dashboard.average')}</Text>
            <Title level={4} style={{ margin: 0 }}>{wpm_stats.avg.toFixed(1)}</Title>
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>{t('dashboard.highest')}</Text>
            <Title level={4} style={{ margin: 0 }}>{wpm_stats.max.toFixed(1)}</Title>
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>{t('dashboard.lowest')}</Text>
            <Title level={4} style={{ margin: 0 }}>{wpm_stats.min.toFixed(1)}</Title>
          </div>
        </Flex>
      </Card>
    </Flex>
  );
}
