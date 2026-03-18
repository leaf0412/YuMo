import { useEffect, useState, useCallback } from 'react';
import { Input, Button, Flex, Space, Tag, Typography, Popconfirm, message, Card } from 'antd';
import { CopyOutlined, DeleteOutlined, ClearOutlined } from '@ant-design/icons';
import { invoke, formatError } from '../lib/logger';
const { Text, Paragraph } = Typography;

interface Transcription {
  id: string;
  text: string;
  enhanced_text?: string;
  timestamp: string;
  model_name: string;
}

const PAGE_SIZE = 20;

export default function History() {
  const [transcriptions, setTranscriptions] = useState<Transcription[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const loadTranscriptions = useCallback(async (cursor: string | null, reset: boolean) => {
    setLoading(true);
    
    try {
      const result = await invoke<{ items: Transcription[], next_cursor: string | null }>('get_transcriptions', {
        limit: PAGE_SIZE,
        cursor: cursor ?? undefined,
        query: searchQuery || undefined,
      });
      const items = result.items || [];
      if (reset) {
        setTranscriptions(items);
      } else {
        setTranscriptions((prev) => [...prev, ...items]);
      }
      setNextCursor(result.next_cursor);
      setHasMore(result.next_cursor !== null);
    } catch {
      /* logged by invoke */
    } finally {
      setLoading(false);
    }
  }, [searchQuery]);

  useEffect(() => {
    loadTranscriptions(null, true);
  }, [loadTranscriptions]);

  const handleSearch = (value: string) => {
    
    setSearchQuery(value);
  };

  const handleLoadMore = () => {
    
    loadTranscriptions(nextCursor, false);
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    
    message.success('已复制');
  };

  const handleDelete = async (id: string) => {
    
    try {
      await invoke('delete_transcription', { id });
      setTranscriptions((prev) => prev.filter((t) => t.id !== id));
      message.success('已删除');
    } catch (e) {
      message.error(formatError(e, '删除失败'));
    }
  };

  const handleClearAll = async () => {
    
    try {
      await invoke('delete_all_transcriptions');
      setTranscriptions([]);
      message.success('已清空');
    } catch (e) {
      message.error(formatError(e, '清空失败'));
    }
  };

  const toggleExpand = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const wordCount = (text: string) => text.trim().split(/\s+/).filter(Boolean).length;

  return (
    <Flex vertical gap="middle" style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Input.Search
          placeholder="搜索转录内容..."
          onSearch={handleSearch}
          allowClear
          style={{ width: 400 }}
        />
        <Popconfirm title="确认清空所有转录记录？" onConfirm={handleClearAll} okText="确认" cancelText="取消">
          <Button danger icon={<ClearOutlined />}>清空全部</Button>
        </Popconfirm>
      </Space>

      {transcriptions.length === 0 && !loading ? (
        <Text type="secondary">暂无转录记录</Text>
      ) : (
        transcriptions.map((item) => {
          const expanded = expandedIds.has(item.id);
          const preview = item.text.length > 120 ? `${item.text.slice(0, 120)}...` : item.text;
          return (
            <Card key={item.id} size="small">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <div style={{ flex: 1 }}>
                  <Space>
                    <Text type="secondary">{item.timestamp}</Text>
                    <Tag>{item.model_name}</Tag>
                    <Tag color="blue">{wordCount(item.text)} 词</Tag>
                  </Space>
                  <Paragraph style={{ cursor: 'pointer', marginBottom: 4, marginTop: 8 }} onClick={() => toggleExpand(item.id)}>
                    {expanded ? item.text : preview}
                  </Paragraph>
                  {expanded && item.enhanced_text && (
                    <Card size="small" title="AI 增强文本" style={{ marginTop: 8 }}>
                      <Paragraph>{item.enhanced_text}</Paragraph>
                    </Card>
                  )}
                </div>
                <Space>
                  <Button type="text" icon={<CopyOutlined />} onClick={() => handleCopy(item.enhanced_text || item.text)} />
                  <Popconfirm title="确认删除？" onConfirm={() => handleDelete(item.id)} okText="确认" cancelText="取消">
                    <Button type="text" danger icon={<DeleteOutlined />} />
                  </Popconfirm>
                </Space>
              </div>
            </Card>
          );
        })
      )}

      {hasMore && transcriptions.length > 0 && (
        <div style={{ textAlign: 'center' }}>
          <Button onClick={handleLoadMore} loading={loading}>加载更多</Button>
        </div>
      )}
    </Flex>
  );
}
