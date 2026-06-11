use super::{EditNudgeDirection, NekoSpiceApp};
use crate::canvas;
use crate::canvas::colors::SchematicColors;
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
        self.handle_canvas_context_menu(ui, rect);

        let painter = ui.painter_at(rect);
        let schematic_colors = SchematicColors::for_mode(self.theme_mode());
        painter.rect_filled(rect, 0.0, schematic_colors.canvas_bg);
        canvas::draw_grid(&painter, rect, self.viewport, schematic_colors);

        if let Some(scene) = &self.scene {
            let visible_bounds = self.viewport.visible_world_bounds(rect);
            canvas::draw_scene(&painter, rect, scene, self.viewport, visible_bounds, schematic_colors);
            if let Some(hit) = &self.selected_hit {
                canvas::draw_bounds(
                    &painter,
                    rect,
                    self.viewport,
                    hit.bounds,
                    schematic_colors.selection,
                    2.0,
                );
            }

            // Hover highlight: perform a hover hit-test and draw a subtle
            // glow around the hovered item (KiCad-style hover feedback).
            if let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
                && rect.contains(pointer)
            {
                let schematic_point = self.viewport.screen_to_world(rect, pointer);
                self.hovered_hit = scene.hit_test(schematic_point).hits.into_iter().next();
            } else {
                self.hovered_hit = None;
            }
            // Draw hover highlight only if it differs from the current selection.
            if let Some(hovered) = &self.hovered_hit {
                let dominated_by_selection = self
                    .selected_hit
                    .as_ref()
                    .is_some_and(|sel| sel.bounds == hovered.bounds);
                if !dominated_by_selection {
                    canvas::draw_hover_highlight(&painter, rect, self.viewport, hovered.bounds, schematic_colors);
                }
            }
        }

        // Draw keyboard shortcut help overlay if visible
        if self.show_shortcuts_overlay {
            self.draw_shortcuts_overlay(&painter, rect, schematic_colors);
        }

        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            && rect.contains(pointer)
        {
            let schematic_point = self.viewport.screen_to_world(rect, pointer);
            self.draw_symbol_placement_preview(&painter, rect, schematic_point, schematic_colors);
            self.draw_schematic_tool_preview(&painter, rect, schematic_point, schematic_colors);
        }
    }

    fn draw_symbol_placement_preview(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        point: osl_kicad::KicadPoint,
        schematic_colors: SchematicColors,
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
        canvas::draw_scene(painter, rect, &preview.scene, self.viewport, visible_bounds, schematic_colors);
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

    /// Handle keyboard shortcuts for the schematic canvas.
    ///
    /// Tool shortcuts: V=Select, W=Wire, L=Label, B=Bus, S=Sheet,
    /// J=Junction, Q=NoConnect, R=Rotate, F=Fit, Del=Delete, Esc=Cancel.
    fn handle_canvas_shortcuts(&mut self, ui: &egui::Ui) {
        if ui.ctx().text_edit_focused() {
            return;
        }

        // Tool switching shortcuts
        use super::schematic_tools::SchematicTool;
        if ui.input(|input| input.key_pressed(egui::Key::V)) {
            self.activate_schematic_tool_direct(SchematicTool::Select);
        }
        if ui.input(|input| input.key_pressed(egui::Key::W)) {
            self.activate_schematic_tool_direct(SchematicTool::Wire);
        }
        if ui.input(|input| input.key_pressed(egui::Key::L)) {
            self.activate_schematic_tool_direct(SchematicTool::Label);
        }
        if ui.input(|input| input.key_pressed(egui::Key::B)) {
            self.activate_schematic_tool_direct(SchematicTool::Bus);
        }
        if ui.input(|input| input.key_pressed(egui::Key::S)) {
            self.activate_schematic_tool_direct(SchematicTool::Sheet);
        }
        if ui.input(|input| input.key_pressed(egui::Key::J)) {
            self.activate_schematic_tool_direct(SchematicTool::Junction);
        }
        if ui.input(|input| input.key_pressed(egui::Key::Q)) {
            self.activate_schematic_tool_direct(SchematicTool::NoConnect);
        }

        // Action shortcuts
        if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.cancel_symbol_placement();
            self.cancel_schematic_tool_pending();
            self.activate_schematic_tool_direct(SchematicTool::Select);
        }
        if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
            self.delete_selected();
        }
        // ? key toggles keyboard shortcut help overlay
        if ui.input(|input| input.key_pressed(egui::Key::Slash) && input.modifiers.shift) {
            self.show_shortcuts_overlay = !self.show_shortcuts_overlay;
        }
        if ui.input(|input| input.key_pressed(egui::Key::R)) {
            self.rotate_selected();
        }
        if ui.input(|input| input.key_pressed(egui::Key::F)) {
            self.viewport
                .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
        }


        // Arrow key nudging
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

        // Undo / Redo
        if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Z)) {
            if ui.input(|input| input.modifiers.shift) {
                self.redo();
            } else {
                self.undo();
            }
        }
        // Ctrl+Y as alternative redo shortcut
        if ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Y)) {
            self.redo();
        }
    }

    /// Handle right-click context menu on the canvas.
    fn handle_canvas_context_menu(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        let response = ui.interact(rect, egui::Id::new("canvas_context"), egui::Sense::click());

        if response.secondary_clicked() {
            let action = self.draw_canvas_context_menu(ui);
            match action {
                super::ContextMenuAction::DeleteSelected => self.delete_selected(),
                super::ContextMenuAction::RotateSelected => {
                    self.rotate_selected();
                }
                super::ContextMenuAction::CutSelected => {
                    self.status_message = Some("Cut to clipboard".to_string());
                }
                super::ContextMenuAction::CopySelected => {
                    self.status_message = Some("Copied to clipboard".to_string());
                }
                super::ContextMenuAction::PasteAtCursor => {
                    self.status_message = Some("Paste from clipboard".to_string());
                }
                super::ContextMenuAction::None => {}
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
