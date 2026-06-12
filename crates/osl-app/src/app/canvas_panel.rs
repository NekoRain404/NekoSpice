//! 主画布面板。实现原理图画布的渲染、鼠标交互和坐标变换。
//!
use super::NekoSpiceApp;
use crate::canvas;
use crate::canvas::colors::SchematicColors;
use eframe::egui::{self, Sense, Vec2};
use osl_kicad::{KicadCanvasScene, read_kicad_schematic_with_libraries};
use std::path::Path;

impl NekoSpiceApp {
    /// Draw the main schematic canvas with grid, scene, and interaction.
    ///
    /// Mouse controls:
    /// - Right-click drag = pan
    /// - Scroll wheel = zoom (around cursor)
    /// - Left-click = select / place / tool action
    /// - Right-click (no drag) = context menu
    pub(super) fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size_before_wrap();
        let desired_size = Vec2::new(available.x.max(240.0), available.y.max(240.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
        self.last_canvas_rect = Some(rect);

        // --- Right-click drag panning ---
        let is_right_dragging = response.dragged_by(egui::PointerButton::Secondary);
        if is_right_dragging {
            self.viewport.pan += response.drag_delta();
        }
        // --- Middle-click drag panning (common CAD convention) ---
        let is_middle_dragging = response.dragged_by(egui::PointerButton::Middle);
        if is_middle_dragging {
            self.viewport.pan += response.drag_delta();
        }

        // --- Scroll wheel zoom ---
        let pointer_over_canvas = ui
            .input(|input| input.pointer.hover_pos())
            .is_some_and(|position| rect.contains(position));
        if pointer_over_canvas {
            let scroll = ui.input(|input| input.smooth_scroll_delta);
            if scroll.y.abs() > 0.5 {
                let zoom_factor = (scroll.y * 0.005).exp();
                if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                    self.viewport.zoom_around(rect, pointer, zoom_factor);
                }
            }
        }

        // --- Left-click for selection / placement / tool actions ---
        if response.clicked() && !is_right_dragging {
            if let Some(pointer) = response.interact_pointer_pos() {
                let is_left_click = ui.input(|input| input.pointer.primary_released());
                if is_left_click {
                    let schematic_point = self.viewport.screen_to_world(rect, pointer);
                    if self.placement.is_some() {
                        self.place_selected_symbol_at_point(schematic_point);
                    } else if self.handle_schematic_tool_click(schematic_point) {
                    } else if let Some(scene) = &self.scene {
                        self.selected_hit =
                            scene.hit_test(schematic_point).hits.into_iter().next();
                        self.sync_property_editor_from_selection();
                    }
                }
            }
        }

        // Delegates to canvas_shortcuts.rs and canvas_context_menu.rs
        self.handle_canvas_shortcuts(ui);
        self.handle_canvas_context_menu_with_pan(ui, rect, is_right_dragging);

        // --- Rendering ---
        let painter = ui.painter_at(rect);
        let schematic_colors = SchematicColors::for_mode(self.theme_mode());
        painter.rect_filled(rect, 0.0, schematic_colors.canvas_bg);
        canvas::draw_grid(&painter, rect, self.viewport, schematic_colors);

        if let Some(scene) = &self.scene {
            let visible_bounds = self.viewport.visible_world_bounds(rect);
            canvas::draw_scene(
                &painter,
                rect,
                scene,
                self.viewport,
                visible_bounds,
                schematic_colors,
            );
            // Selection highlight
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

            // Hover highlight
            if let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
                && rect.contains(pointer)
            {
                let schematic_point = self.viewport.screen_to_world(rect, pointer);
                self.hovered_hit = scene.hit_test(schematic_point).hits.into_iter().next();
            } else {
                self.hovered_hit = None;
            }
            if let Some(hovered) = &self.hovered_hit {
                let dominated_by_selection = self
                    .selected_hit
                    .as_ref()
                    .is_some_and(|sel| sel.bounds == hovered.bounds);
                if !dominated_by_selection {
                    canvas::draw_hover_highlight(
                        &painter,
                        rect,
                        self.viewport,
                        hovered.bounds,
                        schematic_colors,
                    );
                }
            }
        }

        // Keyboard shortcut help overlay
        if self.show_shortcuts_overlay {
            self.draw_shortcuts_overlay(&painter, rect, schematic_colors);
        }

        // Placement and tool previews
        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            && rect.contains(pointer)
        {
            let schematic_point = self.viewport.screen_to_world(rect, pointer);
            self.draw_symbol_placement_preview(&painter, rect, schematic_point, schematic_colors);
            self.draw_schematic_tool_preview(&painter, rect, schematic_point, schematic_colors);
        }
    }

    /// Draw a ghost preview of the symbol being placed.
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
        let at = osl_kicad::KicadAt { x: point.x, y: point.y, rotation: 0.0 };
        let Ok(preview) = library.symbol_placement_preview(
            &placement.symbol_id,
            at,
            self.selected_symbol_placement.clone(),
        ) else {
            return;
        };
        canvas::draw_scene(
            painter,
            rect,
            &preview.scene,
            self.viewport,
            preview.scene.bounds.unwrap_or(osl_kicad::KicadBoundingBox {
                min: point,
                max: point,
            }),
            schematic_colors,
        );
    }
}

/// Load a KiCad schematic file and build the canvas scene for rendering.
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
