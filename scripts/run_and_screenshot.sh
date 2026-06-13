#!/usr/bin/env bash
# 启动 NekoSpice 并截图，用于 UI 验证。
# 用法: ./scripts/run_and_screenshot.sh [等待秒数] [输出路径]
set -euo pipefail
WAIT_SECS="${1:-6}"
OUTPUT="${2:-/tmp/neko-screenshot.png}"
cd "$(dirname "$0")/.."

# 终止已有实例
killall nekospice 2>/dev/null || true
sleep 0.5

# 后台启动
./target/debug/nekospice &
APP_PID=$!
echo "nekospice started (PID=$APP_PID), waiting ${WAIT_SECS}s..."

# 等待窗口就绪
sleep "$WAIT_SECS"

# 检查进程存活
if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "ERROR: nekospice exited before screenshot" >&2
    exit 1
fi

# 截取活动窗口
spectacle -b -a -o "$OUTPUT" 2>&1
echo "Screenshot saved to $OUTPUT"
ls -la "$OUTPUT"
