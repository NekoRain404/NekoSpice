#!/bin/bash
# NekoSpice 启动脚本
# eframe + wgpu 需要较大的主线程栈空间来完成 GPU 初始化
# 默认 8MB 栈空间不够，需要至少 32MB

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

# 增加主线程栈空间到 32 MB
ulimit -s 32768 2>/dev/null || true

exec cargo run -p nsp-app "$@"
