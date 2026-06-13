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
│   │       ├── lib.rs            # OslResult, RunMetadata, Artifact, write_text
│   │       ├── error.rs          # 错误类型定义
│   │       ├── run_metadata.rs   # 仿真运行元数据
│   │       └── measure.rs        # 测量与检查逻辑
│   │
│   ├── osl-kicad/                # KiCad 格式解析器（Rust 原生）
│   │   ├── src/
│   │   │   ├── lib.rs            # S-expression 解析器、公有 API
│   │   │   ├── schematic.rs      # 原理图解析与写回
│   │   │   ├── canvas.rs         # 画布场景数据结构
│   │   │   ├── canvas_items.rs   # KicadCanvasSymbol, KicadCanvasSheet 等
│   │   │   ├── symbols.rs        # 符号库解析
│   │   │   ├── simulation.rs     # SPICE 仿真指令解析（KicadSimulationDirectiveKind）
│   │   │   ├── property.rs       # 属性解析与写回
│   │   │   ├── text.rs           # 文本效果解析
│   │   │   ├── sexpr.rs          # S-expression 低层解析
│   │   │   ├── spice_export.rs   # KiCad → SPICE 网表导出
│   │   │   ├── tests.rs          # 26+ 单元测试
│   │   │   └── ...               # 更多解析模块
│   │   └── tests/
│   │       └── kicad_demo_smoke.rs
│   │
│   ├── osl-sim/                  # 仿真后端（ngspice/Xyce）
│   │   └── src/
│   │       ├── lib.rs            # SimulatorBackend trait
│   │       ├── profile.rs        # SimulationProfile, 指令注入, .ic/.nodeset
│   │       ├── artifacts.rs      # 仿真产物管理
│   │       └── netlist.rs        # 网表验证与转换
│   │
│   ├── osl-waveform/             # 波形数据解析（raw/csv/FFT）
│   │   └── src/
│   │       ├── lib.rs            # 波形数据结构、测量、CSV 导出
│   │       ├── raw_parser_impl.rs # ngspice binary/ASCII raw 解析
│   │       └── fft.rs            # FFT 计算（Cooley-Tukey）、窗函数、Bode
│   │
│   ├── osl-netlist/              # 网表解析与转换
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── kicad_import.rs
│   │       ├── ltspice_import.rs
│   │       ├── ltspice_builtins_impl.rs
│   │       ├── netlist_parse_impl.rs
│   │       └── netlist_suggest_impl.rs
│   │
│   ├── osl-render/               # SVG 渲染
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── svg_render_impl.rs
│   │       └── svg_helpers_impl.rs
│   │
│   ├── osl-report/               # HTML/JSON 报告生成
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── html.rs
│   │       ├── json.rs
│   │       ├── markdown.rs
│   │       └── junit.rs
│   │
│   ├── osl-model/                # TI/ADI 厂商模型目录
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── vendor_import.rs
│   │       └── model_check_impl.rs
│   │
│   ├── osl-cli/                  # 命令行工具
│   │   └── src/
│   │       ├── main.rs
│   │       ├── run.rs
│   │       ├── verify.rs
│   │       ├── bench.rs
│   │       ├── model_check.rs
│   │       ├── import.rs
│   │       └── waveform.rs
│   │
│   └── osl-app/                  # GUI 应用（egui + wgpu）
│       └── src/
│           ├── main.rs           # 二进制入口
│           ├── lib.rs            # 库根，模块声明与常量
│           ├── document.rs       # KicadGuiDocument 封装
│           ├── document_ops.rs
│           ├── library.rs        # KicadGuiLibrary 封装
│           ├── viewport.rs       # CanvasViewport 缩放/平移
│           ├── canvas.rs         # 画布渲染
│           ├── simulation.rs     # GuiSimulationRun 管理
│           ├── waveform_summary.rs # 波形摘要数据
│           ├── placement_config.rs # 符号放置配置
│           ├── report_summary.rs # 报告摘要
│           ├── test_support.rs   # 测试辅助
│           │
│           ├── canvas/           # ── 画布渲染层 ──
│           │   ├── colors.rs
│           │   ├── hover.rs
│           │   ├── scene_renderer.rs         # 编排 + 结构层（sheets, graphics, symbols）
│           │   ├── scene_renderer_wires.rs   # 连接层（wires, buses）
│           │   ├── scene_renderer_annotations.rs # 标注层（labels, text, junctions）
│           │   ├── transforms.rs
│           │   └── primitives/
│           │       ├── mod.rs
│           │       ├── grid.rs
│           │       ├── sheet.rs
│           │       ├── symbol.rs
│           │       └── text.rs
│           │
│           └── app/              # ── 应用核心 ──
│               ├── app_ops.rs    # 编辑操作（加载/删除/移动/旋转/撤销/重做/保存）
│               │                 #   + sync_sim_panel_from_schematic (指令自动同步)
│               ├── panels.rs     # 根面板布局调度
│               ├── runtime.rs    # 原生窗口运行时入口
│               ├── preferences.rs              # 用户偏好（主题/语言/求解器路径）
│               ├── preferences_persistence.rs  # 持久化 JSON 结构（SettingsFile）
│               ├── history.rs    # 撤销/重做历史栈
│               ├── navigation.rs # StudioWorkspace 枚举
│               │
│               ├── canvas_panel.rs       # 主画布面板
│               ├── canvas_shortcuts.rs   # 画布键盘快捷键（F5=Run, Ctrl+S=Save）
│               ├── canvas_context_menu.rs # 画布右键上下文菜单
│               ├── tool_palette.rs       # 垂直工具面板
│               │
│               ├── navigation_panel.rs   # 左侧导航栏
│               ├── project_panel.rs      # 项目侧边栏
│               ├── workspace_panel.rs    # 工作区面板路由
│               ├── center_workspace.rs   # 中央工作区调度
│               ├── status_strip.rs       # 底部状态栏
│               ├── studio_toolbar.rs     # 顶部工具栏
│               │
│               ├── file_dialog.rs        # 原生文件对话框
│               ├── shortcuts_overlay.rs  # 快捷键帮助叠加层
│               ├── diagnostics_panel.rs  # ERC/DRC 诊断面板
│               ├── placement.rs          # 符号放置状态
│               ├── widgets.rs            # 通用 UI 组件
│               ├── theme.rs              # StudioTheme 主题系统
│               ├── locale.rs              # 语言区域选择（StudioLocale）
│               ├── localization.rs       # 多语言支持（中/英）
│               ├── localization_en_impl.rs
│               ├── localization_zh_impl.rs
│               │
│               ├── schematic/            # ── 原理图工作区 ──
│               │   ├── mod.rs
│               │   ├── workspace.rs      # 中央画布渲染
│               │   ├── workspace_widgets.rs
│               │   ├── toolbar.rs        # 原理图顶部工具栏
│               │   ├── document_tabs.rs  # 文档标签页
│               │   ├── selection_properties.rs # 属性编辑器
│               │   ├── symbol_placement.rs # 符号放置交互
│               │   ├── review_panel.rs   # 审查面板
│               │   ├── inspector/        # 属性检查器面板
│               │   │   ├── mod.rs
│               │   │   ├── panel.rs
│               │   │   ├── sections.rs
│               │   │   ├── simulator.rs
│               │   │   └── widgets.rs
│               │   ├── bottom_dock/      # 底部停靠面板
│               │   │   ├── mod.rs
│               │   │   ├── waveforms.rs
│               │   │   └── debug.rs
│               │   └── tools/            # 原理图编辑工具
│               │       ├── mod.rs
│               │       ├── state.rs
│               │       ├── controls.rs
│               │       ├── editing.rs
│               │       └── preview.rs
│               │
│               ├── simulation/           # ── 仿真工作区（25 文件）──
│               │   ├── mod.rs
│               │   ├── analysis.rs       # AnalysisParams + StepSweep（结构化分析参数）
│               │   ├── state.rs          # SimulationPanelState + SimulationBackendKind
│               │   ├── panel.rs          # 右侧仿真面板调度
│               │   ├── directive_editor.rs # 分析指令编辑器 UI
│               │   ├── run_controller.rs # Profile 构建、运行启动、任务轮询
│               │   ├── status_display.rs # 运行结果、日志查看器
│               │   ├── workspace.rs      # 中央仿真工作区（概览 + 配置编辑器 tab）
│               │   ├── workspace_sections.rs # 概览分区（分析设置、网表、运行输出）
│               │   ├── workspace_widgets.rs  # 共享 UI 组件
│               │   ├── profile_editor.rs # 三列 Profile 编辑器 + SimOptions
│               │   ├── profile_editor_options.rs # 右列选项调度
│               │   ├── profile_editor_sections.rs # 左/中列分析 + 参数
│               │   ├── profile_editor_widgets.rs  # 编辑器共享组件
│               │   ├── options_environment.rs     # 温度设置
│               │   ├── options_solver.rs          # 瞬态求解器 + 收敛
│               │   ├── options_ic.rs              # .ic/.nodeset 初始条件
│               │   ├── options_status.rs          # 运行状态 + 最近运行
│               │   ├── step_sweep_editor.rs       # .step 参数扫描编辑器
│               │   ├── measure_editor.rs          # .measure 指令编辑器
│               │   ├── history.rs                 # SimulationHistory（最近 20 次运行）
│               │   ├── history_panel.rs           # 历史表格显示
│               │   ├── waveform_panel.rs          # 波形预览卡片
│               │   ├── report_panel.rs            # 报告摘要
│               │   └── artifacts_panel.rs         # 仿真产物
│               │
│               ├── waveform/             # ── 波形分析工作区（10 文件）──
│               │   ├── mod.rs
│               │   ├── workspace.rs      # 状态（tabs, viewport, overlay_mode, visible_signals）
│               │   ├── workspace_sections.rs # 测量表格、导出、光标面板
│               │   ├── workspace_widgets.rs  # trace_chip, trace_chip_toggle, 表格
│               │   ├── preview.rs        # 静态单/堆叠波形预览
│               │   ├── interactive.rs    # 缩放/平移/光标交互绘图
│               │   ├── helpers.rs        # 共享辅助（轨迹排序、标签）
│               │   ├── preview_primitives.rs   # 网格、桶、轨迹颜色
│               │   ├── freq_domain_primitives.rs # 对数频率映射、幅值/相位轨迹
│               │   └── freq_domain_preview.rs   # FFT/Bode/Noise 绘图函数
│               │
│               ├── home/                 # ── 首页工作区 ──
│               │   ├── mod.rs
│               │   ├── dashboard.rs      # 项目仪表板
│               │   ├── command_center.rs # 命令中心
│               │   ├── insights_panel.rs # 洞察面板
│               │   ├── project_context.rs
│               │   ├── quick_actions.rs
│               │   ├── sections.rs
│               │   ├── templates.rs
│               │   └── widgets.rs
│               │
│               ├── library/              # ── 符号库工作区 ──
│               │   ├── mod.rs
│               │   ├── workspace.rs
│               │   ├── data.rs           # 库数据 + vendor 模型管理
│               │   ├── inspector.rs
│               │   ├── model_browser.rs
│               │   ├── model_validation.rs
│               │   ├── preview.rs
│               │   ├── sections.rs
│               │   ├── vendor_panel.rs   # TI/ADI 厂商模型浏览 + 目录选择器
│               │   └── widgets.rs
│               │
│               ├── optimization/         # ── 参数优化工作区 ──
│               │   ├── mod.rs
│               │   ├── state.rs
│               │   ├── workspace.rs
│               │   ├── sections.rs
│               │   └── widgets.rs
│               │
│               ├── review/               # ── 设计审查工作区 ──
│               │   ├── mod.rs
│               │   ├── state.rs
│               │   ├── checklist.rs
│               │   ├── workspace.rs              # 中心画布 + 辅助方法
│               │   ├── workspace_panel.rs      # 侧边面板（操作按钮 + 风险快照）
│               │   └── widgets.rs
│               │
│               ├── reports/              # ── 报告工作区 ──
│               │   ├── mod.rs
│               │   ├── state.rs
│               │   ├── workspace.rs
│               │   ├── sections.rs
│               │   ├── measurements.rs
│               │   ├── preview.rs
│               │   └── widgets.rs
│               │
│               └── settings/             # ── 设置工作区 ──
│                   ├── mod.rs
│                   ├── workspace.rs
│                   └── theme_preview.rs
```

## Architecture Notes

### Simulation Flow
```
UI directive_kind + analysis_params.to_body()
  → build_simulation_profile()
  → inject_profile_directives()
  → ensure_ngspice_control_exports()
  → ngspice/Xyce CLI executes
  → parse_ngspice_log() on failure
  → display in status panel
```

### Schematic Loading
```
load_schematic(path)
  → KicadGuiDocument::load()
  → sync_sim_panel_from_schematic()
    → parse .tran/.ac/.dc/.op/.noise/.disto/.sens directives
    → parse .step sweep configuration
    → fill AnalysisParams fields from directive body
```

### Key Design Principles
- **解耦**: 每个工作区独立子模块，通过 `app/` 下的 `impl NekoSpiceApp` 扩展
- **文件管理**: 单文件不超过 300 行，超过时拆分为独立模块
- **状态分离**: `state.rs` 定义状态结构，`analysis.rs` 定义参数逻辑
- **UI 组件**: `*_widgets.rs` 提供无状态渲染函数
- **双向兼容**: 兼容 KiCad 原理图格式，支持 ngspice + Xyce 双后端

### Build & Test
```bash
cargo build -p osl-app          # 构建 GUI
cargo test --workspace           # 运行所有测试（171 预期）
cargo run -p osl-app             # 启动 GUI
```
