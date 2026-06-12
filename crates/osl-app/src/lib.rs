//! Hardware-accelerated NekoSpice GUI shell.
//!
//! The crate keeps GUI composition separate from KiCad document/library adapters
//! and canvas viewport/rendering helpers, so future schematic editing tools can
//! grow without coupling UI widgets directly to KiCad file internals.

mod app;
mod canvas;
mod document;
mod library;
mod placement_config;
mod report_summary;
mod simulation;
mod simulation_run_loader;
#[cfg(test)]
mod test_support;
mod viewport;
mod waveform_summary;

pub use app::{NekoSpiceApp, load_canvas_scene, run_native};

/// Test fixture schematic (RC filter with NekoSpice library symbols).
/// Used by unit tests to verify document loading and editing.
#[allow(dead_code)]
pub(crate) const DEFAULT_SCHEMATIC: &str = "examples/kicad_schematic/rc.kicad_sch";
/// Library table for test fixtures (NekoSpice analog symbols).
#[allow(dead_code)]
pub(crate) const DEFAULT_SYMBOL_LIBRARY_TABLE: &str = "examples/kicad_schematic/sym-lib-table";

/// GUI startup schematic -- CM5 Minima demo board (KiCad demo project).
pub(crate) const DEFAULT_GUI_SCHEMATIC: &str = "examples/cm5_minima/CM5.kicad_sch";
pub(crate) const DEFAULT_GUI_LIBRARY_TABLE: &str = "examples/cm5_minima/sym-lib-table";
