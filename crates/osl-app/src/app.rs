use crate::canvas;
use crate::document::KicadGuiDocument;
use crate::library::KicadGuiLibrary;
use crate::viewport::CanvasViewport;
use crate::{DEFAULT_SCHEMATIC, DEFAULT_SYMBOL_LIBRARY_TABLE};
use eframe::egui::{self, Color32, Sense, Vec2};
use osl_kicad::{
    KicadCanvasHit, KicadCanvasScene, KicadPoint, read_kicad_schematic_with_libraries,
};
use std::path::{Path, PathBuf};

mod panels;

const EDIT_NUDGE_MM: f64 = 2.54;

pub fn run_native() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NekoSpice")
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([960.0, 640.0])
            .with_app_id("nekospice"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "NekoSpice",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            let mut style = (*cc.egui_ctx.global_style()).clone();
            style.spacing.item_spacing = Vec2::new(8.0, 6.0);
            style.spacing.button_padding = Vec2::new(10.0, 4.0);
            cc.egui_ctx.set_global_style(style);
            Ok(Box::new(NekoSpiceApp::default()))
        }),
    )
}

#[derive(Debug)]
pub struct NekoSpiceApp {
    pub(super) schematic_path: String,
    pub(super) library_table_path: String,
    pub(super) document: Option<KicadGuiDocument>,
    pub(super) library: Option<KicadGuiLibrary>,
    pub(super) scene: Option<KicadCanvasScene>,
    pub(super) selected_hit: Option<KicadCanvasHit>,
    pub(super) selected_symbol_id: Option<String>,
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
                self.library_error = None;
                self.status_message = Some("Loaded symbol library".to_string());
            }
            Err(error) => {
                self.library = None;
                self.selected_symbol_id = None;
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

    fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size_before_wrap();
        let desired_size = Vec2::new(available.x.max(240.0), available.y.max(240.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if response.dragged_by(egui::PointerButton::Middle) {
            self.viewport.pan += response.drag_delta();
        }

        let pointer_over_canvas = ui
            .input(|input| input.pointer.hover_pos())
            .is_some_and(|position| rect.contains(position));
        if pointer_over_canvas {
            let zoom_delta = ui.input(|input| input.zoom_delta());
            if (zoom_delta - 1.0).abs() > f32::EPSILON
                && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            {
                self.viewport.zoom_around(rect, pointer, zoom_delta);
            }

            let scroll = ui.input(|input| input.smooth_scroll_delta);
            if scroll != Vec2::ZERO {
                self.viewport.pan += scroll;
            }
        }

        if response.clicked()
            && let (Some(scene), Some(pointer)) = (&self.scene, response.interact_pointer_pos())
        {
            let schematic_point = self.viewport.screen_to_world(rect, pointer);
            self.selected_hit = scene.hit_test(schematic_point).hits.into_iter().next();
        }

        if !ui.ctx().text_edit_focused() {
            if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
                self.delete_selected();
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowLeft)) {
                self.nudge_selected(EditNudgeDirection::Left);
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowRight)) {
                self.nudge_selected(EditNudgeDirection::Right);
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                self.nudge_selected(EditNudgeDirection::Up);
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                self.nudge_selected(EditNudgeDirection::Down);
            }
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(248, 249, 250));
        canvas::draw_grid(&painter, rect, self.viewport);

        if let Some(scene) = &self.scene {
            let visible_bounds = self.viewport.visible_world_bounds(rect);
            canvas::draw_scene(&painter, rect, scene, self.viewport, visible_bounds);
            if let Some(hit) = &self.selected_hit {
                canvas::draw_bounds(
                    &painter,
                    rect,
                    self.viewport,
                    hit.bounds,
                    Color32::from_rgb(20, 120, 220),
                    2.0,
                );
            }
        }
    }
}

pub fn load_canvas_scene(path: &Path) -> Result<KicadCanvasScene, String> {
    read_kicad_schematic_with_libraries(path)
        .map(|schematic| schematic.canvas_scene())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_canvas_scene_for_gui() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let scene = load_canvas_scene(&workspace_root.join(DEFAULT_SCHEMATIC)).unwrap();
        assert!(!scene.symbols.is_empty());
        assert!(scene.bounds.is_some());
    }
}
