//! SVG rendering engine for schema schematics.

use nsp_core::html_escape;
use nsp_schema::{
    NspAt, NspBoundingBox, NspCanvasDirectiveLabel, NspCanvasGraphic, NspCanvasImage,
    NspCanvasRuleArea, NspCanvasScene, NspCanvasSheet, NspCanvasSymbol, NspCanvasTable,
    NspCanvasTextBox, NspColor, NspFill, NspLabelKind, NspPoint, NspStroke, NspTextEffects,
    sample_arc_points,
};

const DEFAULT_PADDING_MM: f64 = 6.0;
const DEFAULT_SCALE: f64 = 18.0;

#[derive(Debug, Clone, Copy)]
pub struct SvgRenderOptions {
    pub padding_mm: f64,
    pub scale: f64,
    pub show_grid: bool,
}

impl Default for SvgRenderOptions {
    fn default() -> Self {
        Self {
            padding_mm: DEFAULT_PADDING_MM,
            scale: DEFAULT_SCALE,
            show_grid: true,
        }
    }
}

/// render schema scene svg。
pub fn render_schema_scene_svg(scene: &NspCanvasScene) -> String {
    render_schema_scene_svg_with_options(scene, SvgRenderOptions::default())
}

include!("svg_render_impl.rs");
