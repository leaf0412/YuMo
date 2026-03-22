import { useEffect, useState, useCallback } from 'react';
import { Flex, Space, Button, Slider, Typography, message } from 'antd';
import { FolderOpenOutlined, FileZipOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { invoke, formatError, logEvent } from '../lib/logger';
import type { SpriteManifest } from '../components/SpriteAnimation';
import SpriteCard from '../components/SpriteCard';
import useAppStore from '../stores/useAppStore';

const { Text } = Typography;

type SpriteEntry = SpriteManifest & { dirId: string };

export default function Sprites() {
  const { t } = useTranslation();
  const { updateSetting } = useAppStore();
  const [sprites, setSprites] = useState<SpriteEntry[]>([]);
  const [spriteSrcs, setSpriteSrcs] = useState<Record<string, string>>({});
  const [selectedSpriteId, setSelectedSpriteId] = useState('');
  const [spriteSize, setSpriteSize] = useState(180);
  const [bgThreshold, setBgThreshold] = useState(0.18);
  const [bgProcessing, setBgProcessing] = useState(false);

  const loadSprites = useCallback(async () => {
    try {
      const list = await invoke<SpriteEntry[]>('list_sprites');
      setSprites(list);
      const srcs: Record<string, string> = {};
      await Promise.all(
        list.map(async (s) => {
          try {
            srcs[s.dirId] = await invoke<string>('get_sprite_image', { dirId: s.dirId, fileName: 'sprite_processed.png' });
          } catch {
            try {
              srcs[s.dirId] = await invoke<string>('get_sprite_image', { dirId: s.dirId, fileName: s.spriteFile });
            } catch { /* skip */ }
          }
        })
      );
      setSpriteSrcs(srcs);
    } catch (e) {
      message.error(formatError(e, t('settings.spriteLoadFailed')));
    }
  }, [t]);

  const loadSpriteSettings = useCallback(async () => {
    try {
      const s = await invoke<{ selected_sprite_id?: string; sprite_size?: number }>('get_settings');
      if (s.selected_sprite_id) setSelectedSpriteId(s.selected_sprite_id);
      if (s.sprite_size) setSpriteSize(s.sprite_size);
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    loadSprites();
    loadSpriteSettings();
  }, [loadSprites, loadSpriteSettings]);

  const handleSpriteImportFolder = async () => {
    try {
      const result = await invoke<SpriteEntry | null>('import_sprite_folder');
      if (result) {
        logEvent('Sprite', 'import_folder', { name: result.name });
        message.success(t('settings.spriteImportSuccess', { name: result.name }));
        await loadSprites();
      }
    } catch (e) {
      message.error(formatError(e, t('settings.spriteImportFailed')));
    }
  };

  const handleSpriteImportZip = async () => {
    try {
      const result = await invoke<SpriteEntry | null>('import_sprite_zip');
      if (result) {
        logEvent('Sprite', 'import_zip', { name: result.name });
        message.success(t('settings.spriteImportSuccess', { name: result.name }));
        await loadSprites();
      }
    } catch (e) {
      message.error(formatError(e, t('settings.spriteImportFailed')));
    }
  };

  const handleSpriteDelete = async (dirId: string) => {
    try {
      await invoke('delete_sprite', { dirId });
      logEvent('Sprite', 'delete', { dirId });
      message.success(t('settings.spriteDeleteSuccess'));
      if (selectedSpriteId === dirId) {
        setSelectedSpriteId('');
        updateSetting('selected_sprite_id', '');
      }
      await loadSprites();
    } catch (e) {
      message.error(formatError(e, t('settings.spriteImportFailed')));
    }
  };

  const handleSpriteSelect = (dirId: string) => {
    const newValue = dirId === selectedSpriteId ? '' : dirId;
    setSelectedSpriteId(newValue);
    updateSetting('selected_sprite_id', newValue);
  };

  const handleSpriteSizeChange = (size: number) => {
    setSpriteSize(size);
    updateSetting('sprite_size', String(size));
  };

  const handleBgThresholdCommit = async (value: number) => {
    setBgThreshold(value);
    if (!selectedSpriteId) return;
    setBgProcessing(true);
    try {
      await invoke('process_sprite_background', { dirId: selectedSpriteId, threshold: value });
      logEvent('Sprite', 'process_background', { dirId: selectedSpriteId, threshold: value });
      await loadSprites();
    } catch (e) {
      message.error(formatError(e, t('settings.spriteProcessFailed')));
    }
    setBgProcessing(false);
  };

  return (
    <Flex vertical gap="large" style={{ width: '100%' }}>
      <Typography.Title level={3}>{t('sprites.title')}</Typography.Title>
      <Flex vertical gap={12} style={{ width: '100%' }}>
        <Space>
          <Button icon={<FolderOpenOutlined />} onClick={handleSpriteImportFolder}>
            {t('settings.spriteImportFolder')}
          </Button>
          <Button icon={<FileZipOutlined />} onClick={handleSpriteImportZip}>
            {t('settings.spriteImportZip')}
          </Button>
        </Space>

        {sprites.length === 0 ? (
          <Text type="secondary">{t('settings.spriteNoData')}</Text>
        ) : (
          <Flex wrap="wrap" gap={12}>
            {sprites.map((sprite) =>
              spriteSrcs[sprite.dirId] && (
                <SpriteCard
                  key={sprite.dirId}
                  manifest={sprite}
                  imageSrc={spriteSrcs[sprite.dirId]}
                  isSelected={selectedSpriteId === sprite.dirId}
                  onSelect={() => handleSpriteSelect(sprite.dirId)}
                  onDelete={() => handleSpriteDelete(sprite.dirId)}
                />
              )
            )}
          </Flex>
        )}

        {sprites.length > 0 && (
          <Flex vertical gap={16} style={{ maxWidth: 400 }}>
            <div>
              <Flex justify="space-between" align="center">
                <Text>{t('settings.spriteSize')}</Text>
                <Text type="secondary">{spriteSize}px</Text>
              </Flex>
              <Slider min={80} max={300} step={10} value={spriteSize} onChange={handleSpriteSizeChange} />
            </div>
            <div>
              <Flex justify="space-between" align="center">
                <Text>{t('settings.spriteBgRemoval')}</Text>
                <Text type="secondary">{bgThreshold.toFixed(2)}</Text>
              </Flex>
              <Slider
                min={0.01} max={0.50} step={0.01}
                value={bgThreshold}
                onChange={setBgThreshold}
                onChangeComplete={handleBgThresholdCommit}
                disabled={!selectedSpriteId || bgProcessing}
              />
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t('settings.spriteBgRemovalHint')}
              </Text>
            </div>
          </Flex>
        )}
      </Flex>
    </Flex>
  );
}
