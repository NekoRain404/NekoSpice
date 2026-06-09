use crate::canvas;
use crate::viewport::CanvasViewport;
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke};
use osl_kicad::KicadPoint;

use super::state::{SchematicTool, SchematicToolState};

pub(super) fn draw_schematic_tool_preview(
    painter: &egui::Painter,
    rect: Rect,
    viewport: CanvasViewport,
    tools: &SchematicToolState,
    point: KicadPoint,
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
                    Color32::from_rgb(0, 130, 85),
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
                    Color32::from_rgb(70, 95, 220),
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
                Color32::from_rgb(70, 95, 220),
                2.0,
            );
        }
        SchematicTool::Label | SchematicTool::GlobalLabel | SchematicTool::HierarchicalLabel => {
            painter.text(
                viewport.world_to_screen(rect, point),
                Align2::LEFT_TOP,
                &tools.label_text,
                FontId::monospace(12.0),
                Color32::from_rgb(0, 95, 180),
            );
        }
        SchematicTool::Text => {
            painter.text(
                viewport.world_to_screen(rect, point),
                Align2::LEFT_TOP,
                &tools.text_item,
                FontId::monospace(12.0),
                Color32::from_rgb(165, 45, 45),
            );
        }
        SchematicTool::Junction => {
            painter.circle_filled(
                viewport.world_to_screen(rect, point),
                3.0,
                Color32::from_rgb(0, 150, 72),
            );
        }
        SchematicTool::NoConnect => {
            draw_no_connect_preview(painter, rect, viewport.world_to_screen(rect, point));
        }
        SchematicTool::Select => {}
    }
}

fn draw_no_connect_preview(painter: &egui::Painter, rect: Rect, center: Pos2) {
    if !rect.contains(center) {
        return;
    }
    let size = 5.0;
    painter.line_segment(
        [
            Pos2::new(center.x - size, center.y - size),
            Pos2::new(center.x + size, center.y + size),
        ],
        Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
    );
    painter.line_segment(
        [
            Pos2::new(center.x - size, center.y + size),
            Pos2::new(center.x + size, center.y - size),
        ],
        Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
    );
}
