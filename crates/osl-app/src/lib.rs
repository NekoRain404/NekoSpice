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
#[cfg(test)]
mod test_support;
mod viewport;

pub use app::{NekoSpiceApp, load_canvas_scene, run_native};

pub(crate) const DEFAULT_SCHEMATIC: &str = "examples/kicad_schematic/rc.kicad_sch";
pub(crate) const DEFAULT_SYMBOL_LIBRARY_TABLE: &str = "examples/kicad_schematic/sym-lib-table";
