#!/usr/bin/env bash
set -euo pipefail

# 统一更新所有版本号
# 用法: ./scripts/bump-version.sh <version>
# 示例: ./scripts/bump-version.sh 0.3.0

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
  echo "用法: $0 <version>"
  echo "示例: $0 0.3.0"
  exit 1
fi

# 校验语义化版本格式
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "错误: 版本号必须为语义化格式 (x.y.z)，当前输入: $VERSION"
  exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# 需要更新的文件列表
FILES=(
  "$ROOT/package.json"
  "$ROOT/src-tauri/tauri.conf.json"
  "$ROOT/src-tauri/Cargo.toml"
)

for f in "${FILES[@]}"; do
  if [[ ! -f "$f" ]]; then
    echo "警告: 文件不存在，跳过: $f"
    continue
  fi

  case "$f" in
    *.json)
      # JSON 文件：匹配顶层 "version" 字段
      sed -i '' -E 's/("version":[[:space:]]*")[0-9]+\.[0-9]+\.[0-9]+(")/\1'"$VERSION"'\2/' "$f"
      ;;
    *.toml)
      # TOML 文件：匹配 version = "x.y.z"
      sed -i '' -E 's/(^version[[:space:]]*=[[:space:]]*")[0-9]+\.[0-9]+\.[0-9]+(")/\1'"$VERSION"'\2/' "$f"
      ;;
  esac
done

# 更新 Cargo.lock
if [[ -f "$ROOT/src-tauri/Cargo.lock" ]]; then
  (cd "$ROOT/src-tauri" && cargo update -p yumo --precise "$VERSION" 2>/dev/null || true)
fi

echo "已更新版本号为 $VERSION:"
for f in "${FILES[@]}"; do
  if [[ -f "$f" ]]; then
    short="${f#$ROOT/}"
    current=$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' <<< "$(grep -m1 'version' "$f")")
    echo "  $short → $current"
  fi
done
