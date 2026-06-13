# NekoSpice 项目结构

> 基于 Rust 原生构建的 SPICE 仿真平台，支持 ngspice/Xyce 双后端求解器。

## 顶层目录

```
NekoSpice/
├── Cargo.toml              # 工作空间根配置
├── Cargo.lock              # 依赖锁定文件
├── README.md               # 项目说明（中文）
├── TREE.md                 # 本文件：项目结构说明
├── crates/                 # 所有 Rust crate
├── docs/                   # 文档和 UI 参考图
│   ├── README.md           # 文档说明
│   ├── USER_MANUAL.md      # 用户手册（中文）
│   ├── TREE.md             # 旧结构文件
│   ├── ui/                 # UI 参考设计图
│   │   ├── ui-ref-01.png   # 首页仪表板
│   │   ├── ui-ref-02.png   # 原理图编辑器
│   │   ├── ui-ref-03.png   # 库浏览
│   │   ├── ui-ref-04.png   # 仿真配置
│   │   ├── ui-ref-05.png   # 波形查看器
│   │   ├── ui-ref-06.png   # 优化工作台
│   │   ├── ui-ref-07.png   # 设计审查
│   │   ├── ui-ref-08.png   # 报告生成
│   │   ├── ui-ref-09.png   # 设置面板
│   │   └── ui-ref-10.png   # 高级功能
│   ├── dev.md              # 开发者笔记
│   ├── development-plan.md # 开发计划
│   └── three-day-sprint.md # 三日冲刺计划
├── scripts/                # 构建和运行脚本
│   ├── run.sh              # GUI 启动脚本（设置栈空间）
│   └── screenshot.sh       # UI 截图脚本
├── examples/               # 示例原理图
│   └── cm5_minima/         # CM5 最小示例
└── runs/                   # 仿真运行输出（gitignore）
```

## Crates 模块架构

### 核心层（Core Layer）

```
crates/
├── nsp-core/               # 核心类型和工具
│   └── src/
│       └── lib.rs          # OslResult, RunStatus, 基础类型
│
├── nsp-schema/             # 原理图格式解析和 SPICE 导出
│   ├── src/
│   │   ├── lib.rs          # 模块入口，公共类型导出
│   │   ├── spice_export.rs # SPICE 网表导出（MOSFET检测、子电路处理）
│   │   ├── symbols.rs      # 符号实例和属性定义
│   │   ├── symbols_parse_impl.rs  # S-expression 符号解析
│   │   ├── connectivity.rs # 连接性图和网络分析
│   │   ├── simulation.rs   # 仿真指令解析
│   │   ├── schematic_library_impl.rs  # 库符号解析
│   │   ├── schematic_check_impl.rs    # ERC 检查
│   │   └── transform.rs    # 坐标变换
│   ├── tests/              # 集成测试
│   └── docs/               # crate 文档
│
├── nsp-netlist/            # 网表解析和格式转换
│   ├── src/
│   │   ├── lib.rs          # 公共接口
│   │   └── ...             # 网表解析器
│   └── tests/
│
├── nsp-waveform/           # 波形数据解析
│   └── src/
│       ├── lib.rs          # 公共接口
│       ├── raw.rs          # ngspice raw 格式解析
│       ├── csv.rs          # CSV 波形解析
│       └── fft.rs          # FFT 频域分析
│
└── nsp-model/              # 厂商模型库
    └── src/
        ├── lib.rs          # 模型目录接口
        ├── ti.rs           # TI SPICE 模型
        └── adi.rs          # ADI SPICE 模型
```

### 仿真层（Simulation Layer）

```
crates/
├── nsp-sim/                # 仿真后端引擎
│   └── src/
│       ├── lib.rs          # 仿真引擎 trait 和实现
│       │                 # - NgspiceCliBackend
│       │                 # - XyceCliBackend
│       │                 # - ensure_ngspice_control_exports()
│       │                 # - 默认 .tran 注入
│       └── ...             # 网表注入、控制块处理
│
├── nsp-report/             # 仿真报告生成
│   └── src/
│       ├── lib.rs          # 报告生成接口
│       ├── html.rs         # HTML 报告
│       ├── json.rs         # JSON 报告
│       └── markdown.rs     # Markdown 报告
│
└── nsp-render/             # SVG 原理图渲染
    └── src/
        ├── lib.rs          # 渲染接口
        └── svg_render_impl.rs  # SVG 渲染实现
```

### 应用层（Application Layer）

```
crates/
├── nsp-cli/                # 命令行工具
│   └── src/
│       ├── main.rs         # CLI 入口
│       ├── cli_run.rs      # run-schematic 命令
│       │                 # - normalize_spice_models()
│       │                 # - resolve_include_paths()
│       │                 # - normalize_included_lib_files()
│       └── ...             # 其他子命令
│
└── nsp-app/                # GUI 应用（egui + wgpu）
    └── src/
        ├── main.rs         # 应用入口
        ├── lib.rs          # run_native() 启动
        ├── app.rs          # 主应用结构
        ├── document.rs     # 文档模型
        ├── simulation.rs   # 仿真管理
        ├── library.rs      # 库管理
        ├── viewport.rs     # 视口管理
        │
        ├── app/            # UI 模块（按功能组织）
        │   ├── home/       # 首页仪表板
        │   ├── schematic/  # 原理图编辑器
        │   │   ├── toolbar.rs        # 工具栏
        │   │   ├── workspace.rs      # 工作区布局
        │   │   ├── inspector/        # 属性面板
        │   │   ├── bottom_dock/      # 底部面板
        │   │   └── tools/            # 编辑工具
        │   ├── library/    # 库浏览
        │   ├── simulation/ # 仿真配置
        │   │   ├── state.rs          # 仿真状态
        │   │   ├── analysis.rs       # 分析参数
        │   │   ├── profile_editor.rs # 配置编辑器
        │   │   ├── run_controller.rs # 运行控制
        │   │   ├── options_solver.rs # 求解器选项
        │   │   ├── waveform_panel.rs # 波形面板
        │   │   └── ...               # 更多子模块
        │   ├── waveform/   # 波形分析
        │   ├── optimization/ # 参数优化
        │   ├── review/     # 设计审查
        │   ├── reports/    # 报告生成
        │   ├── settings/   # 设置面板
        │   ├── theme.rs    # 主题定义
        │   ├── locale.rs   # 国际化
        │   └── navigation.rs # 导航管理
        │
        ├── canvas/         # 原理图画布
        │   ├── scene_renderer.rs     # 场景渲染器
        │   ├── scene_renderer_wires.rs  # 导线渲染
        │   ├── scene_renderer_annotations.rs # 标注渲染
        │   ├── primitives/ # 渲染图元
        │   │   ├── grid.rs           # 网格
        │   │   ├── sheet.rs          # 图纸边框
        │   │   ├── symbol.rs         # 符号渲染
        │   │   └── text.rs           # 文本渲染
        │   ├── transforms.rs # 坐标变换
        │   └── colors.rs   # 颜色定义
        │
        ├── document_tests/ # 文档模型测试
        └── simulation_tests.rs # 仿真测试
```

## 仿真工作流

```
原理图 (.kicad_sch / .nsp_sch)
    ↓ 解析
NspSchematic (内存模型)
    ↓ 连接性分析
NspNetGraph (网络图)
    ↓ SPICE 导出
spice_export.rs → netlist text
    ├─ 符号 → SPICE 元件行
    ├─ 连线 → 节点连接
    ├─ .include → 库引用
    ├─ .model → 默认模型注入
    ├─ .tran → 默认分析注入
    └─ .end → 结束标记
    ↓ 后处理
normalize_spice_models() → 修正模型类型
resolve_include_paths() → 解析库路径
normalize_included_lib_files() → 修正库内模型
    ↓ 仿真引擎
NgspiceCliBackend / XyceCliBackend
    ↓ 结果解析
RunMetadata + WaveformData
    ↓ 可视化
波形显示 / FFT 分析 / Bode 图
```

## 技术栈

| 组件 | 技术 |
|------|------|
| GUI 框架 | egui 0.34 + wgpu |
| 仿真后端 | ngspice / Xyce |
| 原理图格式 | KiCad S-expression (.kicad_sch) |
| 波形格式 | ngspice raw / CSV |
| 报告格式 | HTML / JSON / Markdown |
| 构建系统 | Cargo workspace |
| 版本控制 | Git |
| 平台支持 | Linux (Wayland/X11) |

## 快速命令

```bash
# 构建 GUI
cargo build -p nsp-app

# 运行 GUI
bash scripts/run.sh

# 运行所有测试（215 个）
cargo test --workspace -- --skip placement

# CLI 仿真
cargo run -p nsp-cli -- run-schematic <file.kicad_sch>

# 代码检查
cargo clippy --workspace --all-targets
cargo fmt --all
```
