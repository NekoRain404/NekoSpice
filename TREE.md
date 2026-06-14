# NekoSpice 项目结构

> 基于 Rust 原生构建的 SPICE 仿真平台，支持 ngspice/Xyce 双后端求解器。

## 顶层目录

```
NekoSpice/
├── Cargo.toml              # 工作空间根配置（10 个 crate）
├── Cargo.lock              # 依赖锁定文件
├── README.md               # 项目说明（中文）
├── TREE.md                 # 本文件：项目结构说明
├── crates/                 # 所有 Rust crate（277 个源文件，62K 行）
├── docs/                   # 文档和 UI 参考图
│   ├── USER_MANUAL.md      # 用户手册（中文）
│   ├── ui/                 # UI 参考设计图（10 张）
│   ├── dev.md              # 开发者笔记
│   └── development-plan.md # 开发计划
├── scripts/                # 构建和运行脚本
│   └── run.sh              # GUI 启动脚本（设置栈空间 ≥ 256MB）
├── examples/               # 示例原理图
│   └── cm5_minima/         # CM5 最小示例
├── benchmarks/             # 性能基准测试
├── libs/                   # 外部库文件
└── runs/                   # 仿真运行输出（gitignore）
```

## Crates 模块架构

### 核心层（Core Layer）

| Crate | 职责 | 关键模块 |
|-------|------|----------|
| `nsp-core` | 基础类型、错误处理、运行工具 | `OslResult`, `RunStatus`, `make_run_id` |
| `nsp-schema` | 原理图格式解析和 SPICE 导出 | `sexpr`, `spice_export`, `connectivity`, `canvas` |
| `nsp-model` | SPICE 模型验证和厂商导入 | `model_check`, `vendor_import` |
| `nsp-render` | SVG 原理图渲染 | `svg_render`, `svg_helpers` |

### 仿真层（Simulation Layer）

| Crate | 职责 | 关键模块 |
|-------|------|----------|
| `nsp-sim` | 仿真后端、配置注入、日志解析 | `NgspiceCliBackend`, `XyceCliBackend`, `SimulationProfile` |
| `nsp-netlist` | 网表导入和格式转换 | `schema_import`, `netlist_parse`, `ltspice_import` |
| `nsp-waveform` | 波形数据解析和 FFT | `raw_parser`, `fft` |

### 报告层（Report Layer）

| Crate | 职责 |
|-------|------|
| `nsp-report` | HTML/JSON/Markdown/JUnit 报告生成 |

### 应用层（Application Layer）

| Crate | 职责 |
|-------|------|
| `nsp-app` | eframe GUI 应用（wgpu 硬件加速） |
| `nsp-cli` | 命令行工具 |

## nsp-app 模块结构

```
crates/nsp-app/src/
├── main.rs                    # 入口点
├── lib.rs                     # crate 入口
├── document.rs                # 原理图文档抽象层
├── document_ops.rs            # 文档编辑操作
├── library.rs                 # 符号库加载和浏览
├── viewport.rs                # 画布视口变换
├── simulation.rs              # 仿真任务分发器
├── simulation_run_loader.rs   # 运行结果加载
├── waveform_summary.rs        # 波形摘要
├── report_summary.rs          # 报告摘要
├── canvas/                    # 画布渲染
│   ├── scene_renderer.rs      # 场景渲染
│   ├── scene_renderer_wires.rs
│   └── primitives/            # 基础图元
├── placement_config.rs        # 符号放置配置
├── test_support.rs            # 测试辅助
└── app/                       # UI 主模块
    ├── app.rs                 # NekoSpiceApp 核心结构体
    ├── app_ops.rs             # 编辑操作
    ├── runtime.rs             # 窗口启动配置
    ├── theme.rs               # 主题和样式
    ├── navigation.rs          # 导航状态
    ├── schematic/             # 原理图编辑界面
    │   ├── toolbar.rs
    │   ├── workspace.rs
    │   ├── inspector/         # 属性检查器
    │   ├── tools/             # 编辑工具
    │   └── bottom_dock/       # 底部面板（波形、调试）
    ├── simulation/            # 仿真配置界面
    │   ├── panel.rs           # 仿真右侧面板
    │   ├── run_controller.rs  # 运行控制逻辑
    │   ├── analysis.rs        # 分析类型配置
    │   ├── directive_editor.rs
    │   ├── waveform_panel.rs
    │   └── options_*.rs       # 求解器选项
    ├── library/               # 符号库浏览界面
    ├── home/                  # 首页仪表板
    ├── optimization/          # 参数优化
    ├── review/                # 设计审查
    ├── reports/               # 报告生成
    ├── settings/              # 应用设置
    └── locale.rs / localization*.rs  # 国际化
```

## 测试覆盖

### 单元测试（244 通过，2 忽略）

| 测试套件 | 通过 | 描述 |
|----------|------|------|
| `nsp-schema` | 80 | 原理图解析、SPICE 导出、连通性 |
| `nsp-app` | 40 | UI 逻辑、文档操作 |
| `nsp-netlist` | 26 | 网表导入和转换 |
| `nsp-sim` | 14 + 15 e2e | 仿真后端 + **20 个端到端电路测试** |
| `nsp-waveform` | 11 | 波形解析和 FFT |
| `nsp-core` | 10 | 核心类型 |
| 其他 | 43 | 报告、渲染、模型 |

### 端到端仿真测试（20 个真实 KiCad 电路）

| 电路 | 结果 | 说明 |
|------|------|------|
| RC filter | ✅ + 波形解析 | RC 低通滤波器 |
| 555-bipolar | ✅ + 波形解析 | 555 定时器（BJT） |
| Sallen-Key lowpass | ✅ + 波形解析 | 二阶有源低通 |
| Sallen-Key highpass | ✅ | 二阶有源高通 |
| Buck converter | ✅ | 开关电源（MOSFET） |
| Boost converter | ✅ | 升压转换器 |
| CMOS555 | ✅ | CMOS 555 定时器 |
| 741 opamp | ✅ | 运算放大器 |
| analog-multiplier | ✅ | 模拟乘法器 |
| PWM audio | ✅ | PWM 音频放大 |
| Class-D | ⚠️ | 复杂模型，ngspice 兼容性有限 |
| FullBridge | ⚠️ | table() E-source 不支持 |

## 构建和运行

```bash
# 构建
cargo build --release

# 运行 GUI（需要 ≥ 256MB 栈空间）
./scripts/run.sh
# 或手动：ulimit -s unlimited && cargo run -p nsp-app

# 运行所有测试
cargo test --workspace

# 运行端到端仿真测试
cargo test -p nsp-sim --test e2e_simulation -- --nocapture
```

## 依赖架构

```
nsp-app → nsp-schema, nsp-sim, nsp-waveform, nsp-report, nsp-model, nsp-core
nsp-cli → nsp-schema, nsp-netlist, nsp-core
nsp-sim → nsp-core, nsp-waveform
nsp-netlist → nsp-schema, nsp-core
nsp-schema → nsp-core
nsp-render → nsp-core
nsp-waveform → nsp-core
nsp-report → nsp-core
nsp-model → nsp-core
```
