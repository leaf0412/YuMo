import { useEffect, useState, useCallback } from 'react';
import {
  Tabs, Input, Button, Flex, Space, Typography, message,
} from 'antd';
import {
  PlusOutlined, DeleteOutlined, UploadOutlined, DownloadOutlined,
  SwapRightOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
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
  const { t } = useTranslation();
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
      message.success(t('common.added'));
      loadVocabulary();
    } catch (e) {
      message.error(formatError(e, t('dictionary.addFailed')));
    }
  };

  const handleDeleteWord = async (id: string) => {
    try {
      await invoke('delete_vocabulary', { id });
      message.success(t('common.deleted'));
      loadVocabulary();
    } catch (e) {
      message.error(formatError(e, t('dictionary.deleteFailed')));
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
      message.success(t('common.added'));
      loadReplacements();
    } catch (e) {
      message.error(formatError(e, t('dictionary.addFailed')));
    }
  };

  const handleDeleteReplacement = async (id: string) => {
    try {
      await invoke('delete_replacement', { id });
      message.success(t('common.deleted'));
      loadReplacements();
    } catch (e) {
      message.error(formatError(e, t('dictionary.deleteFailed')));
    }
  };

  const handleExportCsv = (_type: 'vocabulary' | 'replacements') => {
    message.info(t('dictionary.csvExportNotImpl'));
  };

  const handleImportCsv = (_type: 'vocabulary' | 'replacements') => {
    message.info(t('dictionary.csvImportNotImpl'));
  };

  const vocabularyTab = (
    <Flex vertical gap={8} style={{ width: '100%' }}>
      <Space style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space.Compact>
          <Input placeholder={t('dictionary.addWordPlaceholder')} value={newWord} onChange={(e) => setNewWord(e.target.value)} onPressEnter={handleAddWord} style={{ width: 300 }} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAddWord}>{t('dictionary.add')}</Button>
        </Space.Compact>
        <Space>
          <Button icon={<UploadOutlined />} onClick={() => handleImportCsv('vocabulary')}>{t('dictionary.importCsv')}</Button>
          <Button icon={<DownloadOutlined />} onClick={() => handleExportCsv('vocabulary')}>{t('dictionary.exportCsv')}</Button>
        </Space>
      </Space>
      {vocabulary.length === 0 ? (
        <Text type="secondary">{t('dictionary.emptyVocabulary')}</Text>
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
          <Input placeholder={t('dictionary.originalPlaceholder')} value={newOriginal} onChange={(e) => setNewOriginal(e.target.value)} style={{ width: 180 }} />
          <Button type="text" icon={<SwapRightOutlined />} disabled />
          <Input placeholder={t('dictionary.replacementPlaceholder')} value={newReplacement} onChange={(e) => setNewReplacement(e.target.value)} onPressEnter={handleAddReplacement} style={{ width: 180 }} />
          <Button type="primary" icon={<PlusOutlined />} onClick={handleAddReplacement}>{t('dictionary.add')}</Button>
        </Space.Compact>
        <Space>
          <Button icon={<UploadOutlined />} onClick={() => handleImportCsv('replacements')}>{t('dictionary.importCsv')}</Button>
          <Button icon={<DownloadOutlined />} onClick={() => handleExportCsv('replacements')}>{t('dictionary.exportCsv')}</Button>
        </Space>
      </Space>
      {replacements.length === 0 ? (
        <Text type="secondary">{t('dictionary.emptyReplacements')}</Text>
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
      <Title level={3}>{t('dictionary.title')}</Title>
      <Tabs
        items={[
          { key: 'vocabulary', label: t('dictionary.vocabularyTab'), children: vocabularyTab },
          { key: 'replacements', label: t('dictionary.replacementsTab'), children: replacementsTab },
        ]}
      />
    </Flex>
  );
}
