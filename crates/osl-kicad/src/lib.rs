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
    KicadCanvasBus, KicadCanvasBusEntry, KicadCanvasDirectiveLabel, KicadCanvasGraphic,
    KicadCanvasGroup, KicadCanvasImage, KicadCanvasJunction, KicadCanvasLabel,
    KicadCanvasNoConnect, KicadCanvasPin, KicadCanvasRuleArea, KicadCanvasScene, KicadCanvasSheet,
    KicadCanvasSheetPin, KicadCanvasSymbol, KicadCanvasTable, KicadCanvasTableCell,
    KicadCanvasText, KicadCanvasTextBox, KicadCanvasWire,
};
pub use canvas_hit::{KicadCanvasHit, KicadCanvasHitReport};
pub use connectivity::{KicadNet, KicadNetGraph};
pub use coordinates::{KicadAt, KicadPoint, KicadSize};
pub use diagnostics::{
    KicadDiagnosticSeverity, KicadHierarchyNetlist, KicadSchematicCheckReport,
    KicadSchematicDiagnostic,
};
pub use edit::{KicadEditSummary, KicadSchematicEdit, KicadSymbolPlacement};
pub use geometry::{KicadBoundingBox, sample_kicad_arc_points};
pub use graphics::{KicadGraphic, KicadRuleArea, KicadSchematicGraphic, KicadSymbolGraphic};
pub use group::KicadGroup;
pub use image::KicadImage;
pub use instances::{
    KicadInstancePath, KicadProjectInstance, KicadSheetInstance, KicadSymbolPathInstance,
    KicadVariantInstance,
};
pub use labels::{KicadDirectiveLabel, KicadLabel, KicadLabelKind};
pub use library_index::{
    KicadIndexedLibrary, KicadIndexedSymbol, KicadIndexedSymbolBodyStyle, KicadIndexedSymbolPin,
    KicadIndexedSymbolUnit, KicadLibraryDiagnostic, KicadSymbolLibraryIndex,
    KicadSymbolLibraryIndexQuery,
};
pub use markers::{KicadJunction, KicadNoConnect};
pub use metadata::{KicadTitleBlock, KicadTitleComment};
pub use pins::{KicadPinAlternate, KicadPinDef, KicadPinDisplay, KicadPinText, KicadSymbolPinRef};
pub use project::{KicadProject, KicadProjectSheet, parse_kicad_project};
pub use property::KicadProperty;
pub use schematic_io::{
    parse_kicad_schematic, read_kicad_schematic, read_kicad_schematic_with_libraries,
    write_kicad_schematic,
};
pub use sexpr::{Sexp, parse_sexpr};
pub use sheet::{KicadSheet, KicadSheetPin};
pub use simulation::{
    KicadSimulationDirective, KicadSimulationDirectiveKind, KicadSimulationDirectiveUpdate,
};
pub use style::{KicadColor, KicadFill, KicadMargins, KicadStroke, KicadTextEffects};
pub use symbol_library::{
    KicadSymbolLibrary, KicadSymbolLibraryTable, KicadSymbolLibraryTableRow,
    parse_kicad_symbol_library, parse_kicad_symbol_library_table,
};
pub use symbols::{KicadSymbolBodyStyles, KicadSymbolDef, KicadSymbolInstance, KicadSymbolPower};
pub use table::{KicadTable, KicadTableBorder, KicadTableCell, KicadTableSeparators};
pub use text::{KicadTextBox, KicadTextItem};
pub use transform::normalize_symbol_mirror;
pub use wiring::{
    KicadBus, KicadBusAlias, KicadBusEntry, KicadNetChain, KicadNetChainEndpoint, KicadWire,
};

use connectivity::{coordinate_key, same_point, same_size};
pub(crate) use coordinates::{
    kicad_at_value, kicad_point_value, kicad_points_value, kicad_size_value, parse_size,
};
use diagnostics::kicad_schematic_diagnostic;
use edit::{
    delete_summary, fnv1a64, is_valid_bus_entry_size, move_sheet_pin_by_uuid, move_summary,
    move_table_cell_by_uuid, points_payload, remove_by_uuid, remove_sheet_pin_by_uuid,
    remove_table_cell_by_uuid, translate_at, translate_graphic, translate_optional_at,
    translate_optional_point, translate_point, translate_points, translate_properties,
    uuid_from_hashes, validate_at, validate_bus_entry_size, validate_point, validate_size,
};
#[cfg(test)]
use geometry::{KICAD_CANVAS_LINE_BOUNDS_PADDING, KICAD_CANVAS_POINT_BOUNDS_RADIUS};
use osl_core::{OslError, OslResult, read_text, write_text};
use pins::{compare_pin_numbers, kicad_pin_alternate_value, kicad_pin_display_value};
use sexpr::format_number;
use sheet::sheet_properties;
use simulation::is_spice_analysis_directive_text;
use spice_export::spice_primitive_for_device;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
pub(crate) use style::{
    default_kicad_text_effects, kicad_color_value, kicad_fill_value, kicad_margins_value,
    kicad_stroke_value, kicad_text_effects_value,
};
pub(crate) use symbols::{
    KicadResolvedSymbolDef, find_symbol_inheritance_parent, library_symbol_definition_for_lib_id,
    qualify_library_symbol_name, resolve_symbol_definition, symbol_instance_properties,
    symbol_item_scope_matches, symbol_sim_pin_order,
};
use transform::transform_symbol_point;
use util::resolve_kicad_uri;

/// read kicad schematic hierarchy netlist。
pub fn read_kicad_schematic_hierarchy_netlist(path: &Path) -> OslResult<KicadHierarchyNetlist> {
    let schematic = read_kicad_schematic_with_libraries(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    schematic.to_spice_netlist_with_hierarchy(base_dir)
}

/// read kicad project。
pub fn read_kicad_project(path: &Path) -> OslResult<KicadProject> {
    let content = read_text(path)?;
    parse_kicad_project(&content, &path.display().to_string())
}

/// read kicad symbol library。
pub fn read_kicad_symbol_library(path: &Path) -> OslResult<KicadSymbolLibrary> {
    let content = read_text(path)?;
    parse_kicad_symbol_library(&content, &path.display().to_string())
}

/// write kicad symbol library。
pub fn write_kicad_symbol_library(path: &Path, library: &KicadSymbolLibrary) -> OslResult<()> {
    write_text(path, &library.to_kicad_symbol_library_sexpr())
}

/// read kicad symbol library table。
pub fn read_kicad_symbol_library_table(path: &Path) -> OslResult<KicadSymbolLibraryTable> {
    let content = read_text(path)?;
    parse_kicad_symbol_library_table(&content, &path.display().to_string())
}

/// read kicad symbol library index。
pub fn read_kicad_symbol_library_index(path: &Path) -> OslResult<KicadSymbolLibraryIndex> {
    let table = read_kicad_symbol_library_table(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    Ok(KicadSymbolLibraryIndex::from_table(table, base_dir))
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematic {
    pub source: String,
    pub version: Option<String>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub uuid: Option<String>,
    pub paper: Option<String>,
    pub title_block: Option<KicadTitleBlock>,
    pub library_symbols: Vec<KicadSymbolDef>,
    pub bus_aliases: Vec<KicadBusAlias>,
    pub symbols: Vec<KicadSymbolInstance>,
    pub wires: Vec<KicadWire>,
    pub buses: Vec<KicadBus>,
    pub bus_entries: Vec<KicadBusEntry>,
    pub net_chains: Vec<KicadNetChain>,
    pub graphics: Vec<KicadSchematicGraphic>,
    pub images: Vec<KicadImage>,
    pub tables: Vec<KicadTable>,
    pub rule_areas: Vec<KicadRuleArea>,
    pub groups: Vec<KicadGroup>,
    pub directive_labels: Vec<KicadDirectiveLabel>,
    pub labels: Vec<KicadLabel>,
    pub sheets: Vec<KicadSheet>,
    pub no_connects: Vec<KicadNoConnect>,
    pub text_items: Vec<KicadTextItem>,
    pub text_boxes: Vec<KicadTextBox>,
    pub junctions: Vec<KicadJunction>,
    pub sheet_instances: Vec<KicadSheetInstance>,
    pub symbol_instances: Vec<KicadSymbolPathInstance>,
    pub embedded_fonts: Option<bool>,
}


include!("schematic_edit_impl.rs");
include!("schematic_library_impl.rs");
include!("schematic_check_impl.rs");
include!("schematic_util_impl.rs");


fn library_symbol_definitions_are_compatible(
    existing: &KicadSymbolDef,
    incoming: &KicadSymbolDef,
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

fn normalize_default_property_effects(symbol: &mut KicadSymbolDef) {
    for property in &mut symbol.properties {
        if property.effects.is_none() {
            property.effects = Some(default_kicad_text_effects());
        }
    }
}

#[cfg(test)]
mod tests;
