import { useEffect, useState, useCallback } from 'react';
import { Card, Flex, Typography, Row, Col, Segmented, Spin, Empty } from 'antd';
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
  Tooltip,
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

const TIME_RANGES = [
  { label: '7 天', value: 7 },
  { label: '30 天', value: 30 },
  { label: '90 天', value: 90 },
  { label: '全部', value: 0 },
];

function formatTimeSaved(minutes: number): string {
  if (minutes < 1) return '不到 1 分钟';
  if (minutes < 60) return `${Math.round(minutes)} 分钟`;
  const hours = Math.floor(minutes / 60);
  const mins = Math.round(minutes % 60);
  return mins > 0 ? `${hours} 小时 ${mins} 分钟` : `${hours} 小时`;
}

function formatDate(dateStr: string): string {
  const [, m, d] = dateStr.split('-');
  return `${Number(m)}/${Number(d)}`;
}

function formatNumber(n: number): string {
  return n.toLocaleString();
}

export default function Dashboard() {
  const [stats, setStats] = useState<Statistics | null>(null);
  const [loading, setLoading] = useState(true);
  const [days, setDays] = useState<number>(30);

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

  if (loading && !stats) {
    return <Flex justify="center" align="center" style={{ height: 300 }}><Spin size="large" /></Flex>;
  }

  if (!stats || stats.total_sessions === 0) {
    return (
      <Flex vertical align="center" justify="center" gap="middle" style={{ height: 400 }}>
        <Empty description="暂无录音数据" />
        <Text type="secondary">开始你的第一次语音录入吧</Text>
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
          你已节省 <span style={{ fontWeight: 800 }}>{formatTimeSaved(time_saved_minutes)}</span> 使用 VoiceInk
        </Title>
        <Text style={{ color: 'rgba(255,255,255,0.8)', fontSize: 14 }}>
          共转录 {formatNumber(total_words)} 个字，完成 {formatNumber(total_sessions)} 次录音。
        </Text>
      </div>

      {/* Stat Cards */}
      <Row gutter={[16, 16]}>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <AudioOutlined style={{ fontSize: 16, color: '#f5222d' }} />
              <Text type="secondary">录音次数</Text>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>{formatNumber(total_sessions)}</Title>
            <Text type="secondary" style={{ fontSize: 12 }}>VoiceInk 录音完成</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <EditOutlined style={{ fontSize: 16, color: '#1890ff' }} />
              <Text type="secondary">转录字数</Text>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>{formatNumber(total_words)}</Title>
            <Text type="secondary" style={{ fontSize: 12 }}>已生成字数</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <DashboardOutlined style={{ fontSize: 16, color: '#52c41a' }} />
              <Text type="secondary">每分钟字数</Text>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>
              {avg_wpm > 0 ? avg_wpm.toFixed(1) : 'N/A'}
            </Title>
            <Text type="secondary" style={{ fontSize: 12 }}>语音输入 vs 手动打字</Text>
          </Card>
        </Col>
        <Col xs={24} sm={12}>
          <Card size="small" style={{ borderRadius: 10 }}>
            <Flex align="center" gap={8} style={{ marginBottom: 8 }}>
              <FieldTimeOutlined style={{ fontSize: 16, color: '#fa8c16' }} />
              <Text type="secondary">节省按键</Text>
            </Flex>
            <Title level={2} style={{ margin: 0 }}>{formatNumber(total_keystrokes_saved)}</Title>
            <Text type="secondary" style={{ fontSize: 12 }}>减少的按键次数</Text>
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
            <span>WPM 历史</span>
          </Flex>
        }
        extra={
          <Flex align="center" gap={12}>
            <Text type="secondary" style={{ fontSize: 12 }}>{daily_wpm.length} 条记录</Text>
            <Segmented
              size="small"
              options={TIME_RANGES.map(r => ({ label: r.label, value: r.value }))}
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
              <Tooltip
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                formatter={(value: any) => [`${Number(value).toFixed(1)} WPM`, '每分钟字数'] as any}
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
            <Text type="secondary" style={{ fontSize: 11 }}>平均</Text>
            <Title level={4} style={{ margin: 0 }}>{wpm_stats.avg.toFixed(1)}</Title>
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>最高</Text>
            <Title level={4} style={{ margin: 0 }}>{wpm_stats.max.toFixed(1)}</Title>
          </div>
          <div>
            <Text type="secondary" style={{ fontSize: 11 }}>最低</Text>
            <Title level={4} style={{ margin: 0 }}>{wpm_stats.min.toFixed(1)}</Title>
          </div>
        </Flex>
      </Card>
    </Flex>
  );
}
