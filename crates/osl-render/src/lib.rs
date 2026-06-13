//! SVG rendering engine for KiCad schematics.

use osl_core::html_escape;
use osl_kicad::{
    KicadAt, KicadBoundingBox, KicadCanvasDirectiveLabel, KicadCanvasGraphic, KicadCanvasImage,
    KicadCanvasRuleArea, KicadCanvasScene, KicadCanvasSheet, KicadCanvasSymbol, KicadCanvasTable,
    KicadCanvasTextBox, KicadColor, KicadFill, KicadLabelKind, KicadPoint, KicadStroke,
    KicadTextEffects, sample_kicad_arc_points,
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

/// render kicad scene svg。
pub fn render_kicad_scene_svg(scene: &KicadCanvasScene) -> String {
    render_kicad_scene_svg_with_options(scene, SvgRenderOptions::default())
}

include!("svg_render_impl.rs");
