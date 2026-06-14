//! Hardware-accelerated NekoSpice GUI shell.
//!
//! The crate keeps GUI composition separate from schematic document/library adapters
//! and canvas viewport/rendering helpers, so future schematic editing tools can
//! grow without coupling UI widgets directly to schematic file internals.

mod app;
mod canvas;
mod document;
mod document_ops;
mod library;
mod placement_config;
mod report_summary;
mod simulation;
mod simulation_run_loader;
#[cfg(test)]
mod test_support;
mod viewport;
mod waveform_summary;

pub use app::{NekoSpiceApp, load_canvas_scene, run_native, run_native_with_boxed};

/// 测试用原理图路径（RC 滤波器，含 NekoSpice 符号库）。
#[allow(dead_code)]
pub(crate) const DEFAULT_SCHEMATIC: &str = "examples/schema_schematic/rc.nsp_sch";
/// 测试用符号库表路径。
#[allow(dead_code)]
pub(crate) const DEFAULT_SYMBOL_LIBRARY_TABLE: &str = "examples/schema_schematic/sym-lib-table";

/// GUI 启动时默认加载的原理图（CM5 Minima 演示板）。
pub(crate) const DEFAULT_GUI_SCHEMATIC: &str = "examples/cm5_minima/CM5.nsp_sch";
/// GUI 启动时默认加载的符号库表。
pub(crate) const DEFAULT_GUI_LIBRARY_TABLE: &str = "examples/cm5_minima/sym-lib-table";
