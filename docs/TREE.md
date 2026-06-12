# NekoSpice 项目结构

## Workspace Crates

```
NekoSpice/
├── Cargo.toml                    # Workspace 根配置
├── docs/
│   ├── README.md                 # 项目说明
│   ├── TREE.md                   # 本文件：项目结构说明
│   ├── dev.md                    # 开发指南
│   ├── development-plan.md       # 开发路线图
│   ├── three-day-sprint.md       # 三天冲刺计划
│   └── ui/                       # UI 参考图片
│       └── ui-ref-01..10.png
├── crates/
│   ├── osl-core/                 # 核心数据类型与工具
│   │   └── src/
│   │       ├── lib.rs            # OslResult, RunMetadata, Artifact, write_text 等
│   │       ├── error.rs          # 错误类型定义
│   │       ├── run_metadata.rs   # 仿真运行元数据
│   │       ├── measure.rs        # 测量与检查逻辑
│   │       └── ...               # 更多核心模块
│   │
│   ├── osl-kicad/                # KiCad 格式解析器（Rust 原生）
│   │   └── src/
│   │       ├── lib.rs            # S-expression 解析器、公有 API
│   │       ├── schematic.rs      # 原理图解析与写回
│   │       ├── canvas.rs         # 画布场景数据结构
│   │       ├── canvas_items.rs   # KicadCanvasSymbol, KicadCanvasSheet 等
│   │       ├── symbols.rs        # 符号库解析
│   │       ├── simulation.rs     # SPICE 仿真指令解析
│   │       ├── property.rs       # 属性解析与写回
│   │       ├── text.rs           # 文本效果解析
│   │       └── ...               # 更多解析模块
│   │
│   ├── osl-sim/                  # 仿真后端（ngspice/Xyce）
│   │   └── src/
│   │       ├── lib.rs            # SimulatorBackend trait
│   │       ├── ngspice.rs        # ngspice CLI 后端
│   │       ├── xyce.rs           # Xyce CLI 后端
│   │       ├── profile.rs        # SimulationProfile (18 options, 5 presets), 指令注入, .ic/.nodeset
│   │       ├── netlist.rs        # 网表验证与转换
│   │       └── ...
│   │
│   ├── osl-waveform/             # 波形数据解析（raw/csv）
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── raw_parser.rs     # ngspice binary/ASCII raw 解析
│   │       ├── csv_parser.rs     # CSV 波形解析
│   │       └── ...
│   │
│   ├── osl-report/               # HTML/JSON 报告生成
│   │   └── src/
│   │       ├── lib.rs
│   │       └── ...
│   │
│   ├── osl-model/                # TI/ADI 厂商模型目录
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── vendor_catalog.rs # 厂商模型扫描与搜索
│   │       └── ...
│   │
│   ├── osl-cli/                  # 命令行工具
│   │   └── src/
│   │       ├── main.rs           # CLI 入口
│   │       ├── run.rs            # osl run 子命令
│   │       ├── verify.rs         # osl verify 子命令
│   │       ├── bench.rs          # osl bench 子命令
│   │       ├── model_check.rs    # osl model-check 子命令
│   │       ├── import.rs         # osl import 子命令
│   │       └── waveform.rs       # osl waveform 子命令
│   │
│   └── osl-app/                  # GUI 应用（egui + wgpu）
│       └── src/
│           ├── main.rs           # 二进制入口
│           ├── lib.rs            # 库根，模块声明与常量
│           │
│           ├── app/              # ── 应用核心 ──
│           │   ├── app_ops.rs    # 编辑操作（加载/删除/移动/旋转/撤销/重做/保存）
│           │   ├── panels.rs     # 根面板布局调度（NekoSpiceApp → eframe::App）
│           │   ├── runtime.rs    # 原生窗口运行时入口
│           │   ├── preferences.rs # 用户偏好（主题/语言/求解器路径，磁盘持久化）
│           │   ├── history.rs    # 撤销/重做历史栈
│           │   ├── navigation.rs # 工作区枚举
│           │   │
│           │   ├── canvas_panel.rs       # 主画布面板（渲染/交互）
│           │   ├── canvas_shortcuts.rs   # 画布键盘快捷键
│           │   ├── canvas_context_menu.rs # 画布右键上下文菜单
│           │   ├── tool_palette.rs       # 垂直工具面板
│           │   │
│           │   ├── navigation_panel.rs   # 左侧导航栏
│           │   ├── project_panel.rs      # 项目侧边栏
│           │   ├── workspace_panel.rs    # 工作区面板路由
│           │   ├── center_workspace.rs   # 中央工作区调度
│           │   ├── status_strip.rs       # 底部状态栏
│           │   ├── studio_toolbar.rs     # 顶部工具栏
│           │   │
│           │   ├── file_dialog.rs        # 原生文件对话框
│           │   ├── placement.rs          # 符号放置状态机
│           │   ├── diagnostics_panel.rs  # 诊断面板
│           │   ├── shortcuts_overlay.rs  # 快捷键叠加层
│           │   ├── localization.rs       # 国际化框架
│           │   ├── localization_en_impl.rs # 英文翻译
│           │   ├── localization_zh_impl.rs # 中文翻译
│           │   ├── theme.rs              # 主题系统（Midnight/Graphite/Light）
│           │   └── widgets.rs            # 共享 UI 组件
│           │
│           │   ├── home/         # ── Home 工作区 ──
│           │   │   ├── dashboard.rs       # 首页仪表板
│           │   │   ├── command_center.rs  # 命令中心
│           │   │   ├── sections.rs        # 最近项目/求解器健康/测量/建议
│           │   │   ├── widgets.rs         # 首页专用组件
│           │   │   ├── templates.rs      # 模板网格（启动电路）
│           │   │   ├── quick_actions.rs  # 快捷操作按钮网格
│           │   │   ├── project_context.rs # 项目上下文摘要
│           │   │   └── insights_panel.rs  # 洞察面板
│           │   │
│           │   ├── schematic/    # ── 原理图工作区 ──
│           │   │   ├── workspace.rs       # 原理图中心工作区（布局编排）
│           │   │   ├── toolbar.rs         # 顶部工具栏（文件/编辑/缩放/绘图工具/DRC）
│           │   │   ├── document_tabs.rs   # 文档标签栏（原理图/子图纸）
│           │   │   ├── bottom_dock/       # 底部停靠面板（目录模块）
│           │   │   │   ├── mod.rs         # 标签页切换路由
│           │   │   │   ├── waveforms.rs   # 波形/FFT/Bode 标签页
│           │   │   │   └── debug.rs       # 控制台/网表/ERC/检查器标签页
│           │   │   ├── workspace_widgets.rs # 工作区组件
│           │   │   ├── selection_properties.rs # 选中项属性编辑器
│           │   │   ├── symbol_placement.rs # 符号放置 UI
│           │   │   ├── review_panel.rs    # 原理图审查面板
│           │   │   ├── tools/             # 绘图工具子模块
│           │   │   │   ├── state.rs       # 工具状态与枚举
│           │   │   │   ├── editing.rs     # 工具编辑逻辑（导线/标签/连接点等）
│           │   │   │   ├── controls.rs    # 工具参数控件
│           │   │   │   └── preview.rs     # 工具预览渲染
│           │   │   └── inspector/         # 属性检查器子模块
│           │   │       ├── panel.rs       # 检查器面板
│           │   │       ├── sections.rs    # 检查器分节
│           │   │       ├── simulator.rs   # 仿真相关检查
│           │   │       └── widgets.rs     # 检查器组件
│           │   │
│           │   ├── simulation/   # ── 仿真工作区 ──
│           │   │   ├── panel.rs           # 仿真右侧面板编排
│           │   │   ├── state.rs           # 后端选择 + AnalysisParams 结构化参数
│           │   │   ├── directive_editor.rs # 结构化分析参数编辑器
│           │   │   ├── run_controller.rs  # 仿真启动与轮询
│           │   │   ├── status_display.rs  # 运行结果/日志/警告查看
│           │   │   ├── workspace.rs       # 仿真中心工作区
│           │   │   ├── workspace_sections.rs # 工作区分节
│           │   │   ├── profile_editor.rs  # 仿真配置编辑器 + SimOptions
│           │   │   ├── profile_editor_options.rs # 配置编辑器编排 (35行)
│           │   │   ├── options_environment.rs # 温度设置 (26行)
│           │   │   ├── options_solver.rs   # 瞬态求解器+收敛+输出 (103行)
│           │   │   ├── options_ic.rs      # 初始条件 .ic/.nodeset (61行)
│           │   │   ├── options_status.rs  # 运行状态+最近运行 (90行)
│           │   │   ├── profile_editor_sections.rs # 分析+组件参数
│           │   │   ├── profile_editor_widgets.rs # 共享组件
│           │   │   ├── waveform_panel.rs  # 波形预览面板
│           │   │   ├── report_panel.rs    # 报告面板
│           │   │   ├── artifacts_panel.rs # 仿真产物面板
│           │   │   └── workspace_widgets.rs # 共享工作区组件
│           │   │
│           │   ├── library/      # ── 符号库工作区 ──
│           │   │   ├── workspace.rs       # 库中心工作区
│           │   │   ├── data.rs            # 库数据
│           │   │   ├── inspector.rs       # 库检查器
│           │   │   ├── model_browser.rs   # 模型浏览器
│           │   │   ├── model_validation.rs # 模型验证
│           │   │   ├── preview.rs         # 库预览
│           │   │   ├── sections.rs        # 库分节
│           │   │   ├── vendor_panel.rs    # 厂商模型面板（TI/ADI）
│           │   │   └── widgets.rs         # 库组件
│           │   │
│           │   ├── waveform/     # ── 波形工作区 ──
│           │   │   ├── workspace.rs
│           │   │   ├── workspace_sections.rs
│           │   │   ├── preview.rs         # 波形预览
│           │   │   └── ...
│           │   │
│           │   ├── optimization/ # ── 优化工作区 ──
│           │   ├── review/       # ── 审查工作区 ──
│           │   ├── reports/      # ── 报告工作区 ──
│           │   └── settings/     # ── 设置工作区 ──
│           │       ├── workspace.rs       # 设置中心工作区
│           │       └── theme_preview.rs   # 主题预览
│           │
│           ├── document.rs       # KiCad 原理图文档抽象（加载/保存/场景构建）
│           ├── document_ops.rs   # 文档编辑操作（删除/移动/旋转）
│           ├── canvas.rs         # 画布渲染管线（模块根，仅 re-export）
│           ├── canvas/
│           │   ├── colors.rs     # 主题感知的画布颜色定义
│           │   ├── scene_renderer.rs # 原理图场景渲染器（13 层）
│           │   ├── transforms.rs # 坐标变换工具
│           │   ├── hover.rs      # 悬停高亮
│           │   └── primitives/   # 底层绘制图元
│           │       ├── mod.rs    # 图元模块根
│           │       ├── grid.rs   # 网格绘制
│           │       ├── sheet.rs  # 图纸绘制
│           │       ├── symbol.rs # 符号图形绘制
│           │       └── text.rs   # 文本绘制
│           │
│           ├── simulation.rs     # 仿真任务分发器（GuiSimulationJob/Task/Run）
│           ├── simulation_run_loader.rs # 仿真结果加载器
│           ├── waveform_summary.rs # 波形摘要
│           ├── report_summary.rs # 报告摘要
│           ├── placement_config.rs # 符号放置配置
│           ├── viewport.rs       # 画布视口（缩放/平移/坐标变换）
│           └── test_support.rs   # 测试辅助工具
│
├── examples/                     # 测试用例和示例电路
├── runs/                         # 仿真运行输出目录
└── kicad-source-mirror-master/   # KiCad 源码参考（只读）
```

## 模块层次关系

```
NekoSpiceApp (app.rs)
  ├── app_ops      — 编辑操作
  ├── panels       — 根布局
  ├── home/        — 首页仪表板
  ├── schematic/   — 原理图编辑
  │   ├── tools/   — 绘图工具（导线/标签/总线/连接点等）
  │   └── inspector/ — 属性检查器
  ├── simulation/  — 仿真工作流 (18 SPICE options, presets, .ic/.nodeset)
  │   ├── directive_editor  — 指令编辑
  │   ├── run_controller    — 启动/轮询
  │   ├── profile_editor    — 求解器配置
  │   └── status_display    — 结果/日志显示
  ├── library/     — 符号库管理
  ├── waveform/    — 波形查看
  ├── optimization/ — 参数优化
  ├── review/      — 设计审查
  ├── reports/     — 报告生成
  └── settings/    — 应用设置

Canvas (canvas.rs)
  ├── scene_renderer — 13 层原理图渲染
  ├── transforms     — 坐标变换
  ├── hover          — 悬停高亮
  └── primitives/    — 网格/线条/图形/文本
```

## 关键工作流

### 仿真工作流
```
UI 指令编辑 (分析类型 + 参数)
  → Preset 选择 (default/fast/accurate/high-freq/convergence-help)
  → SimOptions 配置 (18 SPICE options: ITL1-5, TNOM, GMIN, CHGTOL, PIVTOL...)
  → .ic / .nodeset 初始条件
  → build_simulation_profile() → inject_profile_directives()
  → ensure_ngspice_control_exports() → ngspice/Xyce 执行
  → parse_ngspice_log() → 结果显示 + 波形预览
  → 设置持久化到 ~/.config/nekospice/settings.json
```

### 编辑工作流
```
用户操作 → handle_schematic_tool_click() → history.push(snapshot())
  → document.add_wire/label/junction/... → scene 重绘
```
