# NekoSpice 项目结构

## Workspace Crates

```
NekoSpice/
├── Cargo.toml                     # Workspace 根配置
├── README.md                      # 项目说明（中文）
├── docs/
│   ├── TREE.md                    # 本文件：项目结构说明
│   ├── dev.md                     # 开发指南
│   ├── development-plan.md        # 开发路线图
│   ├── USER_MANUAL.md             # 用户手册
│   └── ui/                        # UI 参考图片
│       └── ui-ref-01..10.png
├── crates/
│   ├── nsp-core/                  # 核心数据类型与工具
│   ├── nsp-schema/                # 原理图格式解析器（Rust 原生）
│   ├── nsp-sim/                   # 仿真后端（ngspice/Xyce）
│   ├── nsp-waveform/              # 波形数据解析（raw/csv/FFT）
│   ├── nsp-netlist/               # 网表解析与转换
│   ├── nsp-render/                # SVG 渲染
│   ├── nsp-report/                # HTML/JSON 报告生成
│   ├── nsp-model/                 # TI/ADI 厂商模型目录
│   ├── nsp-cli/                   # 命令行工具
│   └── nsp-app/                   # GUI 应用（egui + wgpu）
│       └── src/
│           ├── main.rs            # 二进制入口
│           ├── lib.rs             # 库根，模块声明与常量
│           ├── document.rs        # NspGuiDocument 封装
│           ├── document_ops.rs    # 编辑操作
│           ├── document_tests/    # 文档测试模块
│           ├── library.rs         # NspGuiLibrary 封装
│           ├── viewport.rs        # CanvasViewport 缩放/平移
│           ├── canvas/            # 画布渲染层
│           ├── app/               # 应用状态与 UI
│           │   ├── home/          # 首页仪表板
│           │   ├── schematic/     # 原理图编辑器
│           │   ├── library/       # 符号库浏览
│           │   ├── simulation/    # 仿真配置与运行
│           │   ├── optimization/  # 参数优化
│           │   ├── review/        # 设计审查
│           │   ├── waveform/      # 波形分析
│           │   ├── reports/       # 报告生成
│           │   └── settings/      # 应用设置
│           └── simulation.rs      # GuiSimulationRun 管理
├── examples/                      # 示例工程
│   ├── cm5_minima/                # CM5 最小系统
│   ├── schema_schematic/          # 基础原理图
│   ├── schema_hierarchical/       # 层次化原理图
│   ├── schema_project/            # 项目文件
│   └── ...
└── scripts/
    └── screenshot.sh              # 一键截图脚本
```

## 架构说明

### 仿真流程
```
UI 仿真参数 → build_simulation_profile()
  → inject_profile_directives()
  → ngspice/Xyce CLI 执行
  → 解析日志与波形
  → 显示结果 → 保存到磁盘
```

### 核心设计原则
- **解耦**: 每个工作区独立子模块
- **文件管理**: 单文件不超过 300 行
- **状态分离**: state.rs 定义状态，sim_options.rs 定义参数
- **双后端**: 兼容 ngspice + Xyce
- **格式兼容**: 兼容 .kicad_sch/.kicad_sym 文件格式读写（文件格式标准命名）
