import { useEffect, useState, useCallback } from 'react';
import {
  Tabs, Input, Button, Flex, Space, Typography, message,
} from 'antd';
import {
  PlusOutlined, DeleteOutlined, UploadOutlined, DownloadOutlined,
  SwapRightOutlined,
} from '@ant-design/icons';
import { invoke, formatError } from '../lib/logger';

const { Title, Text } = Typography;

interface VocabItem {
  id: string;
  word: string;
}

interface ReplacementItem {
  id: string;
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
    } catch { /* logged */ }
  }, []);

  const loadReplacements = useCallback(async () => {
    try {
      const result = await invoke<ReplacementItem[]>('get_replacements');
      setReplacements(result);
    } catch { /* logged */ }
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
    } catch (e) {
      message.error(formatError(e, '添加失败'));
    }
  };

  const handleDeleteWord = async (id: string) => {
    try {
      await invoke('delete_vocabulary', { id });
      message.success('已删除');
      loadVocabulary();
    } catch (e) {
      message.error(formatError(e, '删除失败'));
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
    } catch (e) {
      message.error(formatError(e, '添加失败'));
    }
  };

  const handleDeleteReplacement = async (id: string) => {
    try {
      await invoke('delete_replacement', { id });
      message.success('已删除');
      loadReplacements();
    } catch (e) {
      message.error(formatError(e, '删除失败'));
    }
  };

  const handleExportCsv = (_type: 'vocabulary' | 'replacements') => {
    message.info('CSV 导出暂未实现');
  };

  const handleImportCsv = (_type: 'vocabulary' | 'replacements') => {
    message.info('CSV 导入暂未实现');
  };

  const vocabularyTab = (
    <Flex vertical gap={8} style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space.Compact>
          <Input placeholder="添加新词汇..." value={newWord} onChange={(e) => setNewWord(e.target.value)} onPressEnter={handleAddWord} style={{ width: 300 }} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAddWord}>添加</Button>
        </Space.Compact>
        <Space>
          <Button icon={<UploadOutlined />} onClick={() => handleImportCsv('vocabulary')}>导入 CSV</Button>
          <Button icon={<DownloadOutlined />} onClick={() => handleExportCsv('vocabulary')}>导出 CSV</Button>
        </Space>
      </Space>
      {vocabulary.length === 0 ? (
        <Text type="secondary">暂无词汇</Text>
      ) : (
        vocabulary.map((item) => (
          <div key={item.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0', borderBottom: '1px solid #f0f0f0' }}>
            <Text>{item.word}</Text>
            <Button type="text" danger icon={<DeleteOutlined />} onClick={() => handleDeleteWord(item.id)} />
          </div>
        ))
      )}
    </Flex>
  );

  const replacementsTab = (
    <Flex vertical gap={8} style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space.Compact>
          <Input placeholder="原文..." value={newOriginal} onChange={(e) => setNewOriginal(e.target.value)} style={{ width: 180 }} />
          <Button type="text" icon={<SwapRightOutlined />} disabled />
          <Input placeholder="替换为..." value={newReplacement} onChange={(e) => setNewReplacement(e.target.value)} onPressEnter={handleAddReplacement} style={{ width: 180 }} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAddReplacement}>添加</Button>
        </Space.Compact>
        <Space>
          <Button icon={<UploadOutlined />} onClick={() => handleImportCsv('replacements')}>导入 CSV</Button>
          <Button icon={<DownloadOutlined />} onClick={() => handleExportCsv('replacements')}>导出 CSV</Button>
        </Space>
      </Space>
      {replacements.length === 0 ? (
        <Text type="secondary">暂无替换规则</Text>
      ) : (
        replacements.map((item) => (
          <div key={item.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 0', borderBottom: '1px solid #f0f0f0' }}>
            <Space>
              <Text>{item.original}</Text>
              <SwapRightOutlined />
              <Text strong>{item.replacement}</Text>
            </Space>
            <Button type="text" danger icon={<DeleteOutlined />} onClick={() => handleDeleteReplacement(item.id)} />
          </div>
        ))
      )}
    </Flex>
  );

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Title level={3}>词典</Title>
      <Tabs
        items={[
          { key: 'vocabulary', label: '词汇表', children: vocabularyTab },
          { key: 'replacements', label: '替换规则', children: replacementsTab },
        ]}
      />
    </Flex>
  );
}
