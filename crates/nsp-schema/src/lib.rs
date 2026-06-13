//! schema file format parser and editor, implemented in Rust.
//!
//! Provides S-expression parsing, schematic read/write, symbol library indexing,
//! canvas scene generation, and SPICE netlist export.

mod canvas;
mod canvas_hit;
mod connectivity;
mod coordinates;
mod diagnostics;
mod edit;
mod geometry;
mod graphics;
mod group;
mod image;
mod instances;
mod json;
mod labels;
mod library_index;
mod markers;
mod metadata;
mod new_schematic;
mod pins;
mod project;
mod property;
mod schematic_io;
mod schematic_summary;
mod sexpr;
mod sheet;
mod simulation;
mod spice_export;
mod style;
mod symbol_library;
mod symbols;
mod table;
mod text;
mod transform;
mod util;
mod wiring;

pub use canvas::{
    NspCanvasBus, NspCanvasBusEntry, NspCanvasDirectiveLabel, NspCanvasGraphic, NspCanvasGroup,
    NspCanvasImage, NspCanvasJunction, NspCanvasLabel, NspCanvasNoConnect, NspCanvasPin,
    NspCanvasRuleArea, NspCanvasScene, NspCanvasSheet, NspCanvasSheetPin, NspCanvasSymbol,
    NspCanvasTable, NspCanvasTableCell, NspCanvasText, NspCanvasTextBox, NspCanvasWire,
};
pub use canvas_hit::{NspCanvasHit, NspCanvasHitReport};
pub use connectivity::{NspNet, NspNetGraph};
pub use coordinates::{NspAt, NspPoint, NspSize};
pub use diagnostics::{
    NspDiagnosticSeverity, NspHierarchyNetlist, NspSchematicCheckReport, NspSchematicDiagnostic,
};
pub use edit::{NspEditSummary, NspSchematicEdit, NspSymbolPlacement};
pub use geometry::{NspBoundingBox, sample_arc_points};
pub use graphics::{NspGraphic, NspRuleArea, NspSchematicGraphic, NspSymbolGraphic};
pub use group::NspGroup;
pub use image::NspImage;
pub use instances::{
    NspInstancePath, NspProjectInstance, NspSheetInstance, NspSymbolPathInstance,
    NspVariantInstance,
};
pub use labels::{NspDirectiveLabel, NspLabel, NspLabelKind};
pub use library_index::{
    NspIndexedLibrary, NspIndexedSymbol, NspIndexedSymbolBodyStyle, NspIndexedSymbolPin,
    NspIndexedSymbolUnit, NspLibraryDiagnostic, NspSymbolLibraryIndex, NspSymbolLibraryIndexQuery,
};
pub use markers::{NspJunction, NspNoConnect};
pub use metadata::{NspTitleBlock, NspTitleComment};
pub use pins::{NspPinAlternate, NspPinDef, NspPinDisplay, NspPinText, NspSymbolPinRef};
pub use project::{NspProject, NspProjectSheet, parse_project};
pub use property::NspProperty;
pub use schematic_io::{
    parse_schematic, read_schematic, read_schematic_with_libraries, write_schematic,
};
pub use sexpr::{Sexp, parse_sexpr};
pub use sheet::{NspSheet, NspSheetPin};
pub use simulation::{
    NspSimulationDirective, NspSimulationDirectiveKind, NspSimulationDirectiveUpdate,
};
pub use style::{NspColor, NspFill, NspMargins, NspStroke, NspTextEffects};
pub use symbol_library::{
    NspSymbolLibrary, NspSymbolLibraryTable, NspSymbolLibraryTableRow, parse_symbol_library,
    parse_symbol_library_table,
};
pub use symbols::{NspSymbolBodyStyles, NspSymbolDef, NspSymbolInstance, NspSymbolPower};
pub use table::{NspTable, NspTableBorder, NspTableCell, NspTableSeparators};
pub use text::{NspTextBox, NspTextItem};
pub use transform::normalize_symbol_mirror;
pub use wiring::{NspBus, NspBusAlias, NspBusEntry, NspNetChain, NspNetChainEndpoint, NspWire};

use connectivity::{coordinate_key, same_point, same_size};
pub(crate) use coordinates::{
    parse_size, schema_at_value, schema_point_value, schema_points_value, schema_size_value,
};
use diagnostics::schema_diagnostic;
use edit::{
    delete_summary, fnv1a64, is_valid_bus_entry_size, move_sheet_pin_by_uuid, move_summary,
    move_table_cell_by_uuid, points_payload, remove_by_uuid, remove_sheet_pin_by_uuid,
    remove_table_cell_by_uuid, translate_at, translate_graphic, translate_optional_at,
    translate_optional_point, translate_point, translate_points, translate_properties,
    uuid_from_hashes, validate_at, validate_bus_entry_size, validate_point, validate_size,
};
use nsp_core::{OslError, OslResult, read_text, write_text};
use pins::{compare_pin_numbers, schema_pin_alternate_value, schema_pin_display_value};
use sexpr::format_number;
use sheet::sheet_properties;
use simulation::is_spice_analysis_directive_text;
use spice_export::spice_primitive_for_device;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
pub(crate) use style::{
    default_schema_text_effects, schema_color_value, schema_fill_value, schema_margins_value,
    schema_stroke_value, schema_text_effects_value,
};
pub(crate) use symbols::{
    NspResolvedSymbolDef, find_symbol_inheritance_parent, library_symbol_definition_for_lib_id,
    qualify_library_symbol_name, resolve_symbol_definition, symbol_instance_properties,
    symbol_item_scope_matches, symbol_sim_pin_order,
};
use transform::transform_symbol_point;
use util::resolve_uri;

/// read schema schematic hierarchy netlist。
pub fn read_schematic_hierarchy_netlist(path: &Path) -> OslResult<NspHierarchyNetlist> {
    let schematic = read_schematic_with_libraries(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    schematic.to_spice_netlist_with_hierarchy(base_dir)
}

/// read schema project。
pub fn read_project(path: &Path) -> OslResult<NspProject> {
    let content = read_text(path)?;
    parse_project(&content, &path.display().to_string())
}

/// read schema symbol library。
pub fn read_symbol_library(path: &Path) -> OslResult<NspSymbolLibrary> {
    let content = read_text(path)?;
    parse_symbol_library(&content, &path.display().to_string())
}

/// write schema symbol library。
pub fn write_symbol_library(path: &Path, library: &NspSymbolLibrary) -> OslResult<()> {
    write_text(path, &library.to_symbol_library_sexpr())
}

/// read schema symbol library table。
pub fn read_symbol_library_table(path: &Path) -> OslResult<NspSymbolLibraryTable> {
    let content = read_text(path)?;
    parse_symbol_library_table(&content, &path.display().to_string())
}

/// read schema symbol library index。
pub fn read_symbol_library_index(path: &Path) -> OslResult<NspSymbolLibraryIndex> {
    let table = read_symbol_library_table(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    Ok(NspSymbolLibraryIndex::from_table(table, base_dir))
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspSchematic {
    pub source: String,
    pub version: Option<String>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub uuid: Option<String>,
    pub paper: Option<String>,
    pub title_block: Option<NspTitleBlock>,
    pub library_symbols: Vec<NspSymbolDef>,
    pub bus_aliases: Vec<NspBusAlias>,
    pub symbols: Vec<NspSymbolInstance>,
    pub wires: Vec<NspWire>,
    pub buses: Vec<NspBus>,
    pub bus_entries: Vec<NspBusEntry>,
    pub net_chains: Vec<NspNetChain>,
    pub graphics: Vec<NspSchematicGraphic>,
    pub images: Vec<NspImage>,
    pub tables: Vec<NspTable>,
    pub rule_areas: Vec<NspRuleArea>,
    pub groups: Vec<NspGroup>,
    pub directive_labels: Vec<NspDirectiveLabel>,
    pub labels: Vec<NspLabel>,
    pub sheets: Vec<NspSheet>,
    pub no_connects: Vec<NspNoConnect>,
    pub text_items: Vec<NspTextItem>,
    pub text_boxes: Vec<NspTextBox>,
    pub junctions: Vec<NspJunction>,
    pub sheet_instances: Vec<NspSheetInstance>,
    pub symbol_instances: Vec<NspSymbolPathInstance>,
    pub embedded_fonts: Option<bool>,
}

include!("schematic_edit_impl.rs");
include!("schematic_library_impl.rs");
include!("schematic_check_impl.rs");
include!("schematic_util_impl.rs");

fn library_symbol_definitions_are_compatible(
    existing: &NspSymbolDef,
    incoming: &NspSymbolDef,
) -> bool {
    if existing == incoming {
        return true;
    }

    let mut existing = existing.clone();
    let mut incoming = incoming.clone();
    normalize_default_property_effects(&mut existing);
    normalize_default_property_effects(&mut incoming);
    existing == incoming
}

fn normalize_default_property_effects(symbol: &mut NspSymbolDef) {
    for property in &mut symbol.properties {
        if property.effects.is_none() {
            property.effects = Some(default_schema_text_effects());
        }
    }
}

#[cfg(test)]
mod tests;
pub use new_schematic::new_schema_empty;
