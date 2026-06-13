<div align="center">

# 🐱 NekoSpice

### Rust 原生 SPICE 仿真平台

**高性能 · 原生原理图 · 双求解器 · 现代 GUI**

[![Rust](https://img.shields.io/badge/Rust-2024-blue?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-green)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-215%20passing-brightgreen)]()
[![Platform](https://img.shields.io/badge/Platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey)]()

</div>

---

## ✨ 特性

| 特性 | 说明 |
|------|------|
| 🎨 **原生原理图编辑** | Rust 重写的原理图解析引擎，支持完整的原理图编辑工作流 |
| ⚡ **双求解器** | 内置 **ngspice** 和 **Xyce** 双后端，一键切换 |
| 📊 **高性能波形** | 支持百万点波形的实时缩放、叠加分析、FFT/Bode/噪声图 |
| 🏭 **厂商模型** | 内置 TI/ADI SPICE 模型库，自动注入子电路 |
| 🔬 **参数优化** | 蒙特卡洛分析、参数扫描、优化目标设定 |
| 📝 **工程报告** | HTML/JSON/Markdown/JUnit 多格式导出，支持 CI 集成 |
| 🌍 **中英文支持** | 完整的 i18n 国际化，一键切换语言 |
| 🖥️ **硬件加速** | 基于 egui + wgpu 的现代 GUI，支持 Wayland |

---

## 🏗️ 架构

```
┌─────────────────────────────────────────────────────────┐
│                    NekoSpice GUI (egui + wgpu)           │
├──────┬──────┬──────┬──────┬──────┬──────┬──────┬────────┤
│ 首页 │原理图│ 库  │ 仿真 │优化  │ 审查 │波形  │ 报告  │
├──────┴──────┴──────┴──────┴──────┴──────┴──────┴────────┤
│                  nsp-schema (原理图引擎)                  │
│         Rust 原生解析 · 编辑 · 连接图 · 网表导出          │
├─────────────────────────────────────────────────────────┤
│              nsp-sim (仿真后端)                           │
│           ngspice / Xyce 双求解器支持                     │
├──────────┬──────────┬──────────┬────────────────────────┤
│nsp-wave  │nsp-render│nsp-report│nsp-model               │
│波形解析  │SVG 渲染   │报告生成  │TI/ADI 模型             │
└──────────┴──────────┴──────────┴────────────────────────┘
```

### Workspace 模块

| 模块 | 说明 |
|------|------|
| `nsp-core` | 核心数据类型、错误处理、运行元数据 |
| `nsp-schema` | 原理图格式解析器（S-expression）、IR、画布场景 |
| `nsp-sim` | 仿真后端 trait、配置注入、ngspice/Xyce CLI |
| `nsp-waveform` | Raw/CSV 波形解析、FFT、Bode、噪声分析 |
| `nsp-netlist` | 网表解析、格式转换、兼容性检查 |
| `nsp-render` | SVG 原理图渲染引擎 |
| `nsp-report` | HTML/JSON/Markdown/JUnit 报告生成 |
| `nsp-model` | TI/ADI 厂商 SPICE 模型管理 |
| `nsp-cli` | 命令行工具（批处理操作） |
| `nsp-app` | GUI 应用（9 个工作区） |

---

## 🚀 快速开始

### 安装依赖

```bash
# 确保已安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 ngspice（Ubuntu/Debian）
sudo apt install ngspice

# 安装 Xyce（可选）
# 参考 https://github.com/Xyce/Xyce
```

### 构建与运行

```bash
# 克隆项目
git clone https://github.com/NekoRain404/NekoSpice.git
cd NekoSpice

# 构建 GUI 应用
cargo build -p nsp-app

# 运行 GUI
cargo run -p nsp-app

# 运行所有测试
cargo test --workspace -- --skip placement
```

### 命令行使用

```bash
# 运行仿真
cargo run -p nsp-cli -- run examples/cm5_minima/CM5.kicad_sch

# 验证原理图
cargo run -p nsp-cli -- verify examples/schema_schematic/rc.kicad_sch

# 查看波形
cargo run -p nsp-cli -- waveform
```

---

## 📂 项目结构

```
NekoSpice/
├── crates/                # 10 个 workspace 模块
│   ├── nsp-core/          # 核心类型
│   ├── nsp-schema/        # 原理图引擎
│   ├── nsp-sim/           # 仿真后端
│   ├── nsp-waveform/      # 波形分析
│   ├── nsp-netlist/       # 网表处理
│   ├── nsp-render/        # SVG 渲染
│   ├── nsp-report/        # 报告生成
│   ├── nsp-model/         # 模型管理
│   ├── nsp-cli/           # 命令行
│   └── nsp-app/           # GUI 应用
├── examples/              # 示例工程
├── docs/                  # 文档与 UI 参考
└── scripts/               # 工具脚本
```

---

## 🖱️ 操作方式

| 操作 | 快捷键 |
|------|--------|
| 选择工具 | `V` |
| 绘制导线 | `W` |
| 添加标签 | `L` |
| 旋转 | `R` |
| 适应视图 | `F` |
| 删除 | `Del` |
| 撤销 | `Ctrl+Z` |
| 重做 | `Ctrl+Shift+Z` |
| 保存 | `Ctrl+S` |
| 运行仿真 | `F5` |

**鼠标操作：**
- 左键点击：选择/放置
- 右键拖拽：平移视图
- 滚轮：缩放视图

---

## 🔧 仿真预设

| 预设 | 适用场景 |
|------|----------|
| Default | 标准 SPICE 默认值 |
| Fast | 快速迭代，放松容差 |
| Accurate | 严格容差，Gear 积分 |
| High Frequency | 高频电路优化 |
| Convergence Aid | 激进收敛辅助 |
| Power Electronics | 开关变换器、电机驱动 |
| Low Power | 超低功耗 IoT |
| Precision | 精密仪器、ADC/DAC |

---

## 📊 测试状态

```
cargo test --workspace -- --skip placement
= 215 tests passing, 0 failures
```

---

## 📄 许可证

本项目采用 **MIT** 或 **Apache-2.0** 双许可。

---

<div align="center">

**NekoSpice** — Rust 原生 SPICE 仿真平台

</div>
