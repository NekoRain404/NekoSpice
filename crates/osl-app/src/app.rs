use crate::document::KicadGuiDocument;
use crate::library::KicadGuiLibrary;
use crate::placement_config::SymbolPlacementConfig;
use crate::viewport::CanvasViewport;
use crate::{DEFAULT_SCHEMATIC, DEFAULT_SYMBOL_LIBRARY_TABLE};
use osl_kicad::{KicadCanvasHit, KicadCanvasScene, KicadPoint};
use std::path::PathBuf;

mod canvas_panel;
mod center_workspace;
mod diagnostics_panel;
mod home_dashboard;
mod home_insights_panel;
mod home_project_context;
mod home_sections;
mod home_widgets;
mod library_data;
mod library_inspector;
mod library_preview;
mod library_sections;
mod library_widgets;
mod library_workspace;
mod localization;
mod navigation;
mod navigation_panel;
mod panels;
mod placement;
mod preferences;
mod project_panel;
mod reports_workspace;
mod reports_workspace_sections;
mod reports_workspace_widgets;
mod runtime;
mod schematic_inspector_panel;
mod schematic_inspector_sections;
mod schematic_inspector_simulator;
mod schematic_inspector_widgets;
mod schematic_tools;
mod schematic_workspace;
mod schematic_workspace_widgets;
mod selection_properties;
mod simulation_artifacts_panel;
mod simulation_panel;
mod simulation_report_panel;
mod simulation_waveform_panel;
mod simulation_workspace;
mod simulation_workspace_sections;
mod simulation_workspace_widgets;
mod status_strip;
mod studio_toolbar;
mod symbol_placement_controls;
mod theme;
mod waveform_preview;
mod waveform_preview_primitives;
mod waveform_workspace;
mod waveform_workspace_sections;
mod waveform_workspace_widgets;
mod widgets;
mod workspace_panel;

pub use canvas_panel::load_canvas_scene;
use navigation::StudioWorkspace;
use placement::SymbolPlacementState;
use preferences::StudioPreferences;
pub use runtime::run_native;
use schematic_inspector_panel::SchematicInspectorPanelState;
use schematic_tools::SchematicToolState;
use selection_properties::SelectionPropertyEditorState;
use simulation_panel::SimulationPanelState;
use waveform_workspace::WaveformWorkspaceState;

const EDIT_NUDGE_MM: f64 = 2.54;

#[derive(Debug)]
pub struct NekoSpiceApp {
    pub(super) schematic_path: String,
    pub(super) library_table_path: String,
    pub(super) document: Option<KicadGuiDocument>,
    pub(super) library: Option<KicadGuiLibrary>,
    pub(super) scene: Option<KicadCanvasScene>,
    pub(super) selected_hit: Option<KicadCanvasHit>,
    pub(super) selected_symbol_id: Option<String>,
    pub(super) selected_symbol_placement: SymbolPlacementConfig,
    pub(crate) placement: Option<SymbolPlacementState>,
    selection_properties: SelectionPropertyEditorState,
    schematic_inspector: SchematicInspectorPanelState,
    pub(super) schematic_tools: SchematicToolState,
    pub(super) simulation_panel: SimulationPanelState,
    waveform_workspace: WaveformWorkspaceState,
    active_workspace: StudioWorkspace,
    preferences: StudioPreferences,
    pub(super) symbol_search: String,
    pub(super) load_error: Option<String>,
    pub(super) library_error: Option<String>,
    pub(super) status_message: Option<String>,
    pub(super) viewport: CanvasViewport,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum EditNudgeDirection {
    Left,
    Right,
    Up,
    Down,
}

impl EditNudgeDirection {
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
            schematic_path: DEFAULT_SCHEMATIC.to_string(),
            library_table_path: DEFAULT_SYMBOL_LIBRARY_TABLE.to_string(),
            document: None,
            library: None,
            scene: None,
            selected_hit: None,
            selected_symbol_id: None,
            selected_symbol_placement: SymbolPlacementConfig::default(),
            placement: None,
            selection_properties: SelectionPropertyEditorState::default(),
            schematic_inspector: SchematicInspectorPanelState::default(),
            schematic_tools: SchematicToolState::default(),
            simulation_panel: SimulationPanelState::default(),
            waveform_workspace: WaveformWorkspaceState::default(),
            active_workspace: initial_workspace(),
            preferences: StudioPreferences::default(),
            symbol_search: String::new(),
            load_error: None,
            library_error: None,
            status_message: None,
            viewport: CanvasViewport::default(),
        };
        app.load_schematic(PathBuf::from(DEFAULT_SCHEMATIC));
        app.load_symbol_library(PathBuf::from(DEFAULT_SYMBOL_LIBRARY_TABLE));
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
                self.status_message = Some("Loaded schematic".to_string());
            }
            Err(error) => {
                self.load_error = Some(error.to_string());
                self.status_message = None;
            }
        }
    }

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

    pub(super) fn delete_selected(&mut self) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

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

        match document.move_item(&uuid, delta) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit = scene.item_hit_by_uuid(&uuid);
                self.scene = Some(scene);
                self.sync_property_editor_from_selection();
                self.load_error = None;
                self.status_message =
                    Some(format!("Moved {} {}", summary.operation, summary.target));
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    pub(super) fn nudge_selected(&mut self, direction: EditNudgeDirection) {
        self.move_selected(direction.delta());
    }

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
