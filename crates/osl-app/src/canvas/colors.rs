#![allow(dead_code)]
//! Theme-aware schematic rendering color palette.
//!
//! Provides a `SchematicColors` struct that adapts all element colors to the
//! current theme mode (light or dark). The old constant-based API is retained
//! as a fallback for tests and non-themed code paths.
//!
//! ## Architecture
//!
//! ```text
//! StudioThemeMode → SchematicColors::for_mode(mode)
//!                 → passed to draw_scene / draw_grid / draw_*
//! ```

use crate::app::theme::StudioThemeMode;
use eframe::egui::Color32;

// ---------------------------------------------------------------------------
// Legacy constants (light theme defaults, used in tests and fallbacks)
// ---------------------------------------------------------------------------

pub(crate) const CANVAS_BG: Color32 = Color32::from_rgb(236, 240, 244);
pub(crate) const GRID_MINOR: Color32 = Color32::from_rgb(228, 232, 237);
pub(crate) const GRID_MAJOR: Color32 = Color32::from_rgb(218, 224, 230);
pub(crate) const WIRE: Color32 = Color32::from_rgb(0, 150, 72);
pub(crate) const JUNCTION: Color32 = Color32::from_rgb(0, 150, 72);
pub(crate) const BUS: Color32 = Color32::from_rgb(70, 95, 220);
pub(crate) const BUS_ENTRY: Color32 = BUS;
pub(crate) const SYMBOL_BODY: Color32 = Color32::from_rgb(25, 25, 25);
pub(crate) const SYMBOL_PIN: Color32 = Color32::from_rgb(30, 30, 30);
pub(crate) const SYMBOL_PIN_NAME: Color32 = Color32::from_rgb(50, 50, 50);
pub(crate) const SYMBOL_PIN_NUMBER: Color32 = Color32::from_rgb(100, 100, 100);
pub(crate) const SYMBOL_REFERENCE: Color32 = Color32::from_rgb(25, 25, 25);
pub(crate) const SYMBOL_VALUE: Color32 = Color32::from_rgb(80, 80, 80);
pub(crate) const LABEL_LOCAL: Color32 = Color32::from_rgb(0, 95, 180);
pub(crate) const LABEL_GLOBAL: Color32 = Color32::from_rgb(0, 70, 160);
pub(crate) const LABEL_HIERARCHICAL: Color32 = Color32::from_rgb(128, 0, 128);
pub(crate) const LABEL_DIRECTIVE: Color32 = Color32::from_rgb(150, 65, 20);
pub(crate) const LABEL_DIRECTIVE_BOUNDS: Color32 = Color32::from_rgb(180, 95, 35);
pub(crate) const TEXT: Color32 = Color32::from_rgb(55, 55, 55);
pub(crate) const TEXT_SPICE_DIRECTIVE: Color32 = Color32::from_rgb(165, 45, 45);
pub(crate) const TEXT_BOX_BORDER: Color32 = Color32::from_rgb(120, 120, 120);
pub(crate) const SHEET_FILL: Color32 = Color32::from_rgb(245, 248, 255);
pub(crate) const SHEET_BORDER: Color32 = Color32::from_rgb(90, 120, 190);
pub(crate) const SHEET_NAME: Color32 = Color32::from_rgb(50, 80, 150);
pub(crate) const SHEET_PIN: Color32 = Color32::from_rgb(0, 95, 180);
pub(crate) const RULE_AREA: Color32 = Color32::from_rgb(150, 110, 20);
pub(crate) const NO_CONNECT: Color32 = Color32::from_rgb(55, 55, 55);
pub(crate) const GRAPHIC: Color32 = Color32::from_rgb(90, 90, 90);
pub(crate) const HOVER_HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(20, 120, 220, 80);
pub(crate) const SELECTION: Color32 = Color32::from_rgb(20, 120, 220);
pub(crate) const PLACEMENT_PREVIEW: Color32 = Color32::from_rgb(80, 120, 190);
pub(crate) const HIT_BOUNDS: Color32 = Color32::from_rgb(20, 120, 220);

// ---------------------------------------------------------------------------
// Theme-aware schematic color palette
// ---------------------------------------------------------------------------

/// All colors needed to render a KiCad schematic scene.
///
/// Construct via `SchematicColors::for_mode(mode)` to get the correct palette
/// for the current theme. Dark themes invert light/dark relationships while
/// keeping semantic meaning (green wires, blue buses, etc.).
#[derive(Debug, Clone, Copy)]
pub(crate) struct SchematicColors {
    // Canvas
    pub canvas_bg: Color32,
    pub grid_minor: Color32,
    pub grid_major: Color32,
    // Wires / buses
    pub wire: Color32,
    pub junction: Color32,
    pub bus: Color32,
    pub bus_entry: Color32,
    // Symbols
    pub symbol_body: Color32,
    pub symbol_pin: Color32,
    pub symbol_pin_name: Color32,
    pub symbol_pin_number: Color32,
    pub symbol_reference: Color32,
    pub symbol_value: Color32,
    // Labels / text
    pub label_local: Color32,
    pub label_global: Color32,
    pub label_hierarchical: Color32,
    pub label_directive: Color32,
    pub label_directive_bounds: Color32,
    pub text: Color32,
    pub text_spice_directive: Color32,
    pub text_box_border: Color32,
    // Sheet / annotation
    pub sheet_fill: Color32,
    pub sheet_border: Color32,
    pub sheet_name: Color32,
    pub sheet_pin: Color32,
    pub rule_area: Color32,
    pub no_connect: Color32,
    // Graphics
    pub graphic: Color32,
    // Selection / interaction
    pub hover_highlight: Color32,
    pub selection: Color32,
    pub placement_preview: Color32,
}

impl SchematicColors {
    /// Return the appropriate color palette for the given theme mode.
    pub(crate) fn for_mode(mode: StudioThemeMode) -> Self {
        match mode {
            StudioThemeMode::Light => Self::light(),
            StudioThemeMode::Midnight | StudioThemeMode::Graphite => Self::dark(),
        }
    }

    /// Light theme: KiCad-standard colors on white/light-gray background.
    fn light() -> Self {
        Self {
            canvas_bg: Color32::from_rgb(236, 240, 244),
            grid_minor: Color32::from_rgb(228, 232, 237),
            grid_major: Color32::from_rgb(218, 224, 230),
            wire: Color32::from_rgb(0, 150, 72),
            junction: Color32::from_rgb(0, 150, 72),
            bus: Color32::from_rgb(70, 95, 220),
            bus_entry: Color32::from_rgb(70, 95, 220),
            symbol_body: Color32::from_rgb(25, 25, 25),
            symbol_pin: Color32::from_rgb(30, 30, 30),
            symbol_pin_name: Color32::from_rgb(50, 50, 50),
            symbol_pin_number: Color32::from_rgb(100, 100, 100),
            symbol_reference: Color32::from_rgb(25, 25, 25),
            symbol_value: Color32::from_rgb(80, 80, 80),
            label_local: Color32::from_rgb(0, 95, 180),
            label_global: Color32::from_rgb(0, 70, 160),
            label_hierarchical: Color32::from_rgb(128, 0, 128),
            label_directive: Color32::from_rgb(150, 65, 20),
            label_directive_bounds: Color32::from_rgb(180, 95, 35),
            text: Color32::from_rgb(55, 55, 55),
            text_spice_directive: Color32::from_rgb(165, 45, 45),
            text_box_border: Color32::from_rgb(120, 120, 120),
            sheet_fill: Color32::from_rgb(245, 248, 255),
            sheet_border: Color32::from_rgb(90, 120, 190),
            sheet_name: Color32::from_rgb(50, 80, 150),
            sheet_pin: Color32::from_rgb(0, 95, 180),
            rule_area: Color32::from_rgb(150, 110, 20),
            no_connect: Color32::from_rgb(55, 55, 55),
            graphic: Color32::from_rgb(90, 90, 90),
            hover_highlight: Color32::from_rgba_premultiplied(20, 120, 220, 80),
            selection: Color32::from_rgb(20, 120, 220),
            placement_preview: Color32::from_rgb(80, 120, 190),
        }
    }

    /// Dark theme: bright element colors on dark background.
    ///
    /// Colors are shifted to maintain contrast and readability against
    /// the dark canvas background used in Midnight/Graphite themes.
    fn dark() -> Self {
        Self {
            canvas_bg: Color32::from_rgb(14, 20, 32),
            grid_minor: Color32::from_rgb(24, 32, 48),
            grid_major: Color32::from_rgb(32, 42, 60),
            wire: Color32::from_rgb(50, 210, 120),        // Brighter green wires
            junction: Color32::from_rgb(50, 210, 120),
            bus: Color32::from_rgb(110, 155, 255),         // Brighter blue buses
            bus_entry: Color32::from_rgb(110, 155, 255),
            symbol_body: Color32::from_rgb(200, 212, 228), // Brighter symbol bodies
            symbol_pin: Color32::from_rgb(190, 205, 225),
            symbol_pin_name: Color32::from_rgb(170, 188, 212),
            symbol_pin_number: Color32::from_rgb(130, 152, 180),
            symbol_reference: Color32::from_rgb(200, 212, 228),
            symbol_value: Color32::from_rgb(160, 178, 202),
            label_local: Color32::from_rgb(90, 170, 255),  // Brighter labels
            label_global: Color32::from_rgb(70, 150, 245),
            label_hierarchical: Color32::from_rgb(190, 130, 230),
            label_directive: Color32::from_rgb(220, 170, 70),
            label_directive_bounds: Color32::from_rgb(190, 140, 55),
            text: Color32::from_rgb(190, 205, 225),
            text_spice_directive: Color32::from_rgb(230, 110, 110),
            text_box_border: Color32::from_rgb(110, 128, 155),
            sheet_fill: Color32::from_rgb(20, 28, 42),
            sheet_border: Color32::from_rgb(110, 155, 215),
            sheet_name: Color32::from_rgb(110, 165, 235),
            sheet_pin: Color32::from_rgb(90, 170, 255),
            rule_area: Color32::from_rgb(210, 170, 60),
            no_connect: Color32::from_rgb(190, 205, 225),
            graphic: Color32::from_rgb(150, 168, 195),
            hover_highlight: Color32::from_rgba_premultiplied(60, 150, 255, 100),
            selection: Color32::from_rgb(60, 150, 255),
            placement_preview: Color32::from_rgb(90, 155, 235),
        }
    }
}
