# NekoSpice 项目结构

> 本文档描述 NekoSpice 项目的完整文件和目录结构。

---

## 根目录

```
NekoSpice/
├── Cargo.toml              # Rust workspace 根配置（10 个 crate）
├── Cargo.lock              # 依赖锁定文件
├── README.md               # 项目说明（中文）
├── .gitignore              # Git 忽略规则
├── crates/                 # 所有 Rust 源代码
├── docs/                   # 文档与 UI 参考图片
├── examples/               # 测试用例和演示工程
└── scripts/                # 工具脚本
```

---

## crates/ — 工作区模块

### 架构分层

```
┌─────────────────────────────────────────────────────────┐
│                  NekoSpice GUI (egui + wgpu)             │
│                  NekoSpice CLI (命令行工具)               │
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

### 依赖关系

```
nsp-app ──────┬── nsp-schema ── nsp-core
              ├── nsp-sim ────── nsp-core
              ├── nsp-render ── nsp-schema
              ├── nsp-waveform
              └── nsp-report

nsp-cli ──────┬── nsp-schema
              ├── nsp-sim
              ├── nsp-netlist ── nsp-core
              ├── nsp-model
              ├── nsp-render
              ├── nsp-waveform
              └── nsp-report
```

---

### nsp-core — 核心基础

```
crates/nsp-core/
├── Cargo.toml
├── README.md
└── src/
    └── lib.rs              # 公共类型、错误处理、工具函数
```

零外部域依赖。所有其他 `nsp-*` crate 都依赖它。

---

### nsp-schema — 原理图引擎

```
crates/nsp-schema/
├── Cargo.toml
├── README.md
├── docs/
│   ├── schematic-impl-split.md    # 拆分说明
│   └── file-split.md             # 文件拆分说明
├── src/
│   ├── lib.rs                     # 模块声明与公共 API
│   ├── sexpr.rs                   # S-expression 解析器
│   ├── schematic_io.rs            # 原理图文件读写
│   ├── schematic_edit_impl.rs     # 编辑操作实现
│   ├── schematic_edit_symbol_ops_impl.rs   # Symbol 编辑操作
│   ├── schematic_edit_wiring_ops_impl.rs   # 导线编辑操作
│   ├── schematic_check_impl.rs    # ERC 检查
│   ├── schematic_util_impl.rs     # 辅助工具
│   ├── schematic_library_impl.rs  # 库管理实现
│   ├── schematic_summary.rs       # 摘要生成
│   ├── symbols.rs                 # Symbol 数据结构
│   ├── symbols_parse_impl.rs      # Symbol 解析实现
│   ├── symbol_library.rs          # 符号库管理
│   ├── library_index.rs           # 符号库索引
│   ├── canvas.rs                  # 画布场景数据
│   ├── canvas_hit.rs              # 画布命中测试
│   ├── canvas_items.rs            # 画布图元
│   ├── canvas_items_bounds_impl.rs    # 图元边界计算
│   ├── canvas_items_graphic_impl.rs   # 图形图元实现
│   ├── canvas_items_leaf_impl.rs      # 叶子图元实现
│   ├── connectivity.rs            # 连接图
│   ├── geometry.rs                # 几何计算
│   ├── coordinates.rs             # 坐标系统
│   ├── transform.rs               # 变换矩阵
│   ├── graphics.rs                # 图形属性
│   ├── style.rs                   # 样式定义
│   ├── wiring.rs                  # 导线数据
│   ├── labels.rs                  # 标签数据
│   ├── pins.rs                    # 引脚数据
│   ├── property.rs                # 属性系统
│   ├── text.rs                    # 文本数据
│   ├── table.rs                   # 表格数据
│   ├── sheet.rs                   # 子图纸
│   ├── group.rs                   # 分组
│   ├── image.rs                   # 嵌入图片
│   ├── markers.rs                 # 标记
│   ├── metadata.rs                # 元数据
│   ├── instances.rs               # 实例管理
│   ├── diagnostics.rs             # 诊断信息
│   ├── edit.rs                    # 编辑操作 DTO
│   ├── json.rs                    # JSON 序列化
│   ├── project.rs                 # 项目文件解析
│   ├── simulation.rs              # 仿真配置
│   ├── spice_export.rs            # SPICE 网表导出
│   ├── new_schematic.rs           # 新建原理图
│   ├── util.rs                    # 通用工具
│   └── tests.rs                   # 单元测试
└── tests/
    └── schema_demo_smoke.rs       # 演示工程冒烟测试
```

**职责**: 原理图格式的 S-expression 解析、Rust IR、画布场景、
命中测试、编辑操作、ERC 检查、SPICE 网表导出。

---

### nsp-sim — 仿真后端

```
crates/nsp-sim/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              # 后端 trait、ngspice/Xyce CLI 集成
    ├── profile.rs          # 仿真配置构建
    └── artifacts.rs        # 仿真产物处理
```

**职责**: 仿真配置注入、求解器调用、日志解析、产物管理。

---

### nsp-waveform — 波形解析

```
crates/nsp-waveform/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── lib.rs              # 波形数据结构与 API
    ├── raw_parser_impl.rs  # Raw 文件解析
    └── fft.rs              # FFT 频域分析
```

**职责**: ngspice raw/CSV 波形解析、FFT/Bode/噪声分析、
百万点波形的高效查询。

---

### nsp-netlist — 网表处理

```
crates/nsp-netlist/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
├── src/
│   ├── lib.rs                  # 网表数据结构
│   ├── netlist_parse_impl.rs   # 网表解析实现
│   ├── netlist_suggest_impl.rs # 网表建议
│   ├── schema_import.rs        # 原理图导入
│   ├── ltspice_import.rs       # LTspice 格式导入
│   ├── ltspice_builtins_impl.rs # LTspice 内置函数
│   └── ltspice_types_impl.rs   # LTspice 类型映射
└── tests/
    └── schema_demo_import_smoke.rs
```

**职责**: SPICE/LTspice 网表解析、格式转换、导入兼容性。

---

### nsp-render — SVG 渲染

```
crates/nsp-render/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── lib.rs                  # 渲染 API
    ├── svg_render_impl.rs      # SVG 渲染实现
    └── svg_helpers_impl.rs     # SVG 辅助函数
```

**职责**: 原理图到 SVG 的高质量渲染。

---

### nsp-report — 报告生成

```
crates/nsp-report/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs          # 报告数据结构
    ├── format.rs       # 格式定义
    ├── html.rs         # HTML 报告
    ├── json.rs         # JSON 报告
    ├── markdown.rs     # Markdown 报告
    ├── junit.rs        # JUnit XML 报告
    ├── bundle.rs       # 报告打包
    └── directory.rs    # 目录管理
```

**职责**: 仿真结果的多格式报告生成。

---

### nsp-model — 厂商模型

```
crates/nsp-model/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs              # 模型数据结构
    ├── vendor_import.rs    # TI/ADI 模型导入
    └── model_check_impl.rs # 模型检查
```

**职责**: TI/ADI SPICE 模型库管理、子电路注入。

---

### nsp-cli — 命令行工具

```
crates/nsp-cli/
├── Cargo.toml
├── README.md
├── docs/
│   └── file-split.md
└── src/
    ├── main.rs           # CLI 入口与子命令路由
    ├── cli_schema.rs     # 原理图子命令
    ├── cli_verify.rs     # 验证子命令
    └── schema_edit.rs    # 编辑操作解析
```

**职责**: 批量仿真、验证、原理图检查的命令行接口。

---

### nsp-app — GUI 应用

```
crates/nsp-app/
├── Cargo.toml
├── README.md
├── docs/
│   ├── crate-structure.md
│   └── schematic-bottom-dock.md
└── src/
    ├── main.rs                 # 二进制入口
    ├── lib.rs                  # 库根，模块声明
    ├── app.rs                  # 应用状态根模块
    ├── document.rs             # NspGuiDocument 封装
    ├── document_ops.rs         # 编辑操作
    ├── library.rs              # NspGuiLibrary 封装
    ├── viewport.rs             # 画布视口（缩放/平移）
    ├── simulation.rs           # GuiSimulationRun 管理
    ├── simulation_run_loader.rs # 仿真运行加载
    ├── placement_config.rs     # 放置配置
    ├── report_summary.rs       # 报告摘要
    ├── waveform_summary.rs     # 波形摘要
    ├── test_support.rs         # 测试辅助
    ├── canvas/                 # 画布渲染层
    │   ├── colors.rs           # 颜色定义
    │   ├── hover.rs            # 悬停检测
    │   ├── transforms.rs       # 视口变换
    │   ├── scene_renderer.rs   # 场景渲染器
    │   ├── scene_renderer_annotations.rs # 标注渲染
    │   ├── scene_renderer_wires.rs       # 导线渲染
    │   └── primitives/         # 渲染图元
    │       ├── grid.rs         # 网格
    │       ├── symbol.rs       # Symbol 图元
    │       ├── wire.rs         # 导线图元
    │       ├── text.rs         # 文本图元
    │       └── sheet.rs        # 子图纸图元
    ├── app/                    # 应用状态与 UI
    │   ├── panels.rs           # 面板管理
    │   ├── placement.rs        # Symbol 放置
    │   ├── runtime.rs          # 运行时状态
    │   ├── canvas_panel.rs     # 画布面板
    │   ├── canvas_context_menu.rs # 右键菜单
    │   ├── canvas_shortcuts.rs    # 快捷键
    │   ├── center_workspace.rs    # 中心工作区
    │   ├── diagnostics_panel.rs   # 诊断面板
    │   ├── file_dialog.rs         # 文件对话框
    │   ├── history.rs             # 撤销/重做
    │   ├── localization.rs        # 国际化
    │   ├── navigation.rs          # 导航
    │   ├── preferences.rs         # 偏好设置
    │   ├── theme.rs               # 主题
    │   ├── widgets.rs             # 通用组件
    │   ├── home/                  # 首页工作区
    │   │   ├── mod.rs
    │   │   ├── dashboard.rs       # 仪表板
    │   │   ├── command_center.rs  # 命令中心
    │   │   ├── quick_actions.rs   # 快捷操作
    │   │   ├── templates.rs       # 模板
    │   │   ├── sections.rs        # 区块
    │   │   ├── widgets.rs         # 组件
    │   │   ├── insights_panel.rs  # 洞察面板
    │   │   └── project_context.rs # 项目上下文
    │   ├── schematic/             # 原理图编辑器
    │   │   ├── mod.rs
    │   │   ├── workspace.rs       # 工作区
    │   │   ├── workspace_widgets.rs # 工作区组件
    │   │   ├── toolbar.rs         # 工具栏
    │   │   ├── selection_properties.rs # 选择属性
    │   │   ├── symbol_placement.rs     # Symbol 放置
    │   │   ├── document_tabs.rs        # 文档标签
    │   │   ├── review_panel.rs         # 审查面板
    │   │   ├── inspector/         # 检查器
    │   │   │   ├── mod.rs
    │   │   │   ├── panel.rs
    │   │   │   ├── sections.rs
    │   │   │   ├── simulator.rs
    │   │   │   └── widgets.rs
    │   │   ├── tools/             # 绘图工具
    │   │   │   ├── mod.rs
    │   │   │   ├── state.rs       # 工具状态
    │   │   │   ├── controls.rs    # 工具控件
    │   │   │   ├── editing.rs     # 编辑操作
    │   │   │   └── preview.rs     # 工具预览
    │   │   └── bottom_dock/       # 底部面板
    │   │       ├── mod.rs
    │   │       ├── waveforms.rs   # 波形
    │   │       └── debug.rs       # 调试
    │   ├── library/               # 符号库浏览
    │   │   ├── mod.rs
    │   │   ├── workspace.rs
    │   │   ├── data.rs
    │   │   ├── inspector.rs
    │   │   ├── preview.rs
    │   │   ├── sections.rs
    │   │   ├── widgets.rs
    │   │   ├── model_browser.rs
    │   │   ├── model_validation.rs
    │   │   └── vendor_panel.rs
    │   ├── simulation/            # 仿真配置
    │   │   ├── mod.rs
    │   │   ├── workspace.rs
    │   │   ├── panel.rs
    │   │   ├── profile_editor.rs
    │   │   ├── analysis.rs
    │   │   ├── directive_editor.rs
    │   │   ├── measure_editor.rs
    │   │   ├── history.rs
    │   │   ├── history_panel.rs
    │   │   ├── artifacts_panel.rs
    │   │   ├── export_panel.rs
    │   │   ├── field_validation.rs
    │   │   ├── options_*.rs       # 仿真选项
    │   │   └── ...
    │   ├── optimization/          # 参数优化
    │   │   ├── mod.rs
    │   │   ├── workspace.rs
    │   │   ├── state.rs
    │   │   ├── widgets.rs
    │   │   └── sections/
    │   │       ├── mod.rs
    │   │       ├── targets.rs
    │   │       ├── sweep.rs
    │   │       └── monte_carlo.rs
    │   ├── review/                # 设计审查
    │   │   ├── mod.rs
    │   │   ├── workspace.rs
    │   │   ├── workspace_panel.rs
    │   │   ├── checklist.rs
    │   │   ├── state.rs
    │   │   └── widgets.rs
    │   ├── waveform/              # 波形分析
    │   │   ├── mod.rs
    │   │   ├── workspace.rs
    │   │   ├── workspace_*.rs
    │   │   ├── preview.rs
    │   │   ├── interactive.rs
    │   │   ├── helpers.rs
    │   │   └── ...
    │   ├── reports/               # 报告生成
    │   │   ├── mod.rs
    │   │   ├── workspace.rs
    │   │   ├── export.rs
    │   │   ├── preview.rs
    │   │   ├── measurements.rs
    │   │   ├── sections.rs
    │   │   ├── state.rs
    │   │   └── widgets.rs
    │   └── settings/              # 应用设置
    │       ├── mod.rs
    │       ├── workspace.rs
    │       └── theme_preview.rs
    └── document_tests/            # 文档测试
        ├── mod.rs
        ├── items.rs
        ├── editing.rs
        ├── placement.rs
        └── simulation.rs
```

---

## examples/ — 示例工程

```
examples/
├── cm5_minima/                    # CM5 最小系统板
│   ├── CM5.nsp_sch             # 原理图
│   ├── CM5IO.nsp_sym           # 符号库
│   └── sym-lib-table             # 符号库表
├── schema_schematic/              # 基础原理图示例
│   ├── rc.nsp_sch              # RC 滤波器
│   ├── neko_spice.nsp_sym      # NekoSpice 符号库
│   └── sym-lib-table
├── schema_hierarchical/           # 层次化设计示例
│   ├── hierarchical.nsp_pro  # 项目文件
│   ├── hierarchical.nsp_sch  # 顶层原理图
│   ├── gain_stage.nsp_sch      # 子图纸
│   └── sym-lib-table
├── schema_project/                # 项目文件示例
│   ├── project.cir         # SPICE 网表
│   ├── project.nsp_pro   # 项目文件
│   └── models/                   # 模型文件
├── schema_project_schematic/      # 完整项目示例
│   ├── project_schematic.nsp_pro
│   ├── project_schematic.nsp_sch
│   └── sym-lib-table
├── schema_import/                 # 导入示例
│   ├── nsp_diode_include.cir
│   ├── nsp_rc.cir
│   └── models/
├── ltspice_import/                # LTspice 导入示例
│   ├── ltspice_rc.asc
│   ├── ltspice_bjt.asc
│   └── ...
├── diode_rectifier/               # 二极管整流器
├── rc_filter/                     # RC 滤波器
├── rc_sweep/                      # RC 参数扫描
├── rlc_resonance/                 # RLC 谐振
├── pin_mapping/                   # 引脚映射测试
├── vendor_model_issues/           # 厂商模型问题测试
├── basic_validation.osl.yaml      # 基础验证配置
├── failing_validation.osl.yaml    # 失败验证配置
└── structured_validation.osl.yaml # 结构化验证配置
```

---

## docs/ — 文档

```
docs/
├── README.md               # 文档索引
├── TREE.md                 # 本文件：项目结构说明
├── dev.md                  # 开发指南
├── development-plan.md     # 开发路线图
├── USER_MANUAL.md          # 用户手册
├── three-day-sprint.md     # 三天冲刺计划
└── ui/                     # UI 参考图片
    ├── ui-ref-01.png       # 首页仪表板
    ├── ui-ref-02.png       # 原理图编辑器
    ├── ui-ref-03.png       # 符号库浏览
    ├── ui-ref-04.png       # 仿真配置
    ├── ui-ref-05.png       # 参数优化
    ├── ui-ref-06.png       # 设计审查
    ├── ui-ref-07.png       # 波形分析
    ├── ui-ref-08.png       # 报告生成
    ├── ui-ref-09.png       # 应用设置
    └── ui-ref-10.png       # 原理图详情
```

---

## scripts/ — 工具脚本

```
scripts/
├── screenshot.sh           # 一键截图（启动软件 + 截图 + 关闭）
└── run_and_screenshot.sh   # 运行并截图
```

---

## 核心设计原则

1. **解耦**: 每个工作区独立子模块，职责单一
2. **文件管理**: 单文件不超过 300 行，超过则拆分
3. **状态分离**: state.rs 定义状态，options.rs 定义参数
4. **双后端**: 兼容 ngspice + Xyce
5. **格式兼容**: 兼容标准原理图 S-expression 格式读写
6. **硬件加速**: egui + wgpu 渲染引擎
