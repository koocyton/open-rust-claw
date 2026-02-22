#!/usr/bin/env bash
# 为 OpenWrt ARM64 (MT7981B / MTK FILOGIC 820 等) 交叉编译 rust-bot
# 目标：aarch64-unknown-linux-musl（与 OpenWrt 常用 musl 一致）

set -e
cd "$(dirname "$0")/.."
TARGET="aarch64-unknown-linux-musl"
OUT_DIR="${1:-./openwrt-out}"

echo "目标: $TARGET (OpenWrt ARM64)"
rustup target add "$TARGET" 2>/dev/null || true

if command -v cross &>/dev/null; then
  echo "使用 cross 编译..."
  cross build --target "$TARGET" --release
else
  echo "使用 cargo 编译（需本机已配置 $TARGET 的 linker）..."
  cargo build --target "$TARGET" --release
fi

mkdir -p "$OUT_DIR"
cp "target/$TARGET/release/rust-bot" "$OUT_DIR/"
echo "已输出: $OUT_DIR/rust-bot"
echo "可上传到 OpenWrt 设备运行（需 config.toml 与依赖库）。"
