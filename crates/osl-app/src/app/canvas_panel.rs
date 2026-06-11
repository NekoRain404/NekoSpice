use super::{EditNudgeDirection, NekoSpiceApp};
use crate::canvas;
use crate::canvas::colors;
use eframe::egui::{self, Color32, Sense, Vec2};
use osl_kicad::{KicadAt, KicadCanvasScene, read_kicad_schematic_with_libraries};
use std::path::Path;

impl NekoSpiceApp {
    pub(super) fn draw_canvas(&mut self, ui: &mut egui::Ui) {
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
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let schematic_point = self.viewport.screen_to_world(rect, pointer);
            if self.placement.is_some() {
                self.place_selected_symbol_at_point(schematic_point);
            } else if self.handle_schematic_tool_click(schematic_point) {
            } else if let Some(scene) = &self.scene {
                self.selected_hit = scene.hit_test(schematic_point).hits.into_iter().next();
                self.sync_property_editor_from_selection();
            }
        }

        self.handle_canvas_shortcuts(ui);

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, self.theme_palette().canvas);
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
                    colors::SELECTION,
                    2.0,
                );
            }
        }

        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            && rect.contains(pointer)
        {
            let schematic_point = self.viewport.screen_to_world(rect, pointer);
            self.draw_symbol_placement_preview(&painter, rect, schematic_point);
            self.draw_schematic_tool_preview(&painter, rect, schematic_point);
        }
    }

    fn draw_symbol_placement_preview(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        point: osl_kicad::KicadPoint,
    ) {
        let Some(placement) = &self.placement else {
            return;
        };
        let Some(library) = &self.library else {
            return;
        };
        let Ok(preview) = library.symbol_placement_preview(
            &placement.symbol_id,
            KicadAt {
                x: point.x,
                y: point.y,
                rotation: 0.0,
            },
            placement.config.clone(),
        ) else {
            return;
        };

        let visible_bounds = self.viewport.visible_world_bounds(rect);
        canvas::draw_scene(painter, rect, &preview.scene, self.viewport, visible_bounds);
        if let Some(bounds) = preview.scene.bounds {
            canvas::draw_bounds(
                painter,
                rect,
                self.viewport,
                bounds,
                Color32::from_rgb(80, 120, 190),
                1.5,
            );
        }
    }

    fn handle_canvas_shortcuts(&mut self, ui: &egui::Ui) {
        if ui.ctx().text_edit_focused() {
            return;
        }

        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.cancel_symbol_placement();
            self.cancel_schematic_tool_pending();
        }
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
}

pub fn load_canvas_scene(path: &Path) -> Result<KicadCanvasScene, String> {
    read_kicad_schematic_with_libraries(path)
        .map(|schematic| schematic.canvas_scene())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DEFAULT_SCHEMATIC;

    #[test]
    fn loads_default_canvas_scene_for_gui() {
        let scene =
            load_canvas_scene(&crate::test_support::workspace_root().join(DEFAULT_SCHEMATIC))
                .unwrap();
        assert!(!scene.symbols.is_empty());
        assert!(scene.bounds.is_some());
    }
}
