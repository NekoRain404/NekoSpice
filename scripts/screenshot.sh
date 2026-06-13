#!/bin/bash
# NekoSpice 一键截图脚本
# 用法: ./scripts/screenshot.sh [输出路径] [等待秒数]
#
# 流程: 后台启动应用 → 等待渲染 → spectacle 截取活动窗口 → kill 清理

set -euo pipefail

OUTPUT="${1:-/tmp/nekospice_screenshot.png}"
WAIT_SECS="${2:-5}"
CARGO_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# 清理旧进程
pkill -f "nekospice" 2>/dev/null || true
pkill -f "target/debug/nsp_app" 2>/dev/null || true
sleep 0.5

echo "[1/4] 启动 NekoSpice..."
cd "$CARGO_DIR"
cargo run -p osl-app &>/dev/null &
APP_PID=$!

echo "[2/4] 等待窗口渲染 (${WAIT_SECS}s)..."
sleep "$WAIT_SECS"

echo "[3/4] 截取活动窗口..."
spectacle -b -a -o "$OUTPUT" 2>/dev/null

echo "[4/4] 关闭应用..."
kill $APP_PID 2>/dev/null || true
sleep 0.5
# 强杀残留进程
pkill -9 -f "target/debug/nsp_app" 2>/dev/null || true

if [ -f "$OUTPUT" ]; then
    SIZE=$(stat -c%s "$OUTPUT" 2>/dev/null || stat -f%z "$OUTPUT" 2>/dev/null)
    echo "✓ 截图已保存: $OUTPUT ($SIZE bytes)"
else
    echo "✗ 截图失败"
    exit 1
fi
