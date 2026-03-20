import { useEffect, useState, useCallback, useRef } from 'react';
import { Input, Button, Flex, Space, Tag, Typography, Popconfirm, message, Card } from 'antd';
import { CopyOutlined, DeleteOutlined, ClearOutlined, PlayCircleOutlined, PauseCircleOutlined } from '@ant-design/icons';
import { emit } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { invoke, formatError, logEvent } from '../lib/logger';
const { Text, Paragraph } = Typography;

interface Transcription {
  id: string;
  text: string;
  enhanced_text?: string;
  timestamp: string;
  model_name: string;
  recording_path?: string;
}

const PAGE_SIZE = 20;

export default function History() {
  const { t } = useTranslation();
  const [transcriptions, setTranscriptions] = useState<Transcription[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());
  const [playingId, setPlayingId] = useState<string | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const handlePlay = async (item: Transcription) => {
    if (!item.recording_path) return;

    // If same item is playing, stop it
    if (playingId === item.id) {
      audioRef.current?.pause();
      audioRef.current = null;
      setPlayingId(null);
      return;
    }

    // Stop any current playback
    audioRef.current?.pause();

    try {
      const dataUri = await invoke<string>('get_recording', { recordingPath: item.recording_path });
      const audio = new Audio(dataUri);
      audio.onended = () => { setPlayingId(null); audioRef.current = null; };
      audio.onerror = () => { setPlayingId(null); audioRef.current = null; message.error(t('history.playbackFailed')); };
      audioRef.current = audio;
      setPlayingId(item.id);
      logEvent('History', 'playback_start');
      audio.play();
    } catch (e) {
      message.error(formatError(e, t('history.cannotLoadRecording')));
    }
  };

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
    logEvent('History', 'search', { query: value });
    setSearchQuery(value);
  };

  const handleLoadMore = () => {
    
    loadTranscriptions(nextCursor, false);
  };

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    logEvent('History', 'copy_text');
    message.success(t('common.copied'));
  };

  const handleDelete = async (id: string) => {
    try {
      await invoke('delete_transcription', { id });
      logEvent('History', 'delete');
      setTranscriptions((prev) => prev.filter((t) => t.id !== id));
      emit('stats-updated');
      message.success(t('common.deleted'));
    } catch (e) {
      message.error(formatError(e, t('history.deleteFailed')));
    }
  };

  const handleClearAll = async () => {
    
    try {
      await invoke('delete_all_transcriptions');
      setTranscriptions([]);
      emit('stats-updated');
      message.success(t('common.cleared'));
    } catch (e) {
      message.error(formatError(e, t('history.clearFailed')));
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

  const wordCount = (text: string) => {
    let count = 0;
    let inWord = false;
    for (const c of text) {
      const code = c.codePointAt(0) ?? 0;
      const isCjk =
        (code >= 0x4e00 && code <= 0x9fff) ||
        (code >= 0x3400 && code <= 0x4dbf) ||
        (code >= 0xf900 && code <= 0xfaff) ||
        (code >= 0x2f800 && code <= 0x2fa1f);
      if (isCjk) {
        count++;
        inWord = false;
      } else if (/\s/.test(c)) {
        inWord = false;
      } else if (!inWord) {
        count++;
        inWord = true;
      }
    }
    return count;
  };

  return (
    <Flex vertical gap="middle" style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Input.Search
          placeholder={t('history.searchPlaceholder')}
          onSearch={handleSearch}
          allowClear
          style={{ width: 400 }}
        />
        <Popconfirm title={t('history.confirmClearAll')} onConfirm={handleClearAll} okText={t('common.confirm')} cancelText={t('common.cancel')}>
          <Button danger icon={<ClearOutlined />}>{t('history.clearAll')}</Button>
        </Popconfirm>
      </Space>

      {transcriptions.length === 0 && !loading ? (
        <Text type="secondary">{t('history.noRecords')}</Text>
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
                    <Tag color="blue">{t('history.wordCount', { count: wordCount(item.text) })}</Tag>
                  </Space>
                  <Paragraph style={{ cursor: 'pointer', marginBottom: 4, marginTop: 8 }} onClick={() => toggleExpand(item.id)}>
                    {expanded ? item.text : preview}
                  </Paragraph>
                  {expanded && item.enhanced_text && (
                    <Card size="small" title={t('history.aiEnhanced')} style={{ marginTop: 8 }}>
                      <Paragraph>{item.enhanced_text}</Paragraph>
                    </Card>
                  )}
                </div>
                <Space>
                  {item.recording_path && (
                    <Button
                      type="text"
                      icon={playingId === item.id ? <PauseCircleOutlined /> : <PlayCircleOutlined />}
                      onClick={() => handlePlay(item)}
                      title={playingId === item.id ? t('history.stop') : t('history.play')}
                    />
                  )}
                  <Button type="text" icon={<CopyOutlined />} onClick={() => handleCopy(item.enhanced_text || item.text)} />
                  <Popconfirm title={t('history.confirmDelete')} onConfirm={() => handleDelete(item.id)} okText={t('common.confirm')} cancelText={t('common.cancel')}>
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
          <Button onClick={handleLoadMore} loading={loading}>{t('common.loadMore')}</Button>
        </div>
      )}
    </Flex>
  );
}
