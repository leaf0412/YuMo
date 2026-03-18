import { useEffect, useState, useCallback } from 'react';
import {
  Tabs, Input, Button, List, Space, Typography, message,
} from 'antd';
import {
  PlusOutlined, DeleteOutlined, UploadOutlined, DownloadOutlined,
  SwapRightOutlined,
} from '@ant-design/icons';
import { invoke } from '@tauri-apps/api/core';

const { Title, Text } = Typography;

interface VocabItem {
  id: number;
  word: string;
}

interface ReplacementItem {
  id: number;
  original: string;
  replacement: string;
}

export default function Dictionary() {
  const [vocabulary, setVocabulary] = useState<VocabItem[]>([]);
  const [replacements, setReplacements] = useState<ReplacementItem[]>([]);
  const [newWord, setNewWord] = useState('');
  const [newOriginal, setNewOriginal] = useState('');
  const [newReplacement, setNewReplacement] = useState('');

  const loadVocabulary = useCallback(async () => {
    try {
      const result = await invoke<VocabItem[]>('get_vocabulary');
      setVocabulary(result);
    } catch { /* ignore */ }
  }, []);

  const loadReplacements = useCallback(async () => {
    try {
      const result = await invoke<ReplacementItem[]>('get_replacements');
      setReplacements(result);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadVocabulary();
    loadReplacements();
  }, [loadVocabulary, loadReplacements]);

  const handleAddWord = async () => {
    if (!newWord.trim()) return;
    try {
      await invoke('add_vocabulary', { word: newWord.trim() });
      setNewWord('');
      message.success('已添加');
      loadVocabulary();
    } catch {
      message.error('添加失败');
    }
  };

  const handleDeleteWord = async (id: number) => {
    try {
      await invoke('delete_vocabulary', { id });
      message.success('已删除');
      loadVocabulary();
    } catch {
      message.error('删除失败');
    }
  };

  const handleAddReplacement = async () => {
    if (!newOriginal.trim() || !newReplacement.trim()) return;
    try {
      await invoke('set_replacement', {
        original: newOriginal.trim(),
        replacement: newReplacement.trim(),
      });
      setNewOriginal('');
      setNewReplacement('');
      message.success('已添加');
      loadReplacements();
    } catch {
      message.error('添加失败');
    }
  };

  const handleDeleteReplacement = async (id: number) => {
    try {
      await invoke('delete_replacement', { id });
      message.success('已删除');
      loadReplacements();
    } catch {
      message.error('删除失败');
    }
  };

  const handleExportCsv = async (type: 'vocabulary' | 'replacements') => {
    try {
      await invoke('export_csv', { type });
      message.success('导出成功');
    } catch {
      message.error('导出失败');
    }
  };

  const handleImportCsv = async (type: 'vocabulary' | 'replacements') => {
    try {
      await invoke('import_csv', { type });
      message.success('导入成功');
      if (type === 'vocabulary') loadVocabulary();
      else loadReplacements();
    } catch {
      message.error('导入失败');
    }
  };

  const vocabularyTab = (
    <Space direction="vertical" style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space.Compact>
          <Input
            placeholder="添加新词汇..."
            value={newWord}
            onChange={(e) => setNewWord(e.target.value)}
            onPressEnter={handleAddWord}
            style={{ width: 300 }}
          />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAddWord}>
            添加
          </Button>
        </Space.Compact>
        <Space>
          <Button icon={<UploadOutlined />} onClick={() => handleImportCsv('vocabulary')}>
            导入 CSV
          </Button>
          <Button icon={<DownloadOutlined />} onClick={() => handleExportCsv('vocabulary')}>
            导出 CSV
          </Button>
        </Space>
      </Space>

      <List
        dataSource={vocabulary}
        locale={{ emptyText: '暂无词汇' }}
        renderItem={(item) => (
          <List.Item
            actions={[
              <Button
                key="delete"
                type="text"
                danger
                icon={<DeleteOutlined />}
                onClick={() => handleDeleteWord(item.id)}
              />,
            ]}
          >
            <Text>{item.word}</Text>
          </List.Item>
        )}
      />
    </Space>
  );

  const replacementsTab = (
    <Space direction="vertical" style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space.Compact>
          <Input
            placeholder="原文..."
            value={newOriginal}
            onChange={(e) => setNewOriginal(e.target.value)}
            style={{ width: 180 }}
          />
          <Button type="text" icon={<SwapRightOutlined />} disabled />
          <Input
            placeholder="替换为..."
            value={newReplacement}
            onChange={(e) => setNewReplacement(e.target.value)}
            onPressEnter={handleAddReplacement}
            style={{ width: 180 }}
          />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAddReplacement}>
            添加
          </Button>
        </Space.Compact>
        <Space>
          <Button icon={<UploadOutlined />} onClick={() => handleImportCsv('replacements')}>
            导入 CSV
          </Button>
          <Button icon={<DownloadOutlined />} onClick={() => handleExportCsv('replacements')}>
            导出 CSV
          </Button>
        </Space>
      </Space>

      <List
        dataSource={replacements}
        locale={{ emptyText: '暂无替换规则' }}
        renderItem={(item) => (
          <List.Item
            actions={[
              <Button
                key="delete"
                type="text"
                danger
                icon={<DeleteOutlined />}
                onClick={() => handleDeleteReplacement(item.id)}
              />,
            ]}
          >
            <Space>
              <Text>{item.original}</Text>
              <SwapRightOutlined />
              <Text strong>{item.replacement}</Text>
            </Space>
          </List.Item>
        )}
      />
    </Space>
  );

  return (
    <Space direction="vertical" size="large" style={{ width: '100%' }}>
      <Title level={3}>词典</Title>
      <Tabs
        items={[
          { key: 'vocabulary', label: '词汇表', children: vocabularyTab },
          { key: 'replacements', label: '替换规则', children: replacementsTab },
        ]}
      />
    </Space>
  );
}
