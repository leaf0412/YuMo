import { useEffect, useState, useCallback } from 'react';
import { Input, List, Button, Space, Tag, Typography, Popconfirm, message, Card } from 'antd';
import { CopyOutlined, DeleteOutlined, ClearOutlined } from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';

const { Text, Paragraph } = Typography;

interface Transcription {
  id: number;
  text: string;
  enhanced_text?: string;
  created_at: string;
  model_name: string;
}

const PAGE_SIZE = 20;

export default function History() {
  const [transcriptions, setTranscriptions] = useState<Transcription[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());

  const loadTranscriptions = useCallback(async (offset: number, reset: boolean) => {
    setLoading(true);
    try {
      const result = await invoke<Transcription[]>('get_transcriptions', {
        limit: PAGE_SIZE,
        offset,
        search: searchQuery || undefined,
      });
      if (reset) {
        setTranscriptions(result);
      } else {
        setTranscriptions((prev) => [...prev, ...result]);
      }
      setHasMore(result.length === PAGE_SIZE);
    } catch {
      /* ignore */
    } finally {
      setLoading(false);
    }
  }, [searchQuery]);

  useEffect(() => {
    loadTranscriptions(0, true);
  }, [loadTranscriptions]);

  const handleSearch = (value: string) => {
    setSearchQuery(value);
  };

  const handleLoadMore = () => {
    loadTranscriptions(transcriptions.length, false);
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    message.success('已复制');
  };

  const handleDelete = async (id: number) => {
    try {
      await invoke('delete_transcription', { id });
      setTranscriptions((prev) => prev.filter((t) => t.id !== id));
      message.success('已删除');
    } catch {
      message.error('删除失败');
    }
  };

  const handleClearAll = async () => {
    try {
      await invoke('delete_all_transcriptions');
      setTranscriptions([]);
      message.success('已清空');
    } catch {
      message.error('清空失败');
    }
  };

  const toggleExpand = (id: number) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const wordCount = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

  return (
    <Space direction="vertical" size="middle" style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Input.Search
          placeholder="搜索转录内容..."
          onSearch={handleSearch}
          allowClear
          style={{ width: 400 }}
        />
        <Popconfirm
          title="确认清空所有转录记录？"
          onConfirm={handleClearAll}
          okText="确认"
          cancelText="取消"
        >
          <Button danger icon={<ClearOutlined />}>
            清空全部
          </Button>
        </Popconfirm>
      </Space>

      <List
        loading={loading}
        dataSource={transcriptions}
        locale={{ emptyText: '暂无转录记录' }}
        renderItem={(item) => {
          const expanded = expandedIds.has(item.id);
          const preview = item.text.length > 120 ? `${item.text.slice(0, 120)}...` : item.text;

          return (
            <List.Item
              actions={[
                <Button
                  key="copy"
                  type="text"
                  icon={<CopyOutlined />}
                  onClick={() => handleCopy(item.enhanced_text || item.text)}
                />,
                <Popconfirm
                  key="delete"
                  title="确认删除？"
                  onConfirm={() => handleDelete(item.id)}
                  okText="确认"
                  cancelText="取消"
                >
                  <Button type="text" danger icon={<DeleteOutlined />} />
                </Popconfirm>,
              ]}
            >
              <List.Item.Meta
                title={
                  <Space>
                    <Text type="secondary">{item.created_at}</Text>
                    <Tag>{item.model_name}</Tag>
                    <Tag color="blue">{wordCount(item.text)} 词</Tag>
                  </Space>
                }
                description={
                  <div>
                    <Paragraph
                      style={{ cursor: 'pointer', marginBottom: 4 }}
                      onClick={() => toggleExpand(item.id)}
                    >
                      {expanded ? item.text : preview}
                    </Paragraph>
                    {expanded && item.enhanced_text && (
                      <Card size="small" title="AI 增强文本" style={{ marginTop: 8 }}>
                        <Paragraph>{item.enhanced_text}</Paragraph>
                      </Card>
                    )}
                  </div>
                }
              />
            </List.Item>
          );
        }}
      />

      {hasMore && transcriptions.length > 0 && (
        <div style={{ textAlign: 'center' }}>
          <Button onClick={handleLoadMore} loading={loading}>
            加载更多
          </Button>
        </div>
      )}
    </Space>
  );
}
