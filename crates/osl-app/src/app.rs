//! NekoSpice 应用程序主模块。定义 [`NekoSpiceApp`] 核心结构体及其编辑操作（移动、旋转、删除、撤销/重做）。所有工作区子模块通过此处声明的模块层次访问共享的应用状态。
//!
use crate::document::KicadGuiDocument;
use crate::library::KicadGuiLibrary;
use crate::placement_config::SymbolPlacementConfig;
use crate::viewport::CanvasViewport;
use crate::{DEFAULT_GUI_SCHEMATIC, DEFAULT_GUI_LIBRARY_TABLE};
use osl_kicad::{KicadCanvasHit, KicadCanvasScene, KicadPoint};
use std::path::PathBuf;

// ── Cross-cutting app modules (stay at app level) ──────────────────────
mod canvas_context_menu;
mod canvas_panel;
mod canvas_shortcuts;
mod center_workspace;
mod diagnostics_panel;
mod file_dialog;
mod history;
mod localization;
mod navigation;
mod navigation_panel;
mod panels;
mod placement;
mod preferences;
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
use waveform::WaveformWorkspaceState;
use review::ReviewWorkspaceState;
pub use runtime::run_native;
use simulation::SimulationProfileEditorState;
use schematic::inspector::SchematicInspectorPanelState;
use schematic::tools::SchematicToolState;
use schematic::SelectionPropertyEditorState;
use simulation::SimulationPanelState;

const EDIT_NUDGE_MM: f64 = 2.54;

/// Active tab in the schematic workspace bottom dock.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// 原理图工作区底部停靠面板的活跃标签页。
///
/// 切换波形预览、FFT 分析、波特图、控制台输出、网表、ERC 检查和属性检查器。
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

#[derive(Debug)]
/// NekoSpice 应用程序核心状态。
///
/// 持有当前文档、库、画布场景、选中项、视口状态和所有工作区的 UI 状态。
/// 所有工作区模块通过 `impl NekoSpiceApp` 块扩展此结构体的功能。
pub struct NekoSpiceApp {
    pub(super) schematic_path: String,
    pub(super) library_table_path: String,
    pub(super) document: Option<KicadGuiDocument>,
    pub(super) library: Option<KicadGuiLibrary>,
    pub(super) scene: Option<KicadCanvasScene>,
    pub(super) selected_hit: Option<KicadCanvasHit>,
    /// Currently hovered canvas item for hover-highlight feedback.
    pub(super) hovered_hit: Option<KicadCanvasHit>,
    /// Whether the keyboard shortcut help overlay is visible.
    pub(super) show_shortcuts_overlay: bool,
    pub(super) selected_symbol_id: Option<String>,
    pub(super) selected_symbol_placement: SymbolPlacementConfig,
    pub(crate) placement: Option<SymbolPlacementState>,
    selection_properties: SelectionPropertyEditorState,
    schematic_inspector: SchematicInspectorPanelState,
    pub(super) schematic_tools: SchematicToolState,
    pub(super) simulation_panel: SimulationPanelState,
    pub(super) simulation_profile_editor: SimulationProfileEditorState,
    optimization_workspace: OptimizationWorkspaceState,
    review_workspace: ReviewWorkspaceState,
    reports_workspace: ReportsWorkspaceState,
    waveform_workspace: WaveformWorkspaceState,
    active_workspace: StudioWorkspace,
    preferences: StudioPreferences,
    pub(super) symbol_search: String,
    /// Active tab in the schematic bottom dock panel.
    pub(super) schematic_bottom_tab: SchematicBottomTab,
    pub(super) load_error: Option<String>,
    pub(super) library_error: Option<String>,
    pub(super) status_message: Option<String>,
    pub(super) viewport: CanvasViewport,
    pub(super) history: history::EditHistory,
    /// Last known canvas rect, used by context menu zoom operations.
    pub(super) last_canvas_rect: Option<eframe::egui::Rect>,
    /// Current cursor world coordinates for status bar display.
    pub(super) cursor_world: Option<osl_kicad::KicadPoint>,
    /// TI/ADI 厂商模型目录
    pub(crate) vendor_catalog: osl_model::VendorModelCatalog,
    /// 厂商模型搜索关键词
    pub(crate) vendor_search: String,
    /// 厂商模型目录路径
    pub(crate) vendor_model_path: String,
    /// 是否显示厂商模型面板
    pub(crate) show_vendor_panel: bool,
}

#[derive(Debug, Clone, Copy)]
/// 键盘微调方向。对应方向键，每次移动 2.54mm（100mil）。
pub(super) enum EditNudgeDirection {
    Left,
    Right,
    Up,
    Down,
}

impl EditNudgeDirection {
    /// delta。
    pub(super) fn delta(self) -> KicadPoint {
        match self {
            Self::Left => KicadPoint {
                x: -EDIT_NUDGE_MM,
                y: 0.0,
            },
            Self::Right => KicadPoint {
                x: EDIT_NUDGE_MM,
                y: 0.0,
            },
            Self::Up => KicadPoint {
                x: 0.0,
                y: -EDIT_NUDGE_MM,
            },
            Self::Down => KicadPoint {
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
            library_table_path: DEFAULT_GUI_LIBRARY_TABLE.to_string(),
            document: None,
            library: None,
            scene: None,
            selected_hit: None,
            hovered_hit: None,
            show_shortcuts_overlay: false,
            selected_symbol_id: None,
            selected_symbol_placement: SymbolPlacementConfig::default(),
            placement: None,
            selection_properties: SelectionPropertyEditorState::default(),
            schematic_inspector: SchematicInspectorPanelState::default(),
            schematic_tools: SchematicToolState::default(),
            simulation_panel: SimulationPanelState::default(),
            simulation_profile_editor: SimulationProfileEditorState::default(),
            optimization_workspace: OptimizationWorkspaceState::default(),
            review_workspace: ReviewWorkspaceState::default(),
            reports_workspace: ReportsWorkspaceState::default(),
            waveform_workspace: WaveformWorkspaceState::default(),
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
            vendor_catalog: osl_model::VendorModelCatalog::default(),
            vendor_search: String::new(),
            vendor_model_path: String::new(),
            show_vendor_panel: false,
        };
        app.load_schematic(PathBuf::from(DEFAULT_GUI_SCHEMATIC));
        app.load_symbol_library(PathBuf::from(DEFAULT_GUI_LIBRARY_TABLE));
        app.load_initial_simulation_run();
        app
    }
}

fn initial_workspace() -> StudioWorkspace {
    std::env::var("NEKOSPICE_INITIAL_WORKSPACE")
        .ok()
        .and_then(|value| StudioWorkspace::from_slug(&value))
        .unwrap_or_default()
}

impl NekoSpiceApp {
    fn load_initial_simulation_run(&mut self) {
        let Some(path) = std::env::var_os("NEKOSPICE_INITIAL_RUN_DIR") else {
            return;
        };
        match crate::simulation::GuiSimulationRun::from_output_dir(PathBuf::from(path)) {
            Ok(run) => {
                self.sync_selected_waveform_signal(&run.waveform);
                self.simulation_panel.last_run = Some(run);
            }
            Err(error) => {
                self.simulation_panel.last_error = Some(error.clone());
                self.status_message = Some(error);
            }
        }
    }

    /// load schematic。
    pub(super) fn load_schematic(&mut self, path: PathBuf) {
        match KicadGuiDocument::load(path.clone()) {
            Ok(document) => {
                let scene = document.scene();
                self.schematic_path = path.display().to_string();
                self.viewport.fit_scene(scene.bounds);
                self.document = Some(document);
                self.scene = Some(scene);
                self.selected_hit = None;
                self.clear_property_editor();
                self.schematic_tools.clear_pending();
                self.load_error = None;
                self.history.clear();
                self.status_message = Some("Loaded schematic".to_string());
            }
            Err(error) => {
                self.load_error = Some(error.to_string());
                self.status_message = None;
            }
        }
    }

    /// load symbol library。
    pub(super) fn load_symbol_library(&mut self, path: PathBuf) {
        match KicadGuiLibrary::load(path.clone()) {
            Ok(library) => {
                self.library_table_path = path.display().to_string();
                self.library = Some(library);
                self.selected_symbol_id = self
                    .library
                    .as_ref()
                    .and_then(|library| library.index().symbols.first())
                    .map(|symbol| symbol.id.clone());
                self.selected_symbol_placement = SymbolPlacementConfig::default();
                self.placement = None;
                self.library_error = None;
                self.status_message = Some("Loaded symbol library".to_string());
            }
            Err(error) => {
                self.library = None;
                self.selected_symbol_id = None;
                self.selected_symbol_placement = SymbolPlacementConfig::default();
                self.placement = None;
                self.library_error = Some(error.to_string());
            }
        }
    }

    /// delete selected。
    pub(super) fn delete_selected(&mut self) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        // Snapshot before edit for undo support
        self.history.push(document.snapshot());

        match document.delete_item(&uuid) {
            Ok(summary) => {
                let scene = document.scene();
                self.viewport.fit_scene(scene.bounds);
                self.scene = Some(scene);
                self.selected_hit = None;
                self.clear_property_editor();
                self.load_error = None;
                self.status_message =
                    Some(format!("Deleted {} {}", summary.operation, summary.target));
                self.history.clear_redo();
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    fn move_selected(&mut self, delta: KicadPoint) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        // Snapshot before edit for undo support
        self.history.push(document.snapshot());

        match document.move_item(&uuid, delta) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit = scene.item_hit_by_uuid(&uuid);
                self.scene = Some(scene);
                self.sync_property_editor_from_selection();
                self.load_error = None;
                self.status_message =
                    Some(format!("Moved {} {}", summary.operation, summary.target));
                self.history.clear_redo();
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    /// nudge selected。
    pub(super) fn nudge_selected(&mut self, direction: EditNudgeDirection) {
        self.move_selected(direction.delta());
    }

    /// rotate selected。
    pub(super) fn rotate_selected(&mut self) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        // Snapshot before edit for undo support
        self.history.push(document.snapshot());

        match document.rotate_item(&uuid, 90.0) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit = scene.item_hit_by_uuid(&uuid);
                self.scene = Some(scene);
                self.sync_property_editor_from_selection();
                self.load_error = None;
                self.status_message =
                    Some(format!("Rotated {} {}", summary.operation, summary.target));
                self.history.clear_redo();
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    /// Restore the schematic to the previous undo state.
    pub(super) fn undo(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let current = document.snapshot();
        let Some(previous) = self.history.undo(current) else {
            self.status_message = Some("Nothing to undo".to_string());
            return;
        };
        document.restore_snapshot(previous);
        let scene = document.scene();
        self.viewport.fit_scene(scene.bounds);
        self.scene = Some(scene);
        self.selected_hit = None;
        self.clear_property_editor();
        self.status_message = Some("Undo".to_string());
    }

    /// Re-apply the most recently undone edit.
    pub(super) fn redo(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let current = document.snapshot();
        let Some(next) = self.history.redo(current) else {
            self.status_message = Some("Nothing to redo".to_string());
            return;
        };
        document.restore_snapshot(next);
        let scene = document.scene();
        self.viewport.fit_scene(scene.bounds);
        self.scene = Some(scene);
        self.selected_hit = None;
        self.clear_property_editor();
        self.status_message = Some("Redo".to_string());
    }

    /// save document。
    pub(super) fn save_document(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        match document.save() {
            Ok(()) => {
                self.status_message = Some(format!("Saved {}", document.path().display()));
                self.load_error = None;
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }
}
