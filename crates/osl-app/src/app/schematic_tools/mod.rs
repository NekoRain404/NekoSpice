use super::NekoSpiceApp;
use eframe::egui::{self, Rect};
use osl_kicad::KicadPoint;

mod controls;
mod editing;
mod preview;
mod state;

pub(crate) use state::SchematicToolState;

impl NekoSpiceApp {
    pub(super) fn draw_schematic_tool_preview(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        point: KicadPoint,
    ) {
        preview::draw_schematic_tool_preview(
            painter,
            rect,
            self.viewport,
            &self.schematic_tools,
            point,
        );
    }
}
