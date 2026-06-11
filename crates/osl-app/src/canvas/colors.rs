#![allow(dead_code)]
//! Schematic rendering color palette.
//!
//! Maps KiCad element types to display colors following the standard
//! KiCad color scheme for light-background schematic rendering.
//! All colors are optimized for readability on the light canvas background.

use eframe::egui::Color32;

// ---------------------------------------------------------------------------
// Canvas background
// ---------------------------------------------------------------------------

/// Light canvas background color.
pub(crate) const CANVAS_BG: Color32 = Color32::from_rgb(236, 240, 244);

/// Minor grid line color (subtle, unobtrusive).
pub(crate) const GRID_MINOR: Color32 = Color32::from_rgb(228, 232, 237);

/// Major grid line color (slightly more visible).
pub(crate) const GRID_MAJOR: Color32 = Color32::from_rgb(218, 224, 230);

// ---------------------------------------------------------------------------
// Schematic elements
// ---------------------------------------------------------------------------

/// Wire color — KiCad standard green.
pub(crate) const WIRE: Color32 = Color32::from_rgb(0, 150, 72);

/// Junction dot color — same green as wires.
pub(crate) const JUNCTION: Color32 = Color32::from_rgb(0, 150, 72);

/// Bus color — blue, distinct from wires.
pub(crate) const BUS: Color32 = Color32::from_rgb(70, 95, 220);

/// Bus entry color.
pub(crate) const BUS_ENTRY: Color32 = BUS;

// ---------------------------------------------------------------------------
// Symbols
// ---------------------------------------------------------------------------

/// Symbol graphic body color (polygons, rectangles, circles, arcs).
pub(crate) const SYMBOL_BODY: Color32 = Color32::from_rgb(25, 25, 25);

/// Symbol pin line color.
pub(crate) const SYMBOL_PIN: Color32 = Color32::from_rgb(30, 30, 30);

/// Symbol pin name color.
pub(crate) const SYMBOL_PIN_NAME: Color32 = Color32::from_rgb(50, 50, 50);

/// Symbol pin number color.
pub(crate) const SYMBOL_PIN_NUMBER: Color32 = Color32::from_rgb(100, 100, 100);

/// Symbol reference designator color (e.g. R1, C2, U3).
pub(crate) const SYMBOL_REFERENCE: Color32 = Color32::from_rgb(25, 25, 25);

/// Symbol value label color (e.g. 10k, 100nF).
pub(crate) const SYMBOL_VALUE: Color32 = Color32::from_rgb(80, 80, 80);

// ---------------------------------------------------------------------------
// Labels and text
// ---------------------------------------------------------------------------

/// Local net label color.
pub(crate) const LABEL_LOCAL: Color32 = Color32::from_rgb(0, 95, 180);

/// Global label color.
pub(crate) const LABEL_GLOBAL: Color32 = Color32::from_rgb(0, 70, 160);

/// Hierarchical label color.
pub(crate) const LABEL_HIERARCHICAL: Color32 = Color32::from_rgb(128, 0, 128);

/// Directive label / netclass flag color.
pub(crate) const LABEL_DIRECTIVE: Color32 = Color32::from_rgb(150, 65, 20);

/// Directive label background bounds color.
pub(crate) const LABEL_DIRECTIVE_BOUNDS: Color32 = Color32::from_rgb(180, 95, 35);

/// Free text color.
pub(crate) const TEXT: Color32 = Color32::from_rgb(55, 55, 55);

/// SPICE directive text color (distinct red).
pub(crate) const TEXT_SPICE_DIRECTIVE: Color32 = Color32::from_rgb(165, 45, 45);

/// Text box border color.
pub(crate) const TEXT_BOX_BORDER: Color32 = Color32::from_rgb(120, 120, 120);

// ---------------------------------------------------------------------------
// Sheet and annotation
// ---------------------------------------------------------------------------

/// Hierarchical sheet fill color.
pub(crate) const SHEET_FILL: Color32 = Color32::from_rgb(245, 248, 255);

/// Hierarchical sheet border color.
pub(crate) const SHEET_BORDER: Color32 = Color32::from_rgb(90, 120, 190);

/// Hierarchical sheet name label color.
pub(crate) const SHEET_NAME: Color32 = Color32::from_rgb(50, 80, 150);

/// Sheet pin label color.
pub(crate) const SHEET_PIN: Color32 = Color32::from_rgb(0, 95, 180);

/// Rule area border color.
pub(crate) const RULE_AREA: Color32 = Color32::from_rgb(150, 110, 20);

/// No-connect marker color.
pub(crate) const NO_CONNECT: Color32 = Color32::from_rgb(55, 55, 55);

// ---------------------------------------------------------------------------
// Scene-level graphic colors
// ---------------------------------------------------------------------------

/// Top-level graphic element color (not part of a symbol).
pub(crate) const GRAPHIC: Color32 = Color32::from_rgb(90, 90, 90);

// ---------------------------------------------------------------------------
// Selection and interaction
// ---------------------------------------------------------------------------

/// Selection highlight color.
pub(crate) const SELECTION: Color32 = Color32::from_rgb(20, 120, 220);

/// Symbol placement preview color.
pub(crate) const PLACEMENT_PREVIEW: Color32 = Color32::from_rgb(80, 120, 190);

/// Hit-test bounds highlight color.
pub(crate) const HIT_BOUNDS: Color32 = Color32::from_rgb(20, 120, 220);
