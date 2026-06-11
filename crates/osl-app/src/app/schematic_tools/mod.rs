use super::NekoSpiceApp;
use crate::canvas::colors::SchematicColors;
use eframe::egui::{self, Rect};
use osl_kicad::KicadPoint;

mod controls;
mod editing;
mod preview;
pub(crate) mod state;

pub(crate) use state::SchematicToolState;
pub(crate) use state::SchematicTool;

impl NekoSpiceApp {
    pub(super) fn draw_schematic_tool_preview(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        point: KicadPoint,
        schematic_colors: SchematicColors,
    ) {
        preview::draw_schematic_tool_preview(
            painter,
            rect,
            self.viewport,
            &self.schematic_tools,
            point,
            schematic_colors,
        );
    }
}
