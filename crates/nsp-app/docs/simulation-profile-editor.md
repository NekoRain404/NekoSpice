# Simulation Profile Editor 模块说明

## 概述

仿真配置编辑器（Simulation Profile Editor）是仿真工作区的子视图，提供三栏布局：
- **左栏**：分析设置 + 元件参数表 + 模型参数表
- **中栏**：参数定义编辑器（name/value/unit 可编辑表格）
- **右栏**：仿真选项 + 运行状态 + 最近运行记录

## 文件结构

```
crates/osl-app/src/app/
├── simulation_workspace.rs                    # 仿真工作区入口（含子视图切换）
├── simulation_profile_editor.rs               # 配置编辑器状态与主绘制
├── simulation_profile_editor_sections.rs      # 左栏与中栏面板
├── simulation_profile_editor_options.rs       # 右栏选项面板
└── simulation_profile_editor_widgets.rs       # 共享组件（表格、标题、状态标签）
```

## 各文件职责

### simulation_profile_editor.rs
- `SimulationSubView` 枚举 — Overview / ProfileEditor 子视图切换
- `SimulationProfileEditorState` — 编辑器状态（组件参数、模型参数、仿真选项）
- `SimOptions` — 可编辑的仿真参数（温度、最大迭代、最小步长、容差）
- `draw_profile_editor` — 三栏布局入口

### simulation_profile_editor_sections.rs
- `draw_analysis_setup_panel` — 分析模式选择（.tran/.ac/.dc/.op）
- `draw_component_params` — 元件参数可编辑表格
- `draw_model_params` — 模型参数可编辑表格
- `draw_parameter_definitions` — 参数定义编辑器（center column）
- `load_rc_template` / `load_opamp_template` — 预设模板加载

### simulation_profile_editor_options.rs
- `draw_simulation_options` — 仿真选项（温度、迭代、步长、容差）
- `draw_run_status_summary` — 运行状态摘要
- `draw_recent_runs` — 最近运行记录

### simulation_profile_editor_widgets.rs
- `section_header` — 面板标题渲染
- `param_table` — 通用可编辑参数表格（支持行删除）
- `status_pill` — 状态徽章组件

## 本地化

所有 UI 文字通过 `UiText` 枚举管理，支持 English 和 SimplifiedChinese。
新增键：`SimulationProfileEditor`、`SimulationOverview`、`ComponentParameters`、
`ModelParameters`、`SimulationOptions`、`RunStatus`、`RecentRuns`。

## 集成方式

1. `simulation_workspace.rs` 中通过 `draw_simulation_sub_view_tabs` 提供 Overview/ProfileEditor 切换
2. `simulation_profile_editor.sub_view` 控制当前子视图
3. `NekoSpiceApp` 持有 `simulation_profile_editor: SimulationProfileEditorState`
