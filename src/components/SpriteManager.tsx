import { useEffect, useState, useCallback } from 'react';
import { Button, Flex, Slider, Space, Typography, message, Empty } from 'antd';
import { FolderOpenOutlined, FileZipOutlined } from '@ant-design/icons';
import { invoke, formatError, logEvent } from '../lib/logger';
import type { SpriteManifest } from './SpriteAnimation';
import SpriteCard from './SpriteCard';

const { Text } = Typography;

type SpriteEntry = SpriteManifest & { dirId: string };

interface SpriteManagerProps {
  selectedSpriteId: string;
  onSelectedChange: (dirId: string) => void;
}

export default function SpriteManager({ selectedSpriteId, onSelectedChange }: SpriteManagerProps) {
  const [sprites, setSprites] = useState<SpriteEntry[]>([]);
  const [imageSrcs, setImageSrcs] = useState<Record<string, string>>({});
  const [threshold, setThreshold] = useState(0.18);
  const [processing, setProcessing] = useState(false);

  const loadSprites = useCallback(async () => {
    try {
      const list = await invoke<SpriteEntry[]>('list_sprites');
      setSprites(list);
      // Load images for all sprites
      const srcs: Record<string, string> = {};
      await Promise.all(
        list.map(async (s) => {
          try {
            srcs[s.dirId] = await invoke<string>('get_sprite_image', {
              dirId: s.dirId,
              fileName: 'sprite_processed.png',
            });
          } catch {
            try {
              srcs[s.dirId] = await invoke<string>('get_sprite_image', {
                dirId: s.dirId,
                fileName: s.spriteFile,
              });
            } catch { /* skip */ }
          }
        })
      );
      setImageSrcs(srcs);
    } catch (e) {
      message.error(formatError(e, '加载精灵图失败'));
    }
  }, []);

  useEffect(() => { loadSprites(); }, [loadSprites]);

  const handleImportFolder = async () => {
    try {
      const result = await invoke<SpriteEntry | null>('import_sprite_folder');
      if (result) {
        logEvent('Sprite', 'import_folder', { name: result.name });
        message.success(`导入成功: ${result.name}`);
        await loadSprites();
      }
    } catch (e) {
      message.error(formatError(e, '导入失败'));
    }
  };

  const handleImportZip = async () => {
    try {
      const result = await invoke<SpriteEntry | null>('import_sprite_zip');
      if (result) {
        logEvent('Sprite', 'import_zip', { name: result.name });
        message.success(`导入成功: ${result.name}`);
        await loadSprites();
      }
    } catch (e) {
      message.error(formatError(e, '导入失败'));
    }
  };

  const handleDelete = async (dirId: string) => {
    try {
      await invoke('delete_sprite', { dirId });
      logEvent('Sprite', 'delete', { dirId });
      message.success('已删除');
      if (selectedSpriteId === dirId) {
        onSelectedChange('');
      }
      await loadSprites();
    } catch (e) {
      message.error(formatError(e, '删除失败'));
    }
  };

  const handleSelect = (dirId: string) => {
    const newValue = dirId === selectedSpriteId ? '' : dirId;
    onSelectedChange(newValue);
  };

  const handleThresholdCommit = async (value: number) => {
    setThreshold(value);
    if (!selectedSpriteId) return;
    setProcessing(true);
    try {
      await invoke('process_sprite_background', {
        dirId: selectedSpriteId,
        threshold: value,
      });
      logEvent('Sprite', 'process_background', { dirId: selectedSpriteId, threshold: value });
      await loadSprites();
    } catch (e) {
      message.error(formatError(e, '处理失败'));
    }
    setProcessing(false);
  };

  return (
    <Flex vertical gap={12} style={{ width: '100%' }}>
      <Space>
        <Button icon={<FolderOpenOutlined />} onClick={handleImportFolder}>
          导入文件夹
        </Button>
        <Button icon={<FileZipOutlined />} onClick={handleImportZip}>
          导入 ZIP
        </Button>
      </Space>

      {sprites.length === 0 ? (
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description="暂无精灵图，请导入"
        />
      ) : (
        <Flex wrap="wrap" gap={12}>
          {sprites.map((sprite) => (
            imageSrcs[sprite.dirId] && (
              <SpriteCard
                key={sprite.dirId}
                manifest={sprite}
                imageSrc={imageSrcs[sprite.dirId]}
                isSelected={selectedSpriteId === sprite.dirId}
                onSelect={() => handleSelect(sprite.dirId)}
                onDelete={() => handleDelete(sprite.dirId)}
              />
            )
          ))}
        </Flex>
      )}

      {sprites.length > 0 && (
        <div style={{ maxWidth: 400 }}>
          <Flex justify="space-between" align="center">
            <Text>背景去除</Text>
            <Text type="secondary">{threshold.toFixed(2)}</Text>
          </Flex>
          <Slider
            min={0.01}
            max={0.50}
            step={0.01}
            value={threshold}
            onChange={setThreshold}
            onChangeComplete={handleThresholdCommit}
            disabled={!selectedSpriteId || processing}
          />
          <Text type="secondary" style={{ fontSize: 12 }}>
            值越高去除背景越彻底，但可能影响精灵图本体颜色。需先选中一个精灵图。
          </Text>
        </div>
      )}
    </Flex>
  );
}
