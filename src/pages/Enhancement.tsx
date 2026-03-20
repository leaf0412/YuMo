import { useEffect, useState, useCallback } from 'react';
import {
  Card, Switch, Select, Input, Button, Flex, Space, Typography, Modal,
  Form, message, Tag, Divider,
} from 'antd';
import {
  PlusOutlined, EditOutlined, DeleteOutlined, CheckCircleOutlined,
} from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { invoke, formatError, logEvent } from '../lib/logger';

const { Title, Text, Paragraph } = Typography;
const { TextArea } = Input;

interface Prompt {
  id: string;
  name: string;
  system_message: string;
  user_message_template: string;
  is_predefined: boolean;
}

interface Settings {
  ai_enhancement_enabled?: boolean;
  llm_provider?: string;
  llm_model?: string;
  ollama_url?: string;
}

const PROVIDERS = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'ollama', label: 'Ollama' },
];

const MODEL_OPTIONS: Record<string, { value: string; label: string }[]> = {
  openai: [
    { value: 'gpt-4o', label: 'GPT-4o' },
    { value: 'gpt-4o-mini', label: 'GPT-4o Mini' },
    { value: 'gpt-4-turbo', label: 'GPT-4 Turbo' },
  ],
  anthropic: [
    { value: 'claude-sonnet-4-20250514', label: 'Claude Sonnet 4' },
    { value: 'claude-haiku-4-20250414', label: 'Claude Haiku 4' },
  ],
  ollama: [
    { value: 'llama3', label: 'Llama 3' },
    { value: 'mistral', label: 'Mistral' },
    { value: 'qwen2', label: 'Qwen 2' },
  ],
};

export default function Enhancement() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<Settings>({});
  const [prompts, setPrompts] = useState<Prompt[]>([]);
  const [selectedPromptId, setSelectedPromptId] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState('');
  const [modalOpen, setModalOpen] = useState(false);
  const [editingPrompt, setEditingPrompt] = useState<Prompt | null>(null);
  const [form] = Form.useForm();

  const loadSettings = useCallback(async () => {
    try {
      const result = await invoke<Record<string, unknown>>('get_settings');
      setSettings(result as unknown as Settings);
      const pid = result?.selected_prompt_id;
      setSelectedPromptId(typeof pid === 'string' ? pid : null);
    } catch { /* logged */ }
  }, []);

  const loadPrompts = useCallback(async () => {
    try {
      const result = await invoke<Prompt[]>('list_prompts');
      setPrompts(result);
    } catch { /* logged */ }
  }, []);

  const loadApiKey = useCallback(async () => {
    try {
      const result = await invoke<string>('get_api_key', { provider: settings.llm_provider });
      setApiKey(result ? '********' : '');
    } catch { /* logged */ }
  }, [settings.llm_provider]);

  useEffect(() => {
    loadSettings();
    loadPrompts();
  }, [loadSettings, loadPrompts]);

  useEffect(() => {
    if (settings.llm_provider) loadApiKey();
  }, [settings.llm_provider, loadApiKey]);

  const updateSetting = async (key: string, value: unknown) => {
    try {
      await invoke('update_setting', { key, value });
      setSettings((prev) => ({ ...prev, [key]: value }));
      if (key === 'llm_provider') {
        logEvent('Enhancement', 'provider_changed', { provider: value as string });
      }
    } catch (e) {
      message.error(formatError(e, t('enhancement.settingUpdateFailed')));
    }
  };

  const handleSaveApiKey = async () => {
    try {
      await invoke('store_api_key', { provider: settings.llm_provider, key: apiKey });
      logEvent('Enhancement', 'api_key_saved', { provider: settings.llm_provider });
      message.success(t('enhancement.apiKeySaved'));
    } catch (e) {
      message.error(formatError(e, t('enhancement.saveFailed')));
    }
  };

  const handleSelectPrompt = async (id: string) => {
    try {
      await invoke('select_prompt', { id });
      logEvent('Enhancement', 'prompt_selected', { id });
      message.success(t('enhancement.promptSwitched'));
      loadPrompts();
    } catch (e) {
      message.error(formatError(e, t('enhancement.switchFailed')));
    }
  };

  const handleDeletePrompt = async (id: string) => {
    try {
      await invoke('delete_prompt', { id });
      logEvent('Enhancement', 'prompt_deleted', { id });
      message.success(t('common.deleted'));
      loadPrompts();
    } catch (e) {
      message.error(formatError(e, t('enhancement.deleteFailed')));
    }
  };

  const openCreateModal = () => {
    setEditingPrompt(null);
    form.resetFields();
    setModalOpen(true);
  };

  const openEditModal = (prompt: Prompt) => {
    setEditingPrompt(prompt);
    form.setFieldsValue({
      name: prompt.name,
      systemMsg: prompt.system_message,
      userMsg: prompt.user_message_template,
    });
    setModalOpen(true);
  };

  const handleModalOk = async () => {
    try {
      const values = await form.validateFields();
      if (editingPrompt) {
        await invoke('update_prompt', {
          id: editingPrompt.id,
          name: values.name,
          systemMsg: values.systemMsg,
          userMsg: values.userMsg,
        });
        logEvent('Enhancement', 'prompt_updated', { id: editingPrompt.id });
        message.success(t('enhancement.updated'));
      } else {
        await invoke('add_prompt', {
          name: values.name,
          systemMsg: values.systemMsg,
          userMsg: values.userMsg,
        });
        logEvent('Enhancement', 'prompt_created');
        message.success(t('enhancement.created'));
      }
      setModalOpen(false);
      loadPrompts();
    } catch {
      /* validation error */
    }
  };

  const provider = settings.llm_provider || 'openai';
  const modelOptions = MODEL_OPTIONS[provider] || [];

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Title level={3}>{t('enhancement.title')}</Title>
      <Card>
        <Flex vertical gap="middle" style={{ width: '100%' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Text strong>{t('enhancement.enableToggle')}</Text>
            <Switch checked={settings.ai_enhancement_enabled} onChange={(checked) => updateSetting('ai_enhancement_enabled', checked)} />
          </div>
          <Divider style={{ margin: '8px 0' }} />
          <div>
            <Text>{t('enhancement.provider')}</Text>
            <Select value={settings.llm_provider} onChange={(v) => updateSetting('llm_provider', v)} style={{ width: '100%', marginTop: 8 }} options={PROVIDERS} placeholder={t('enhancement.selectProvider')} />
          </div>
          <div>
            <Text>{t('enhancement.model')}</Text>
            <Select value={settings.llm_model} onChange={(v) => updateSetting('llm_model', v)} style={{ width: '100%', marginTop: 8 }} options={modelOptions} placeholder={t('enhancement.selectModel')} />
          </div>
          <div>
            <Text>{t('enhancement.apiKey')}</Text>
            <Space.Compact style={{ width: '100%', marginTop: 8 }}>
              <Input.Password placeholder={t('enhancement.enterApiKey')} value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
              <Button onClick={handleSaveApiKey}>{t('common.save')}</Button>
            </Space.Compact>
          </div>
          {provider === 'ollama' && (
            <div>
              <Text>{t('enhancement.ollamaUrl')}</Text>
              <Input placeholder="http://localhost:11434" value={settings.ollama_url || ''} onChange={(e) => updateSetting('ollama_url', e.target.value)} style={{ marginTop: 8 }} />
            </div>
          )}
        </Flex>
      </Card>
      <Card title={t('enhancement.promptManagement')} extra={<Button type="primary" icon={<PlusOutlined />} onClick={openCreateModal}>{t('enhancement.newPrompt')}</Button>}>
        {prompts.length === 0 ? (
          <Text type="secondary">{t('enhancement.noPrompts')}</Text>
        ) : (
          prompts.map((prompt) => (
            <div key={prompt.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px 0', borderBottom: '1px solid #f0f0f0' }}>
              <div style={{ flex: 1 }}>
                <Space>
                  <Text>{prompt.name}</Text>
                  {selectedPromptId === prompt.id && <Tag color="green" icon={<CheckCircleOutlined />}>{t('enhancement.tagCurrent')}</Tag>}
                  {prompt.is_predefined && <Tag>{t('enhancement.tagBuiltIn')}</Tag>}
                </Space>
                <Paragraph type="secondary" ellipsis={{ rows: 2 }} style={{ marginBottom: 0, marginTop: 4 }}>{prompt.system_message}</Paragraph>
              </div>
              <Space>
                {selectedPromptId !== prompt.id && <Button type="link" onClick={() => handleSelectPrompt(prompt.id)}>{t('enhancement.use')}</Button>}
                {!prompt.is_predefined && <Button type="text" icon={<EditOutlined />} onClick={() => openEditModal(prompt)} />}
                {!prompt.is_predefined && <Button type="text" danger icon={<DeleteOutlined />} onClick={() => handleDeletePrompt(prompt.id)} />}
              </Space>
            </div>
          ))
        )}
      </Card>
      <Modal title={editingPrompt ? t('enhancement.editPrompt') : t('enhancement.newPrompt')} open={modalOpen} onOk={handleModalOk} onCancel={() => setModalOpen(false)} okText={t('common.save')} cancelText={t('common.cancel')}>
        <Form form={form} layout="vertical">
          <Form.Item name="name" label={t('enhancement.nameLabel')} rules={[{ required: true, message: t('enhancement.nameRequired') }]}><Input placeholder={t('enhancement.namePlaceholder')} /></Form.Item>
          <Form.Item name="systemMsg" label={t('enhancement.systemMessageLabel')} rules={[{ required: true, message: t('enhancement.systemMessageRequired') }]}><TextArea rows={4} placeholder={t('enhancement.systemMessagePlaceholder')} /></Form.Item>
          <Form.Item name="userMsg" label={t('enhancement.userMessageLabel')} rules={[{ required: true, message: t('enhancement.userMessageRequired') }]}><TextArea rows={4} placeholder={t('enhancement.userMessagePlaceholder', { skipInterpolation: true })} /></Form.Item>
        </Form>
      </Modal>
    </Flex>
  );
}
