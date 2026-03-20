import { useEffect, useRef, useState } from 'react';
import { Card, Button, Popconfirm, Typography } from 'antd';
import { DeleteOutlined, CheckCircleFilled } from '@ant-design/icons';
import type { SpriteManifest } from './SpriteAnimation';

const { Text } = Typography;

interface SpriteCardProps {
  manifest: SpriteManifest & { dirId: string };
  imageSrc: string;
  isSelected: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

export default function SpriteCard({
  manifest,
  imageSrc,
  isSelected,
  onSelect,
  onDelete,
}: SpriteCardProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const PREVIEW_SIZE = 100;

  useEffect(() => {
    const img = new Image();
    img.onload = () => setImage(img);
    img.src = imageSrc;
    return () => { img.onload = null; };
  }, [imageSrc]);

  // Animation loop
  useEffect(() => {
    if (!image || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let frame = 0;
    const draw = () => {
      const col = frame % manifest.columns;
      const row = Math.floor(frame / manifest.columns);
      ctx.clearRect(0, 0, PREVIEW_SIZE, PREVIEW_SIZE);
      ctx.drawImage(
        image,
        col * manifest.frameWidth, row * manifest.frameHeight,
        manifest.frameWidth, manifest.frameHeight,
        0, 0, PREVIEW_SIZE, PREVIEW_SIZE,
      );
    };

    draw();
    const id = setInterval(() => {
      frame = (frame + 1) % manifest.frameCount;
      draw();
    }, 80);

    return () => clearInterval(id);
  }, [image, manifest]);

  return (
    <Card
      hoverable
      onClick={onSelect}
      style={{
        width: 140,
        border: isSelected ? '2px solid #1677ff' : '2px solid transparent',
        borderRadius: 12,
        overflow: 'hidden',
        position: 'relative',
      }}
      styles={{ body: { padding: 8, textAlign: 'center' } }}
    >
      {isSelected && (
        <CheckCircleFilled
          style={{
            position: 'absolute',
            top: 8,
            right: 8,
            fontSize: 18,
            color: '#1677ff',
            zIndex: 1,
          }}
        />
      )}
      <canvas
        ref={canvasRef}
        width={PREVIEW_SIZE}
        height={PREVIEW_SIZE}
        style={{ width: PREVIEW_SIZE, height: PREVIEW_SIZE, display: 'block', margin: '0 auto' }}
      />
      <Text
        style={{ fontSize: 12, display: 'block', marginTop: 4 }}
        ellipsis={{ tooltip: manifest.name }}
      >
        {manifest.name}
      </Text>
      <Text type="secondary" style={{ fontSize: 10 }}>
        {manifest.columns}x{manifest.rows} · {manifest.frameCount}帧
      </Text>
      <Popconfirm
        title="确认删除此精灵图？"
        onConfirm={(e) => { e?.stopPropagation(); onDelete(); }}
        onCancel={(e) => e?.stopPropagation()}
        okText="删除"
        cancelText="取消"
      >
        <Button
          type="text"
          danger
          size="small"
          icon={<DeleteOutlined />}
          onClick={(e) => e.stopPropagation()}
          style={{ marginTop: 4 }}
        />
      </Popconfirm>
    </Card>
  );
}
