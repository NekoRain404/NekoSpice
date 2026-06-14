#!/bin/bash
# NekoSpice 启动脚本
# wgpu GPU 初始化 + NekoSpiceApp::default() 需要较大的主线程栈空间
# 默认 8MB 不够，需要至少 256MB

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

# 将主线程栈空间提升至 256 MB（单位 KB）
ulimit -s 262144 2>/dev/null || ulimit -s unlimited 2>/dev/null || true

exec cargo run -p nsp-app "$@"
