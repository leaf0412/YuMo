import { useEffect, useState, useCallback } from 'react';
import {
  Card, Switch, Select, Input, Button, Flex, Space, Typography, Modal,
  Form, message, Tag, Divider,
} from 'antd';
import {
  PlusOutlined, EditOutlined, DeleteOutlined, CheckCircleOutlined,
} from '@ant-design/icons';
import { invoke, formatError } from '../lib/logger';

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
    } catch (e) {
      message.error(formatError(e, '设置更新失败'));
    }
  };

  const handleSaveApiKey = async () => {
    try {
      await invoke('store_api_key', { provider: settings.llm_provider, key: apiKey });
      message.success('API Key 已保存');
    } catch (e) {
      message.error(formatError(e, '保存失败'));
    }
  };

  const handleSelectPrompt = async (id: string) => {
    try {
      await invoke('select_prompt', { id });
      message.success('已切换 Prompt');
      loadPrompts();
    } catch (e) {
      message.error(formatError(e, '切换失败'));
    }
  };

  const handleDeletePrompt = async (id: string) => {
    try {
      await invoke('delete_prompt', { id });
      message.success('已删除');
      loadPrompts();
    } catch (e) {
      message.error(formatError(e, '删除失败'));
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
        message.success('已更新');
      } else {
        await invoke('add_prompt', {
          name: values.name,
          systemMsg: values.systemMsg,
          userMsg: values.userMsg,
        });
        message.success('已创建');
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
      <Title level={3}>AI 增强</Title>
      <Card>
        <Flex vertical gap="middle" style={{ width: '100%' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Text strong>启用 AI 增强</Text>
            <Switch checked={settings.ai_enhancement_enabled} onChange={(checked) => updateSetting('ai_enhancement_enabled', checked)} />
          </div>
          <Divider style={{ margin: '8px 0' }} />
          <div>
            <Text>LLM 服务商</Text>
            <Select value={settings.llm_provider} onChange={(v) => updateSetting('llm_provider', v)} style={{ width: '100%', marginTop: 8 }} options={PROVIDERS} placeholder="选择服务商" />
          </div>
          <div>
            <Text>模型</Text>
            <Select value={settings.llm_model} onChange={(v) => updateSetting('llm_model', v)} style={{ width: '100%', marginTop: 8 }} options={modelOptions} placeholder="选择模型" />
          </div>
          <div>
            <Text>API Key</Text>
            <Space.Compact style={{ width: '100%', marginTop: 8 }}>
              <Input.Password placeholder="输入 API Key" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
              <Button onClick={handleSaveApiKey}>保存</Button>
            </Space.Compact>
          </div>
          {provider === 'ollama' && (
            <div>
              <Text>Ollama URL</Text>
              <Input placeholder="http://localhost:11434" value={settings.ollama_url || ''} onChange={(e) => updateSetting('ollama_url', e.target.value)} style={{ marginTop: 8 }} />
            </div>
          )}
        </Flex>
      </Card>
      <Card title="Prompt 管理" extra={<Button type="primary" icon={<PlusOutlined />} onClick={openCreateModal}>新建 Prompt</Button>}>
        {prompts.length === 0 ? (
          <Text type="secondary">暂无 Prompt</Text>
        ) : (
          prompts.map((prompt) => (
            <div key={prompt.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '12px 0', borderBottom: '1px solid #f0f0f0' }}>
              <div style={{ flex: 1 }}>
                <Space>
                  <Text>{prompt.name}</Text>
                  {selectedPromptId === prompt.id && <Tag color="green" icon={<CheckCircleOutlined />}>当前</Tag>}
                  {prompt.is_predefined && <Tag>内置</Tag>}
                </Space>
                <Paragraph type="secondary" ellipsis={{ rows: 2 }} style={{ marginBottom: 0, marginTop: 4 }}>{prompt.system_message}</Paragraph>
              </div>
              <Space>
                {selectedPromptId !== prompt.id && <Button type="link" onClick={() => handleSelectPrompt(prompt.id)}>使用</Button>}
                {!prompt.is_predefined && <Button type="text" icon={<EditOutlined />} onClick={() => openEditModal(prompt)} />}
                {!prompt.is_predefined && <Button type="text" danger icon={<DeleteOutlined />} onClick={() => handleDeletePrompt(prompt.id)} />}
              </Space>
            </div>
          ))
        )}
      </Card>
      <Modal title={editingPrompt ? '编辑 Prompt' : '新建 Prompt'} open={modalOpen} onOk={handleModalOk} onCancel={() => setModalOpen(false)} okText="保存" cancelText="取消">
        <Form form={form} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}><Input placeholder="Prompt 名称" /></Form.Item>
          <Form.Item name="systemMsg" label="系统消息" rules={[{ required: true, message: '请输入系统消息' }]}><TextArea rows={4} placeholder="系统消息..." /></Form.Item>
          <Form.Item name="userMsg" label="用户消息模板" rules={[{ required: true, message: '请输入用户消息模板' }]}><TextArea rows={4} placeholder="使用 {{text}} 作为转录文本占位符" /></Form.Item>
        </Form>
      </Modal>
    </Flex>
  );
}
