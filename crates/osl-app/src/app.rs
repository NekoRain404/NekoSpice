use crate::document::KicadGuiDocument;
use crate::library::KicadGuiLibrary;
use crate::viewport::CanvasViewport;
use crate::{DEFAULT_SCHEMATIC, DEFAULT_SYMBOL_LIBRARY_TABLE};
use osl_kicad::{KicadCanvasHit, KicadCanvasScene, KicadPoint};
use std::path::PathBuf;

mod canvas_panel;
mod panels;
mod placement;
mod runtime;
mod symbol_browser;

pub use canvas_panel::load_canvas_scene;
use placement::SymbolPlacementState;
pub use runtime::run_native;

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
    pub(crate) placement: Option<SymbolPlacementState>,
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
            placement: None,
            symbol_search: String::new(),
            load_error: None,
            library_error: None,
            status_message: None,
            viewport: CanvasViewport::default(),
        };
        app.load_schematic(PathBuf::from(DEFAULT_SCHEMATIC));
        app.load_symbol_library(PathBuf::from(DEFAULT_SYMBOL_LIBRARY_TABLE));
        app
    }
}

impl NekoSpiceApp {
    pub(super) fn load_schematic(&mut self, path: PathBuf) {
        match KicadGuiDocument::load(path.clone()) {
            Ok(document) => {
                let scene = document.scene();
                self.schematic_path = path.display().to_string();
                self.viewport.fit_scene(scene.bounds);
                self.document = Some(document);
                self.scene = Some(scene);
                self.selected_hit = None;
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
                self.selected_symbol_id = None;
                self.placement = None;
                self.library_error = None;
                self.status_message = Some("Loaded symbol library".to_string());
            }
            Err(error) => {
                self.library = None;
                self.selected_symbol_id = None;
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
