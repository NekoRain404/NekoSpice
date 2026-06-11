use super::library_data::spice_preview_lines;
use super::library_widgets::code_line;
use crate::canvas::colors::SchematicColors;
use eframe::egui::{self, Color32, Vec2};
use osl_kicad::{KicadCanvasScene, KicadIndexedSymbol};

const SPICE_PREVIEW_LINES: usize = 16;

pub(super) fn draw_symbol_preview(ui: &mut egui::Ui, scene: &KicadCanvasScene, fill: Color32, mode: crate::app::theme::StudioThemeMode) {
    let available_width = ui.available_width().clamp(220.0, 520.0);
    let desired_size = Vec2::new(available_width, 260.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, fill);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, Color32::from_rgb(210, 216, 222)),
        egui::StrokeKind::Inside,
    );

    let viewport = crate::viewport::CanvasViewport::for_rect(rect, scene.bounds);
    let visible_bounds = viewport.visible_world_bounds(rect);
    let colors = SchematicColors::for_mode(mode);
    crate::canvas::draw_scene(&painter, rect, scene, viewport, visible_bounds, colors);
    if let Some(bounds) = scene.bounds {
        crate::canvas::draw_bounds(
            &painter,
            rect,
            viewport,
            bounds,
            Color32::from_rgb(130, 150, 170),
            1.0,
        );
    }
}

pub(super) fn draw_spice_preview(ui: &mut egui::Ui, symbol: &KicadIndexedSymbol) {
    egui::ScrollArea::both()
        .id_salt("library_workspace_spice_preview")
        .max_height(320.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (index, line) in spice_preview_lines(symbol)
                .into_iter()
                .take(SPICE_PREVIEW_LINES)
                .enumerate()
            {
                code_line(ui, index + 1, &line);
            }
        });
}
