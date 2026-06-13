//! 绘图工具状态机模块。管理工具的激活、切换和预览渲染。
//!
use crate::app::NekoSpiceApp;
use crate::canvas::colors::SchematicColors;
use eframe::egui::{self, Rect};
use osl_kicad::KicadPoint;

mod controls;
mod editing;
mod preview;
pub(crate) mod state;

pub(crate) use state::SchematicTool;
pub(crate) use state::SchematicToolState;

impl NekoSpiceApp {
    /// draw schematic tool preview。
    pub(crate) fn draw_schematic_tool_preview(
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
