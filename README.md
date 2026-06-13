# NekoSpice

> 🐱 基于 Rust 原生构建的高性能 SPICE 仿真平台

NekoSpice 是一款现代化的电路仿真软件，采用 Rust 语言从零构建，兼容标准原理图格式，支持 ngspice 和 Xyce 双后端求解器。目标是打造最强的开源 SPICE 仿真工具。

## 核心特性

### 🎨 原理图编辑器
- 兼容 KiCad 原理图格式 (`.kicad_sch`)
- 实时连通性分析和 ERC 检查
- 符号库浏览和放置
- 导线、标签、总线编辑工具
- 硬件加速渲染 (egui + wgpu)

### ⚡ 仿真引擎
- **ngspice** — 开源 SPICE 仿真器
- **Xyce** — 高性能并行仿真器
- 支持 DC、AC、Transient、Noise 分析
- 参数扫描和 Monte Carlo 优化
- 自动 MOSFET/BJT/子电路类型检测

### 📊 波形分析
- 时域波形查看和测量
- FFT 频域分析（Hanning 窗函数）
- Bode 图（增益/相位）
- 噪声分析和眼图

### 📋 报告生成
- HTML/JSON/Markdown 格式报告
- ERC 检查报告和风险评估
- 仿真结果汇总和导出

### 🌐 国际化
- 中文/英文双语界面
- 完整的快捷键系统
- 可自定义主题

## 快速开始

### 环境要求

- Rust 1.75+
- ngspice（已安装并添加到 PATH）
- Linux（Wayland 或 X11）

### 构建和运行

```bash
# 克隆项目
git clone https://github.com/NekoRain404/NekoSpice.git
cd NekoSpice

# 构建 GUI
cargo build -p nsp-app

# 运行（需要较大栈空间）
bash scripts/run.sh

# 或手动设置栈空间
ulimit -s unlimited && cargo run -p nsp-app
```

### CLI 仿真

```bash
# 从原理图运行仿真
cargo run -p nsp-cli -- run-schematic examples/cm5_minima/CM5.nsp_sch

# 运行网表文件
cargo run -p nsp-cli -- run netlist.cir
```

## 项目架构

```
NekoSpice/
├── crates/
│   ├── nsp-core/      # 核心类型和工具
│   ├── nsp-schema/    # 原理图解析和 SPICE 导出
│   ├── nsp-netlist/   # 网表解析和转换
│   ├── nsp-sim/       # 仿真后端引擎
│   ├── nsp-waveform/  # 波形数据解析
│   ├── nsp-model/     # 厂商模型库（TI/ADI）
│   ├── nsp-render/    # SVG 原理图渲染
│   ├── nsp-report/    # 报告生成
│   ├── nsp-cli/       # 命令行工具
│   └── nsp-app/       # GUI 应用
├── docs/              # 文档和 UI 参考图
├── examples/          # 示例原理图
└── scripts/           # 构建脚本
```

详细的项目结构说明请查看 [TREE.md](TREE.md)。

## 仿真测试结果

使用开源 KiCad 仿真 demo 进行测试：

| 状态 | 数量 | 通过率 |
|------|------|--------|
| ✅ 通过 | 23 | 59% |
| ❌ 失败 | 16 | 41% |
| **总计** | **39** | — |

已通过的 demo 包括：555 定时器、741 运放、Boost/Buck 变换器、Class-D 放大器、PWM 音频、RC 滤波器、Sallen-Key 滤波器等。

## 开发状态

- ✅ 核心仿真流程
- ✅ 原理图解析和 SPICE 导出
- ✅ MOSFET/BJT/子电路类型自动检测
- ✅ ngspice/Xyce 双后端支持
- ✅ 波形查看和 FFT 分析
- ✅ GUI 基础框架
- 🔄 原理图编辑体验优化
- 🔄 更多厂商模型支持
- 🔄 高级优化功能

## 技术栈

| 组件 | 技术 |
|------|------|
| 语言 | Rust |
| GUI | egui + wgpu |
| 仿真 | ngspice / Xyce |
| 格式 | KiCad S-expression |
| 渲染 | wgpu 硬件加速 |
| 构建 | Cargo workspace |

## 许可证

MIT License

## 致谢

- [KiCad](https://www.kicad.org/) — 原理图格式参考
- [ngspice](http://ngspice.sourceforge.net/) — 开源 SPICE 仿真器
- [egui](https://github.com/emilk/egui) — Rust GUI 框架
- [wgpu](https://github.com/gfx-rs/wgpu) — 跨平台图形 API
