#!/bin/bash
# NekoSpice 启动脚本
# wgpu GPU 初始化 + NekoSpiceApp::default() 需要大主线程栈空间
# 使用 512MB（unlimited 会导致 SIGKILL）

set -e
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

ulimit -s 524288 2>/dev/null || ulimit -s 262144 2>/dev/null || true
exec cargo run -p nsp-app "$@"
