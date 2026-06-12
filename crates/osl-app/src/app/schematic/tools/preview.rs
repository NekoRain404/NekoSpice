use crate::canvas;
use crate::canvas::colors::SchematicColors;
use crate::viewport::CanvasViewport;
use eframe::egui::{self, Align2, FontId, Pos2, Rect, Stroke, StrokeKind};
use osl_kicad::KicadPoint;

use super::state::{SchematicTool, SchematicToolState};

/// draw schematic tool preview。
pub(crate) fn draw_schematic_tool_preview(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    tools: &SchematicToolState,
    point: KicadPoint,
    colors: SchematicColors,
) {
    match tools.active {
        SchematicTool::Wire => {
            if let Some(start) = tools.pending_wire_start {
                canvas::draw_line(
                    painter,
                    rect,
                    viewport,
                    start,
                    point,
                    colors.wire,
                    1.5,
                );
            }
        }
        SchematicTool::Bus => {
            if let Some(start) = tools.pending_bus_start {
                canvas::draw_line(
                    painter,
                    rect,
                    viewport,
                    start,
                    point,
                    colors.bus,
                    2.5,
                );
            }
        }
        SchematicTool::BusEntry => {
            let end = KicadPoint {
                x: point.x + tools.bus_entry_size.width,
                y: point.y + tools.bus_entry_size.height,
            };
            canvas::draw_line(
                painter,
                rect,
                viewport,
                point,
                end,
                colors.bus,
                2.0,
            );
        }
        SchematicTool::Label | SchematicTool::GlobalLabel | SchematicTool::HierarchicalLabel => {
            painter.text(
                viewport.world_to_screen(rect, point),
                Align2::LEFT_TOP,
                &tools.label_text,
                FontId::monospace(12.0),
                colors.label_local,
            );
        }
        SchematicTool::Text => {
            painter.text(
                viewport.world_to_screen(rect, point),
                Align2::LEFT_TOP,
                &tools.text_item,
                FontId::monospace(12.0),
                colors.text_spice_directive,
            );
        }
        SchematicTool::Sheet => {
            let start = viewport.world_to_screen(rect, point);
            let end = viewport.world_to_screen(
                rect,
                KicadPoint {
                    x: point.x + tools.sheet_size.width,
                    y: point.y + tools.sheet_size.height,
                },
            );
            let sheet_rect = Rect::from_two_pos(start, end);
            painter.rect_stroke(
                sheet_rect,
                0.0,
                Stroke::new(1.5, colors.sheet_border),
                StrokeKind::Inside,
            );
            painter.text(
                sheet_rect.left_top() + egui::vec2(4.0, 4.0),
                Align2::LEFT_TOP,
                &tools.sheet_name,
                FontId::monospace(12.0),
                colors.sheet_name,
            );
        }
        SchematicTool::Junction => {
            painter.circle_filled(
                viewport.world_to_screen(rect, point),
                3.0,
                colors.junction,
            );
        }
        SchematicTool::NoConnect => {
            draw_no_connect_preview(painter, rect, viewport.world_to_screen(rect, point), colors);
        }
        SchematicTool::Select => {}
    }
}

fn draw_no_connect_preview(painter: &egui::Painter, rect: Rect, center: Pos2, colors: SchematicColors) {
    if !rect.contains(center) {
        return;
    }
    let size = 5.0;
    painter.line_segment(
        [
            Pos2::new(center.x - size, center.y - size),
            Pos2::new(center.x + size, center.y + size),
        ],
        Stroke::new(1.5, colors.no_connect),
    );
    painter.line_segment(
        [
            Pos2::new(center.x - size, center.y + size),
            Pos2::new(center.x + size, center.y - size),
        ],
        Stroke::new(1.5, colors.no_connect),
    );
}
