# NekoSpice 项目结构

> NekoSpice 是一个基于 Rust 的 SPICE 仿真平台，集成 KiCad 原理图编辑、
> ngspice/Xyce 仿真后端，以及硬件加速 GUI（egui + wgpu）。

---

## 根目录

```
NekoSpice/
├── Cargo.toml              # 工作区根：10 个 crate，Rust 2024 edition
├── Cargo.lock              # 依赖锁定文件
├── README.md               # 项目概览、CLI 用法、快速开始
├── PROJECT_STRUCTURE.md    # 本文件
├── .gitignore              # Git 忽略规则
├── crates/                 # 所有 Rust 源代码（10 个工作区 crate）
├── docs/                   # 文档和 UI 参考图片
├── examples/               # 测试用例和演示项目
├── benchmarks/             # 基准测试配置
├── runs/                   # 仿真运行输出（已 gitignore）
└── kicad-source-mirror-master/  # KiCad 源码参考（已 gitignore）
```

---

## crates/ — 工作区 Crate

### 架构分层

```
┌─────────────────────────────────────────────────┐
│  osl-app (GUI)        osl-cli (CLI 二进制)       │  ← 用户界面层
├─────────────────────────────────────────────────┤
│  osl-render   osl-report   osl-waveform          │  ← 输出与可视化层
├─────────────────────────────────────────────────┤
│  osl-sim      osl-netlist   osl-model             │  ← 仿真与导入层
├─────────────────────────────────────────────────┤
│  osl-kicad                                          │  ← KiCad IR 与操作层
├─────────────────────────────────────────────────┤
│  osl-core                                          │  ← 共享基础层
└─────────────────────────────────────────────────┘
```

### 依赖关系

```
osl-app ──────┬── osl-kicad ──── osl-core
              ├── osl-sim ────── osl-core
              ├── osl-render ─── osl-kicad
              ├── osl-waveform
              └── osl-report

osl-cli ──────┬── osl-kicad
              ├── osl-sim
              ├── osl-netlist ── osl-core
              ├── osl-model
              ├── osl-render
              ├── osl-waveform
              └── osl-report
```

---

### osl-core — 共享基础

```
crates/osl-core/
├── Cargo.toml
├── README.md
└── src/
    └── lib.rs              # 公共类型、错误处理、工具函数
```

零外部域依赖。所有其他 `osl-*` crate 都依赖它。

---

### osl-kicad — KiCad IR 与操作

```
crates/osl-kicad/
├── Cargo.toml
├── README.md
├── docs/
│   ├── file-split.md
│   └── schematic-impl-split.md
├── tests/
│   └── kicad_demo_smoke.rs        # 外部 KiCad 演示互操作测试
└── src/
    ├── lib.rs                      # Crate 根，公共 API 导出
    │
    ├── [文件解析]
    ├── sexpr.rs                    # S-expression 解析器
    ├── schematic_io.rs             # 读写 .kicad_sch 文件
    ├── symbols_parse_impl.rs       # 解析 .kicad_sym 符号库
    ├── symbol_library.rs           # 符号库索引和查找
    ├── library_index.rs            # sym-lib-table 解析
    ├── project.rs                  # .kicad_pro 项目文件解析
    │
    ├── [画布场景]
    ├── canvas.rs                   # KicadCanvasScene: 从原理图 IR 构建场景
    ├── canvas_items.rs             # 画布元素类型
    ├── canvas_items_graphic_impl.rs  # 图形元素
    ├── canvas_items_leaf_impl.rs     # 导线/标签/连接点/无连接
    ├── canvas_items_bounds_impl.rs   # 包围盒计算
    ├── canvas_hit.rs               # 碰撞检测
    │
    ├── [编辑操作]
    ├── edit.rs                     # KicadSchematicEdit: 编辑操作枚举
    ├── schematic_edit_impl.rs      # 编辑操作执行
    ├── schematic_edit_symbol_ops_impl.rs  # 符号放置/移动/删除
    ├── schematic_edit_wiring_ops_impl.rs  # 导线/总线/标签操作
    ├── schematic_util_impl.rs      # 原理图 IR 工具操作
    ├── schematic_check_impl.rs     # DRC/ERC 诊断
    ├── schematic_library_impl.rs   # 库操作
    ├── schematic_summary.rs        # GUI 显示用汇总统计
    │
    ├── [坐标与变换]
    ├── geometry.rs                 # 包围盒、碰撞检测、点在多边形内
    ├── transform.rs                # 坐标变换（镜像/旋转）
    ├── coordinates.rs              # KiCad 坐标系辅助
    │
    ├── [数据类型]
    ├── graphics.rs                 # 图形元素类型
    ├── symbols.rs                  # 符号定义类型
    ├── pins.rs                     # 引脚定义和形状
    ├── labels.rs                   # 网络标签
    ├── text.rs                     # 文本元素类型
    ├── sheet.rs                    # 层次化图纸类型
    ├── wiring.rs                   # 导线和总线类型
    ├── markers.rs                  # 连接点和无连接标记
    ├── group.rs, table.rs, image.rs, property.rs, instances.rs, metadata.rs, style.rs
    │
    ├── [高级功能]
    ├── connectivity.rs             # 网络连通性分析
    ├── spice_export.rs             # 从原理图生成 SPICE 网表
    ├── simulation.rs               # 仿真指令提取
    ├── diagnostics.rs              # 诊断消息类型
    ├── json.rs                     # JSON 序列化辅助
    └── util.rs                     # 内部工具函数
```

---

### osl-sim — 仿真后端

```
crates/osl-sim/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              # NgspiceCliBackend, XyceCliBackend, SimulatorBackend trait
    └── artifacts.rs        # 运行产物收集（.raw, .csv, 汇总）
```

---

### osl-netlist — 网表解析与导入

```
crates/osl-netlist/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
├── tests/
│   └── kicad_demo_import_smoke.rs  # 外部 KiCad 演示导入测试
└── src/
    ├── lib.rs                     # Crate 根
    ├── kicad_import.rs            # KiCad 网表导入
    ├── ltspice_import.rs          # LTspice 原理图导入
    ├── netlist_parse_impl.rs      # SPICE 网表解析器
    ├── netlist_suggest_impl.rs    # 信号建议引擎
    ├── ltspice_builtins_impl.rs   # LTspice 内建组件映射
    └── ltspice_types_impl.rs      # LTspice 类型定义
```

---

### osl-model — SPICE 模型检查

```
crates/osl-model/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs                     # Crate 根
    └── model_check_impl.rs        # .subckt/.model 验证、引脚映射、方言风险检测
```

---

### osl-waveform — 波形数据解析

```
crates/osl-waveform/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── lib.rs                     # Crate 根
    └── raw_parser_impl.rs         # .raw 文件解析器（二进制 + ASCII）
```

---

### osl-render — SVG 渲染器

```
crates/osl-render/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── lib.rs                     # Crate 根
    ├── svg_render_impl.rs         # 主 SVG 渲染管线
    └── svg_helpers_impl.rs        # SVG 辅助工具
```

---

### osl-report — 报告生成

```
crates/osl-report/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs                     # Crate 根
    ├── format.rs                  # 报告格式类型
    ├── json.rs                    # JSON 报告
    ├── html.rs                    # HTML 报告
    ├── junit.rs                   # JUnit XML 报告
    ├── markdown.rs                # Markdown 报告
    ├── bundle.rs                  # 多格式打包
    └── directory.rs               # 目录级报告输出
```

---

### osl-cli — 命令行界面

```
crates/osl-cli/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── main.rs                    # 入口，CLI 参数解析
    ├── kicad_edit.rs              # KiCad 编辑命令实现
    ├── cli_kicad.rs               # KiCad 子命令处理
    └── cli_verify.rs              # Verify 子命令处理
```

---

### osl-app — GUI 应用程序（114 个 .rs 文件）

```
crates/osl-app/
├── Cargo.toml
├── README.md
├── docs/                           # Crate 内部架构文档
│   ├── crate-structure.md
│   ├── ui-improvements.md
│   ├── schematic-bottom-dock.md
│   ├── simulation-profile-editor.md
│   └── context-menu-tool-palette.md
│
└── src/
    ├── main.rs                     # 入口点（原生窗口启动）
    ├── lib.rs                      # Crate 根
    ├── document.rs                 # KicadGuiDocument: 文档加载/保存/编辑
    ├── library.rs                  # KicadGuiLibrary: 符号库加载/查找
    ├── viewport.rs                 # CanvasViewport: 缩放/平移/坐标变换
    ├── simulation.rs               # 仿真任务分发（ngspice + Xyce）
    ├── simulation_run_loader.rs    # 运行输出加载器
    ├── waveform_summary.rs         # 波形汇总数据
    ├── report_summary.rs           # 报告汇总数据
    ├── placement_config.rs         # 符号放置配置
    ├── test_support.rs             # 测试辅助
    │
    ├── canvas.rs                   # 画布渲染管线
    └── canvas/
        ├── colors.rs               # 主题感知颜色定义
        └── primitives/             # 画布绘制图元
            ├── grid.rs             # 网格渲染
            ├── sheet.rs            # 图纸边界渲染
            ├── symbol.rs           # 符号渲染
            └── text.rs             # 文本渲染
```

#### app/ — 主应用程序模块

`app.rs` 定义 `NekoSpiceApp` 主结构体，子模块按工作区分组：

```
app/
├── app.rs                      # NekoSpiceApp 结构体、编辑操作、use 声明
│
├── [布局与面板]                    # 跨工作区的布局基础设施
├── panels.rs                   # 根布局：顶/底栏、导航、工作区
├── runtime.rs                  # eframe::App 实现
├── workspace_panel.rs          # 左/右面板调度
├── center_workspace.rs         # 中央工作区调度（路由到各工作区）
├── status_strip.rs             # 底部状态栏
├── studio_toolbar.rs           # 顶部工具栏
├── project_panel.rs            # 项目侧边栏
├── diagnostics_panel.rs        # 诊断面板
│
├── [导航与主题]
├── navigation.rs               # StudioWorkspace 枚举
├── navigation_panel.rs         # 左侧导航栏
├── theme.rs                    # 主题系统（Midnight/Graphite/Light）
│
├── [国际化]
├── localization.rs             # UiText 枚举
├── localization_en_impl.rs     # 英文翻译
├── localization_zh_impl.rs     # 中文翻译
│
├── [画布交互]
├── canvas_panel.rs             # 主画布：视口、鼠标交互、渲染
├── canvas_shortcuts.rs         # 键盘快捷键处理
├── canvas_context_menu.rs      # 右键上下文菜单
├── tool_palette.rs             # 垂直工具面板
├── shortcuts_overlay.rs        # 快捷键帮助叠加层
│
├── [放置与编辑]
├── placement.rs                # 放置状态管理
├── history.rs                  # 撤销/重做历史栈
├── file_dialog.rs              # 文件打开/保存对话框
├── preferences.rs              # 用户偏好
├── widgets.rs                  # 共享 UI 组件（metric_row 等）
│
├── home/                       # 首页工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── dashboard.rs            # 首页仪表板
│   ├── command_center.rs       # 快速操作中心
│   ├── insights_panel.rs       # AI 助手 + 洞察 + 快捷方式
│   ├── project_context.rs      # 项目上下文摘要
│   ├── sections.rs             # 首页分区布局
│   └── widgets.rs              # 首页专属组件
│
├── schematic/                  # 原理图编辑工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── workspace.rs            # 原理图视图主布局
│   ├── workspace_widgets.rs    # 工具栏按钮、文档标签
│   ├── bottom_dock.rs          # 底部停靠面板（波形/FFT/Bode/控制台/网表/ERC/检查器）
│   ├── review_panel.rs         # 原理图审查面板
│   ├── selection_properties.rs # 选中项属性编辑器
│   ├── symbol_placement.rs     # 符号放置 UI 控件
│   ├── tools/                  # 绘图工具状态机
│   │   ├── mod.rs              # 工具模块入口与预览调度
│   │   ├── state.rs            # SchematicTool 枚举和 SchematicToolState
│   │   ├── controls.rs         # 工具激活和切换逻辑
│   │   ├── editing.rs          # 导线/总线/标签/图纸创建
│   │   └── preview.rs          # 工具预览渲染
│   └── inspector/              # 右侧属性检查面板
│       ├── mod.rs              # 检查器模块入口
│       ├── panel.rs            # 检查面板主视图
│       ├── sections.rs         # 检查面板分区渲染
│       ├── simulator.rs        # 仿真器配置面板
│       └── widgets.rs          # 检查面板组件
│
├── simulation/                 # 仿真工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── workspace.rs            # 仿真工作区视图
│   ├── workspace_sections.rs   # 分区布局
│   ├── workspace_widgets.rs    # 组件
│   ├── panel.rs                # 仿真控制面板（含后端选择器 ngspice/Xyce）
│   ├── profile_editor.rs       # 分析配置编辑器
│   ├── profile_editor_options.rs # 配置选项渲染
│   ├── profile_editor_sections.rs # 配置分区渲染
│   ├── profile_editor_widgets.rs # 配置组件
│   ├── artifacts_panel.rs      # 仿真产物面板
│   ├── waveform_panel.rs       # 仿真波形面板
│   └── report_panel.rs         # 仿真报告面板
│
├── library/                    # 符号库工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── workspace.rs            # 符号库浏览器主视图
│   ├── data.rs                 # 库数据管理
│   ├── inspector.rs            # 库检查面板
│   ├── model_browser.rs        # 模型文件浏览器
│   ├── model_validation.rs     # 模型验证
│   ├── preview.rs              # 符号/模型预览渲染
│   ├── sections.rs             # 库分区布局
│   └── widgets.rs              # 库组件
│
├── waveform/                   # 波形查看工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── workspace.rs            # 波形查看器主视图
│   ├── workspace_sections.rs   # 分区布局
│   ├── workspace_widgets.rs    # 组件
│   ├── preview.rs              # 波形预览渲染（堆叠/单通道）
│   └── preview_primitives.rs   # 预览绘制图元（网格、零轴、桶渲染）
│
├── reports/                    # 报告工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── workspace.rs            # 报告查看器主视图
│   ├── measurements.rs         # 测量数据显示
│   ├── preview.rs              # 报告预览渲染
│   ├── sections.rs             # 报告分区布局
│   ├── state.rs                # ReportsTab 枚举和状态
│   └── widgets.rs              # 报告组件
│
├── review/                     # 设计审查工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── workspace.rs            # 设计审查主视图
│   ├── checklist.rs            # 审查清单管理
│   ├── state.rs                # 审查状态（严重性过滤等）
│   └── widgets.rs              # 审查组件
│
├── optimization/               # 参数优化工作区
│   ├── mod.rs                  # 模块声明与文档
│   ├── workspace.rs            # 优化工作区主视图
│   ├── sections.rs             # 优化分区布局
│   ├── state.rs                # OptimizationTab 枚举和状态
│   └── widgets.rs              # 优化组件
│
└── settings/                   # 设置工作区
    ├── mod.rs                  # 模块声明与文档
    ├── workspace.rs            # 设置/偏好主视图
    └── theme_preview.rs        # 主题预览渲染器
```

**模块层次说明**：
- `app/` 下的顶层 `.rs` 文件是跨工作区的基础设施（布局、导航、主题、画布交互等）
- 每个工作区子目录（`home/`、`schematic/`、`simulation/` 等）有独立的 `mod.rs`
- `schematic/tools/` 和 `schematic/inspector/` 是原理图模块的二级子目录
- 所有子目录内的方法使用 `pub(crate)` 可见性，确保从 `app.rs` 可调用
- 子目录文件通过 `crate::app::` 路径引用跨模块类型

## docs/ — 文档

```
docs/
├── README.md                 # 文档索引
├── dev.md                    # 开发者设置和构建说明
├── development-plan.md       # 架构概览和路线图
├── three-day-sprint.md       # 初始冲刺计划
└── ui/                       # UI 设计参考图片（已 gitignore）
    ├── ui-ref-01.png         # 原理图编辑器主页
    ├── ui-ref-02.png         # 仿真工作区
    ├── ui-ref-03.png         # 波形查看器
    ├── ui-ref-04.png         # 库浏览器
    ├── ui-ref-05.png         # 设计审查
    ├── ui-ref-06.png         # 报告查看器
    ├── ui-ref-07.png         # 设置页面
    ├── ui-ref-08.png         # 深色主题
    ├── ui-ref-09.png         # 浅色主题
    └── ui-ref-10.png         # 工具面板
```

---

## examples/ — 测试用例与演示

```
examples/
├── cm5_minima/               # CM5 Minima 演示板（默认 GUI 原理图）
├── kicad_schematic/          # RC 滤波器测试用例
├── kicad_hierarchical/       # 多层次化设计
├── kicad_project_schematic/  # KiCad 项目含原理图
├── kicad_project/            # 完整 KiCad 项目
├── kicad_import/             # KiCad 网表导入示例
├── ltspice_import/           # LTspice 原理图导入示例
├── rc_filter/                # 简单 RC 滤波器
├── rc_sweep/                 # RC 扫描分析
├── rlc_resonance/            # RLC 谐振
├── diode_rectifier/          # 二极管整流器
├── pin_mapping/              # 运放引脚映射测试
├── vendor_model_issues/      # SPICE 模型验证边界情况
├── basic_validation.osl.yaml # 验证计划示例
├── failing_validation.osl.yaml  # 预期失败验证计划
└── structured_validation.osl.yaml  # 结构化验证计划
```

---

## benchmarks/

```
benchmarks/
└── basic/
    └── basic.osl.yaml        # 基准测试配置
```

---

## runs/（已 gitignore）

```
runs/
└── gui/                      # GUI 仿真运行输出（自动生成）
```
