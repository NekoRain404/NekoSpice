//! NekoSpice 应用程序主模块。定义 [`NekoSpiceApp`] 核心结构体、枚举类型和模块层次。
//!
//! 编辑操作（移动、旋转、删除、撤销/重做、文件加载）已拆分至 [`app_ops`] 模块。
//!
use crate::DEFAULT_GUI_SCHEMATIC;
use crate::document::NspGuiDocument;
use crate::library::NspGuiLibrary;
use crate::placement_config::SymbolPlacementConfig;
use crate::viewport::CanvasViewport;
use nsp_schema::{NspCanvasHit, NspCanvasScene};

// ── 编辑操作（从本文件拆分）────────────────────────────────────────────
mod app_ops;
mod app_sim_sync;

// ── Cross-cutting app modules (stay at app level) ──────────────────────
mod canvas_context_menu;
mod canvas_panel;
mod canvas_shortcuts;
mod center_workspace;
mod clipboard;
mod diagnostics_panel;
mod file_dialog;
mod global_shortcuts;
mod history;
mod locale;
mod localization;
mod navigation;
mod navigation_panel;
mod panels;
mod placement;
mod preferences;
mod preferences_persistence;
mod project_panel;
mod runtime;
mod shortcuts_overlay;
mod status_strip;
mod studio_toolbar;
pub(crate) mod theme;
mod tool_palette;
mod widgets;
mod workspace_panel;

// ── Workspace sub-modules ──────────────────────────────────────────────
mod home;
mod library;
mod optimization;
mod reports;
mod review;
mod schematic;
mod settings;
mod simulation;
mod waveform;

pub use canvas_panel::load_canvas_scene;
use navigation::StudioWorkspace;
use optimization::OptimizationWorkspaceState;
use placement::SymbolPlacementState;
use preferences::StudioPreferences;
use reports::ReportsWorkspaceState;
use review::ReviewWorkspaceState;
pub use runtime::{run_native, run_native_with_boxed};
use schematic::SelectionPropertyEditorState;
use schematic::inspector::SchematicInspectorPanelState;
use schematic::tools::SchematicToolState;
use simulation::SimulationHistory;
use simulation::SimulationPanelState;
use simulation::SimulationProfileEditorState;
use simulation::measure_editor::MeasureEntry;
use simulation::options_xyce::XyceOptions;
use simulation::run_compare::RunCompareState;
use waveform::WaveformWorkspaceState;

const EDIT_NUDGE_MM: f64 = 2.54;

/// 原理图工作区底部停靠面板的活跃标签页。
///
/// 切换波形预览、FFT 分析、波特图、控制台输出、网表、ERC 检查和属性检查器。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum SchematicBottomTab {
    #[default]
    Waveforms,
    Fft,
    Bode,
    Console,
    Netlist,
    Erc,
    Inspector,
}

/// NekoSpice 应用程序核心状态。
///
/// 持有当前文档、库、画布场景、选中项、视口状态和所有工作区的 UI 状态。
/// 所有工作区模块通过 `impl NekoSpiceApp` 块扩展此结构体的功能。
/// 编辑操作定义在 [`app_ops`] 子模块。
#[derive(Debug)]
pub struct NekoSpiceApp {
    pub(super) schematic_path: String,
    pub(super) library_table_path: String,
    pub(super) document: Option<NspGuiDocument>,
    pub(super) library: Option<NspGuiLibrary>,
    pub(super) scene: Option<NspCanvasScene>,
    pub(super) selected_hit: Option<NspCanvasHit>,
    /// 当前悬停的画布元素，用于悬停高亮反馈。
    pub(super) hovered_hit: Option<NspCanvasHit>,
    /// 快捷键帮助叠加层是否可见。
    pub(super) show_shortcuts_overlay: bool,
    /// Internal clipboard buffer for cut/copy/paste operations.
    pub(crate) clipboard_buffer: Option<clipboard::ClipboardBuffer>,
    pub(super) selected_symbol_id: Option<String>,
    pub(super) selected_symbol_placement: SymbolPlacementConfig,
    pub(crate) placement: Option<SymbolPlacementState>,
    selection_properties: SelectionPropertyEditorState,
    schematic_inspector: SchematicInspectorPanelState,
    pub(super) schematic_tools: SchematicToolState,
    pub(super) simulation_panel: Box<SimulationPanelState>,
    pub(super) simulation_profile_editor: Box<SimulationProfileEditorState>,
    pub(super) simulation_history: Box<SimulationHistory>,
    pub(super) simulation_measurements: Vec<MeasureEntry>,
    /// Xyce-specific solver options.
    pub(crate) xyce_options: XyceOptions,
    /// Name for saving a new custom preset.
    pub(crate) custom_preset_name: String,
    /// Run comparison state for comparing two historical runs.
    pub(crate) run_compare: RunCompareState,
    optimization_workspace: Box<OptimizationWorkspaceState>,
    review_workspace: Box<ReviewWorkspaceState>,
    reports_workspace: Box<ReportsWorkspaceState>,
    waveform_workspace: Box<WaveformWorkspaceState>,
    active_workspace: StudioWorkspace,
    preferences: StudioPreferences,
    pub(super) symbol_search: String,
    /// 原理图底部停靠面板的活跃标签页。
    pub(super) schematic_bottom_tab: SchematicBottomTab,
    pub(super) load_error: Option<String>,
    pub(super) library_error: Option<String>,
    pub(super) status_message: Option<String>,
    pub(super) viewport: CanvasViewport,
    pub(super) history: history::EditHistory,
    /// 最近一次画布矩形，供上下文菜单缩放操作使用。
    pub(super) last_canvas_rect: Option<eframe::egui::Rect>,
    /// 当前光标世界坐标，用于状态栏显示。
    pub(super) cursor_world: Option<nsp_schema::NspPoint>,
    /// TI/ADI 厂商模型目录
    pub(crate) vendor_catalog: Box<nsp_model::VendorModelCatalog>,
    /// 厂商模型搜索关键词
    pub(crate) vendor_search: String,
    /// 厂商模型目录路径
    pub(crate) vendor_model_path: String,
    /// 是否显示厂商模型面板
    pub(crate) show_vendor_panel: bool,
}

/// 键盘微调方向。对应方向键，每次移动 2.54mm（100mil）。
#[derive(Debug, Clone, Copy)]
pub(super) enum EditNudgeDirection {
    Left,
    Right,
    Up,
    Down,
}

impl EditNudgeDirection {
    /// 返回该方向对应的偏移量（单位 mm）。
    pub(super) fn delta(self) -> nsp_schema::NspPoint {
        match self {
            Self::Left => nsp_schema::NspPoint {
                x: -EDIT_NUDGE_MM,
                y: 0.0,
            },
            Self::Right => nsp_schema::NspPoint {
                x: EDIT_NUDGE_MM,
                y: 0.0,
            },
            Self::Up => nsp_schema::NspPoint {
                x: 0.0,
                y: -EDIT_NUDGE_MM,
            },
            Self::Down => nsp_schema::NspPoint {
                x: 0.0,
                y: EDIT_NUDGE_MM,
            },
        }
    }
}

impl Default for NekoSpiceApp {
    fn default() -> Self {
        let mut app = Self {
            schematic_path: DEFAULT_GUI_SCHEMATIC.to_string(),
            library_table_path: crate::DEFAULT_GUI_LIBRARY_TABLE.to_string(),
            document: None,
            library: None,
            scene: None,
            selected_hit: None,
            hovered_hit: None,
            show_shortcuts_overlay: false,
            clipboard_buffer: None,
            selected_symbol_id: None,
            selected_symbol_placement: SymbolPlacementConfig::default(),
            placement: None,
            selection_properties: SelectionPropertyEditorState::default(),
            schematic_inspector: SchematicInspectorPanelState::default(),
            schematic_tools: SchematicToolState::default(),
            simulation_panel: Box::new(SimulationPanelState::from_disk()),
            simulation_profile_editor: Box::new(SimulationProfileEditorState::from_disk()),
            simulation_history: Box::new(SimulationHistory::default()),
            simulation_measurements: Vec::new(),
            xyce_options: XyceOptions::default(),
            custom_preset_name: String::new(),
            run_compare: RunCompareState::default(),
            optimization_workspace: Box::new(OptimizationWorkspaceState::default()),
            review_workspace: Box::new(ReviewWorkspaceState::default()),
            reports_workspace: Box::new(ReportsWorkspaceState::default()),
            waveform_workspace: Box::new(WaveformWorkspaceState::default()),
            active_workspace: initial_workspace(),
            preferences: StudioPreferences::default(),
            symbol_search: String::new(),
            schematic_bottom_tab: SchematicBottomTab::Waveforms,
            load_error: None,
            library_error: None,
            status_message: None,
            viewport: CanvasViewport::default(),
            history: history::EditHistory::default(),
            last_canvas_rect: None,
            cursor_world: None,
            vendor_catalog: Box::new(nsp_model::VendorModelCatalog::default()),
            vendor_search: String::new(),
            vendor_model_path: String::new(),
            show_vendor_panel: false,
        };
        app.load_initial_resources();
        app
    }
}

/// 根据环境变量选择初始工作区，默认为 Home。
fn initial_workspace() -> StudioWorkspace {
    std::env::var("NEKOSPICE_INITIAL_WORKSPACE")
        .ok()
        .and_then(|value| StudioWorkspace::from_slug(&value))
        .unwrap_or_default()
}
