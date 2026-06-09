mod canvas;
mod canvas_hit;
mod edit;
mod geometry;
mod json;
mod library_index;
mod project;
mod sexpr;

pub use canvas::{
    KicadCanvasBus, KicadCanvasBusEntry, KicadCanvasDirectiveLabel, KicadCanvasGraphic,
    KicadCanvasGroup, KicadCanvasImage, KicadCanvasJunction, KicadCanvasLabel,
    KicadCanvasNoConnect, KicadCanvasPin, KicadCanvasRuleArea, KicadCanvasScene, KicadCanvasSheet,
    KicadCanvasSheetPin, KicadCanvasSymbol, KicadCanvasTable, KicadCanvasTableCell,
    KicadCanvasText, KicadCanvasTextBox, KicadCanvasWire,
};
pub use canvas_hit::{KicadCanvasHit, KicadCanvasHitReport};
pub use edit::{KicadEditSummary, KicadSchematicEdit, KicadSymbolPlacement};
pub use geometry::{KicadBoundingBox, sample_kicad_arc_points};
pub use library_index::{
    KicadIndexedLibrary, KicadIndexedSymbol, KicadIndexedSymbolBodyStyle, KicadIndexedSymbolPin,
    KicadIndexedSymbolUnit, KicadLibraryDiagnostic, KicadSymbolLibraryIndex,
    KicadSymbolLibraryIndexQuery,
};
pub use project::{KicadProject, KicadProjectSheet, parse_kicad_project};
pub use sexpr::{Sexp, parse_sexpr};

use edit::{
    delete_summary, fnv1a64, is_valid_bus_entry_size, move_sheet_pin_by_uuid, move_summary,
    move_table_cell_by_uuid, points_payload, remove_by_uuid, remove_sheet_pin_by_uuid,
    remove_table_cell_by_uuid, translate_at, translate_graphic, translate_optional_at,
    translate_optional_point, translate_point, translate_points, translate_properties,
    uuid_from_hashes, validate_at, validate_bus_entry_size, validate_point, validate_size,
};
#[cfg(test)]
use geometry::KICAD_CANVAS_POINT_BOUNDS_RADIUS;
use geometry::{
    KICAD_CANVAS_LINE_BOUNDS_PADDING, KicadBoundingBoxBuilder, kicad_points_bounds,
    kicad_rotated_rect_bounds, pin_body_end, rotate_point,
};
use json::{json_bool_option, json_option};
use osl_core::{OslError, OslResult, json_escape, read_text, write_text};
use sexpr::{
    atom_text, child, child_value, direct_children, expect_root_list, format_number, head,
    list_items, list_value, sexpr_atom_or_string, sexpr_string, write_sexpr_inline,
};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
pub fn read_kicad_schematic(path: &Path) -> OslResult<KicadSchematic> {
    let content = read_text(path)?;
    parse_kicad_schematic(&content, &path.display().to_string())
}

pub fn read_kicad_schematic_with_libraries(path: &Path) -> OslResult<KicadSchematic> {
    let mut schematic = read_kicad_schematic(path)?;
    if let Some(project_dir) = path.parent() {
        schematic.resolve_project_symbol_libraries(project_dir)?;
    }
    Ok(schematic)
}

pub fn read_kicad_schematic_hierarchy_netlist(path: &Path) -> OslResult<KicadHierarchyNetlist> {
    let schematic = read_kicad_schematic_with_libraries(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    schematic.to_spice_netlist_with_hierarchy(base_dir)
}

pub fn read_kicad_project(path: &Path) -> OslResult<KicadProject> {
    let content = read_text(path)?;
    parse_kicad_project(&content, &path.display().to_string())
}

pub fn write_kicad_schematic(path: &Path, schematic: &KicadSchematic) -> OslResult<()> {
    write_text(path, &schematic.to_kicad_schematic_sexpr())
}

pub fn read_kicad_symbol_library(path: &Path) -> OslResult<KicadSymbolLibrary> {
    let content = read_text(path)?;
    parse_kicad_symbol_library(&content, &path.display().to_string())
}

pub fn write_kicad_symbol_library(path: &Path, library: &KicadSymbolLibrary) -> OslResult<()> {
    write_text(path, &library.to_kicad_symbol_library_sexpr())
}

pub fn read_kicad_symbol_library_table(path: &Path) -> OslResult<KicadSymbolLibraryTable> {
    let content = read_text(path)?;
    parse_kicad_symbol_library_table(&content, &path.display().to_string())
}

pub fn read_kicad_symbol_library_index(path: &Path) -> OslResult<KicadSymbolLibraryIndex> {
    let table = read_kicad_symbol_library_table(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    Ok(KicadSymbolLibraryIndex::from_table(table, base_dir))
}

pub fn parse_kicad_schematic(input: &str, source: &str) -> OslResult<KicadSchematic> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "kicad_sch")?;
    let library_symbols = direct_children(root_list, "lib_symbols")
        .flat_map(|lib_symbols| direct_children(list_items(lib_symbols), "symbol"))
        .filter_map(parse_symbol_def)
        .collect::<Vec<_>>();

    Ok(KicadSchematic {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        generator: child_value(root_list, "generator"),
        generator_version: child_value(root_list, "generator_version"),
        uuid: child_value(root_list, "uuid"),
        paper: child_value(root_list, "paper"),
        title_block: child(root_list, "title_block").map(parse_title_block),
        library_symbols,
        bus_aliases: direct_children(root_list, "bus_alias")
            .filter_map(parse_bus_alias)
            .collect(),
        symbols: direct_children(root_list, "symbol")
            .filter_map(parse_symbol_instance)
            .collect(),
        wires: direct_children(root_list, "wire")
            .map(parse_wire)
            .collect::<Vec<_>>(),
        buses: direct_children(root_list, "bus")
            .map(parse_bus)
            .collect::<Vec<_>>(),
        bus_entries: direct_children(root_list, "bus_entry")
            .filter_map(parse_bus_entry)
            .collect(),
        net_chains: direct_children(root_list, "net_chain")
            .filter_map(parse_net_chain)
            .collect(),
        graphics: root_list
            .iter()
            .filter_map(parse_schematic_graphic)
            .collect(),
        images: direct_children(root_list, "image")
            .filter_map(parse_image)
            .collect(),
        tables: direct_children(root_list, "table")
            .filter_map(parse_table)
            .collect(),
        rule_areas: direct_children(root_list, "rule_area")
            .filter_map(parse_rule_area)
            .collect(),
        groups: direct_children(root_list, "group")
            .filter_map(parse_group)
            .collect(),
        directive_labels: direct_children(root_list, "netclass_flag")
            .filter_map(parse_directive_label)
            .collect(),
        labels: direct_children(root_list, "label")
            .filter_map(|node| parse_label(node, KicadLabelKind::Local))
            .chain(
                direct_children(root_list, "global_label")
                    .filter_map(|node| parse_label(node, KicadLabelKind::Global)),
            )
            .chain(
                direct_children(root_list, "hierarchical_label")
                    .filter_map(|node| parse_label(node, KicadLabelKind::Hierarchical)),
            )
            .collect(),
        sheets: direct_children(root_list, "sheet")
            .filter_map(parse_sheet)
            .collect(),
        no_connects: direct_children(root_list, "no_connect")
            .filter_map(parse_no_connect)
            .collect(),
        text_items: direct_children(root_list, "text")
            .filter_map(parse_text_item)
            .collect(),
        text_boxes: direct_children(root_list, "text_box")
            .filter_map(parse_text_box)
            .collect(),
        junctions: direct_children(root_list, "junction")
            .filter_map(parse_junction)
            .collect(),
        sheet_instances: child(root_list, "sheet_instances")
            .map(parse_sheet_instances)
            .unwrap_or_default(),
        symbol_instances: child(root_list, "symbol_instances")
            .map(parse_symbol_path_instances)
            .unwrap_or_default(),
        embedded_fonts: child_value(root_list, "embedded_fonts").and_then(parse_kicad_bool_value),
    })
}

pub fn parse_kicad_symbol_library(input: &str, source: &str) -> OslResult<KicadSymbolLibrary> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "kicad_symbol_lib")?;

    Ok(KicadSymbolLibrary {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        generator: child_value(root_list, "generator"),
        generator_version: child_value(root_list, "generator_version"),
        symbols: direct_children(root_list, "symbol")
            .filter_map(parse_symbol_def)
            .collect(),
    })
}

pub fn parse_kicad_symbol_library_table(
    input: &str,
    source: &str,
) -> OslResult<KicadSymbolLibraryTable> {
    let root = parse_sexpr(input)?;
    let root_list = expect_root_list(&root, "sym_lib_table")?;

    Ok(KicadSymbolLibraryTable {
        source: source.to_string(),
        version: child_value(root_list, "version"),
        libraries: direct_children(root_list, "lib")
            .filter_map(parse_symbol_library_table_row)
            .collect(),
    })
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

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematicCheckReport {
    pub source: String,
    pub symbol_count: usize,
    pub sheet_count: usize,
    pub net_count: usize,
    pub spice_directive_count: usize,
    pub diagnostics: Vec<KicadSchematicDiagnostic>,
}

impl KicadSchematicCheckReport {
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == KicadDiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == KicadDiagnosticSeverity::Warning)
            .count()
    }

    pub fn info_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == KicadDiagnosticSeverity::Info)
            .count()
    }

    pub fn to_json(&self) -> String {
        let diagnostics = self
            .diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    concat!(
                        "    {{ \"severity\": \"{}\", \"code\": \"{}\", ",
                        "\"message\": \"{}\", \"item\": {}, \"net\": {}, \"pin\": {} }}"
                    ),
                    diagnostic.severity.as_str(),
                    json_escape(&diagnostic.code),
                    json_escape(&diagnostic.message),
                    json_option(diagnostic.item.as_deref()),
                    json_option(diagnostic.net.as_deref()),
                    json_option(diagnostic.pin.as_deref())
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"symbol_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"net_count\": {},\n",
                "  \"spice_directive_count\": {},\n",
                "  \"diagnostic_count\": {},\n",
                "  \"error_count\": {},\n",
                "  \"warning_count\": {},\n",
                "  \"info_count\": {},\n",
                "  \"diagnostics\": [\n",
                "{}\n",
                "  ]\n",
                "}}"
            ),
            json_escape(&self.source),
            self.symbol_count,
            self.sheet_count,
            self.net_count,
            self.spice_directive_count,
            self.diagnostics.len(),
            self.error_count(),
            self.warning_count(),
            self.info_count(),
            diagnostics
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadHierarchyNetlist {
    pub netlist: String,
    pub diagnostics: Vec<KicadSchematicDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematicDiagnostic {
    pub severity: KicadDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub item: Option<String>,
    pub net: Option<String>,
    pub pin: Option<String>,
}

impl KicadSchematic {
    pub fn apply_edit(&mut self, edit: KicadSchematicEdit) -> OslResult<KicadEditSummary> {
        match edit {
            KicadSchematicEdit::MoveSymbol {
                reference,
                to,
                rotation,
            } => self.move_symbol(&reference, to, rotation),
            KicadSchematicEdit::MoveItem { uuid, delta } => self.move_item_by_uuid(&uuid, delta),
            KicadSchematicEdit::DeleteItem { uuid } => self.delete_item_by_uuid(&uuid),
            KicadSchematicEdit::ConfigureSymbol {
                reference,
                unit,
                body_style,
                mirror,
                pin_alternates,
            } => self.configure_symbol(&reference, unit, body_style, mirror, pin_alternates),
            KicadSchematicEdit::SetSymbolProperty {
                reference,
                name,
                value,
                at,
            } => self.set_symbol_property(&reference, &name, &value, at),
            KicadSchematicEdit::PlaceSymbol {
                definition,
                library_symbols,
                reference,
                value,
                at,
                unit,
                body_style,
                pin_alternates,
                uuid,
            } => self.place_symbol(KicadSymbolPlacement {
                definition: *definition,
                library_symbols,
                reference,
                value,
                at,
                unit,
                body_style,
                pin_alternates,
                uuid,
            }),
            KicadSchematicEdit::AddWire { points, uuid } => self.add_wire(points, uuid),
            KicadSchematicEdit::AddBus { points, uuid } => self.add_bus(points, uuid),
            KicadSchematicEdit::AddBusEntry { at, size, uuid } => {
                self.add_bus_entry(at, size, uuid)
            }
            KicadSchematicEdit::AddJunction { at, uuid } => self.add_junction(at, uuid),
            KicadSchematicEdit::AddNoConnect { at, uuid } => self.add_no_connect(at, uuid),
            KicadSchematicEdit::AddLabel {
                text,
                kind,
                at,
                uuid,
            } => self.add_label(text, kind, at, uuid),
            KicadSchematicEdit::AddSheet {
                name,
                file,
                at,
                size,
                pins,
                uuid,
            } => self.add_sheet(&name, &file, at, size, pins, uuid),
            KicadSchematicEdit::AddText { text, at, uuid } => self.add_text(text, at, uuid),
        }
    }

    pub fn move_symbol(
        &mut self,
        reference: &str,
        to: KicadPoint,
        rotation: Option<f64>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(to, "symbol target")?;
        let index = self.symbol_index_by_reference(reference)?;
        let symbol = &mut self.symbols[index];
        let old_at = symbol.at.unwrap_or(KicadAt {
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
        });
        let dx = to.x - old_at.x;
        let dy = to.y - old_at.y;
        symbol.at = Some(KicadAt {
            x: to.x,
            y: to.y,
            rotation: rotation.unwrap_or(old_at.rotation),
        });

        for property in &mut symbol.properties {
            if let Some(at) = &mut property.at {
                at.x += dx;
                at.y += dy;
            }
        }

        Ok(KicadEditSummary {
            operation: "move-symbol".to_string(),
            target: reference.to_string(),
        })
    }

    pub fn move_item_by_uuid(
        &mut self,
        uuid: &str,
        delta: KicadPoint,
    ) -> OslResult<KicadEditSummary> {
        let uuid = uuid.trim();
        if uuid.is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad move-item UUID must not be empty".to_string(),
            ));
        }
        validate_point(delta, "item move delta")?;

        if let Some(symbol) = self
            .symbols
            .iter_mut()
            .find(|symbol| symbol.uuid.as_deref() == Some(uuid))
        {
            if let Some(at) = &mut symbol.at {
                translate_at(at, delta);
            } else {
                symbol.at = Some(KicadAt {
                    x: delta.x,
                    y: delta.y,
                    rotation: 0.0,
                });
            }
            translate_properties(&mut symbol.properties, delta);
            return Ok(move_summary("symbol", uuid));
        }
        if let Some(wire) = self
            .wires
            .iter_mut()
            .find(|wire| wire.uuid.as_deref() == Some(uuid))
        {
            translate_points(&mut wire.points, delta);
            return Ok(move_summary("wire", uuid));
        }
        if let Some(bus) = self
            .buses
            .iter_mut()
            .find(|bus| bus.uuid.as_deref() == Some(uuid))
        {
            translate_points(&mut bus.points, delta);
            return Ok(move_summary("bus", uuid));
        }
        if let Some(entry) = self
            .bus_entries
            .iter_mut()
            .find(|entry| entry.uuid.as_deref() == Some(uuid))
        {
            translate_point(&mut entry.at, delta);
            return Ok(move_summary("bus-entry", uuid));
        }
        if let Some(junction) = self
            .junctions
            .iter_mut()
            .find(|junction| junction.uuid.as_deref() == Some(uuid))
        {
            translate_point(&mut junction.at, delta);
            return Ok(move_summary("junction", uuid));
        }
        if let Some(marker) = self
            .no_connects
            .iter_mut()
            .find(|marker| marker.uuid.as_deref() == Some(uuid))
        {
            translate_point(&mut marker.at, delta);
            return Ok(move_summary("no-connect", uuid));
        }
        if let Some(label) = self
            .labels
            .iter_mut()
            .find(|label| label.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut label.at, delta);
            translate_properties(&mut label.properties, delta);
            return Ok(move_summary("label", uuid));
        }
        if let Some(label) = self
            .directive_labels
            .iter_mut()
            .find(|label| label.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut label.at, delta);
            translate_properties(&mut label.properties, delta);
            return Ok(move_summary("directive-label", uuid));
        }
        if let Some(text) = self
            .text_items
            .iter_mut()
            .find(|text| text.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut text.at, delta);
            return Ok(move_summary("text", uuid));
        }
        if let Some(text_box) = self
            .text_boxes
            .iter_mut()
            .find(|text_box| text_box.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut text_box.at, delta);
            return Ok(move_summary("text-box", uuid));
        }
        if let Some(sheet) = self
            .sheets
            .iter_mut()
            .find(|sheet| sheet.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut sheet.at, delta);
            translate_properties(&mut sheet.properties, delta);
            for pin in &mut sheet.pins {
                translate_optional_at(&mut pin.at, delta);
            }
            return Ok(move_summary("sheet", uuid));
        }
        if move_sheet_pin_by_uuid(&mut self.sheets, uuid, delta) {
            return Ok(move_summary("sheet-pin", uuid));
        }
        if let Some(graphic) = self
            .graphics
            .iter_mut()
            .find(|graphic| graphic.uuid.as_deref() == Some(uuid))
        {
            translate_graphic(&mut graphic.graphic, delta);
            return Ok(move_summary("graphic", uuid));
        }
        if let Some(rule_area) = self
            .rule_areas
            .iter_mut()
            .find(|rule_area| rule_area.uuid.as_deref() == Some(uuid))
        {
            translate_points(&mut rule_area.points, delta);
            return Ok(move_summary("rule-area", uuid));
        }
        if let Some(image) = self
            .images
            .iter_mut()
            .find(|image| image.uuid.as_deref() == Some(uuid))
        {
            translate_optional_point(&mut image.at, delta);
            return Ok(move_summary("image", uuid));
        }
        if let Some(table) = self
            .tables
            .iter_mut()
            .find(|table| table.uuid.as_deref() == Some(uuid))
        {
            for cell in &mut table.cells {
                translate_optional_at(&mut cell.at, delta);
            }
            return Ok(move_summary("table", uuid));
        }
        if move_table_cell_by_uuid(&mut self.tables, uuid, delta) {
            return Ok(move_summary("table-cell", uuid));
        }
        if self
            .groups
            .iter()
            .any(|group| group.uuid.as_deref() == Some(uuid))
        {
            return Err(OslError::InvalidInput(format!(
                "KiCad schematic group UUID '{uuid}' has no geometry; move its member items instead"
            )));
        }

        Err(OslError::InvalidInput(format!(
            "KiCad schematic item UUID '{uuid}' was not found"
        )))
    }

    pub fn delete_item_by_uuid(&mut self, uuid: &str) -> OslResult<KicadEditSummary> {
        let uuid = uuid.trim();
        if uuid.is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad delete-item UUID must not be empty".to_string(),
            ));
        }

        if remove_by_uuid(&mut self.symbols, uuid, |symbol| symbol.uuid.as_deref()) {
            return Ok(delete_summary("symbol", uuid));
        }
        if remove_by_uuid(&mut self.wires, uuid, |wire| wire.uuid.as_deref()) {
            return Ok(delete_summary("wire", uuid));
        }
        if remove_by_uuid(&mut self.buses, uuid, |bus| bus.uuid.as_deref()) {
            return Ok(delete_summary("bus", uuid));
        }
        if remove_by_uuid(&mut self.bus_entries, uuid, |entry| entry.uuid.as_deref()) {
            return Ok(delete_summary("bus-entry", uuid));
        }
        if remove_by_uuid(&mut self.junctions, uuid, |junction| {
            junction.uuid.as_deref()
        }) {
            return Ok(delete_summary("junction", uuid));
        }
        if remove_by_uuid(&mut self.no_connects, uuid, |marker| marker.uuid.as_deref()) {
            return Ok(delete_summary("no-connect", uuid));
        }
        if remove_by_uuid(&mut self.labels, uuid, |label| label.uuid.as_deref()) {
            return Ok(delete_summary("label", uuid));
        }
        if remove_by_uuid(&mut self.directive_labels, uuid, |label| {
            label.uuid.as_deref()
        }) {
            return Ok(delete_summary("directive-label", uuid));
        }
        if remove_by_uuid(&mut self.text_items, uuid, |text| text.uuid.as_deref()) {
            return Ok(delete_summary("text", uuid));
        }
        if remove_by_uuid(&mut self.text_boxes, uuid, |text_box| {
            text_box.uuid.as_deref()
        }) {
            return Ok(delete_summary("text-box", uuid));
        }
        if remove_by_uuid(&mut self.sheets, uuid, |sheet| sheet.uuid.as_deref()) {
            return Ok(delete_summary("sheet", uuid));
        }
        if remove_sheet_pin_by_uuid(&mut self.sheets, uuid) {
            return Ok(delete_summary("sheet-pin", uuid));
        }
        if remove_by_uuid(&mut self.graphics, uuid, |graphic| graphic.uuid.as_deref()) {
            return Ok(delete_summary("graphic", uuid));
        }
        if remove_by_uuid(&mut self.rule_areas, uuid, |rule_area| {
            rule_area.uuid.as_deref()
        }) {
            return Ok(delete_summary("rule-area", uuid));
        }
        if remove_by_uuid(&mut self.images, uuid, |image| image.uuid.as_deref()) {
            return Ok(delete_summary("image", uuid));
        }
        if remove_by_uuid(&mut self.tables, uuid, |table| table.uuid.as_deref()) {
            return Ok(delete_summary("table", uuid));
        }
        if remove_table_cell_by_uuid(&mut self.tables, uuid) {
            return Ok(delete_summary("table-cell", uuid));
        }
        if remove_by_uuid(&mut self.groups, uuid, |group| group.uuid.as_deref()) {
            return Ok(delete_summary("group", uuid));
        }

        Err(OslError::InvalidInput(format!(
            "KiCad schematic item UUID '{uuid}' was not found"
        )))
    }

    pub fn configure_symbol(
        &mut self,
        reference: &str,
        unit: Option<u32>,
        body_style: Option<Option<u32>>,
        mirror: Option<Option<String>>,
        pin_alternates: Option<BTreeMap<String, String>>,
    ) -> OslResult<KicadEditSummary> {
        if unit == Some(0) {
            return Err(OslError::InvalidInput(
                "KiCad symbol unit must be positive".to_string(),
            ));
        }
        if body_style == Some(Some(0)) {
            return Err(OslError::InvalidInput(
                "KiCad symbol body style must be positive".to_string(),
            ));
        }
        let normalized_mirror = match mirror {
            Some(Some(mirror)) => Some(normalize_symbol_mirror(&mirror)?),
            Some(None) => Some(None),
            None => None,
        };

        let index = self.symbol_index_by_reference(reference)?;
        let current_symbol = self.symbols[index].clone();
        let definition = self
            .resolved_symbol_definition(&current_symbol.lib_id)
            .ok_or_else(|| {
                OslError::InvalidInput(format!(
                    "KiCad symbol reference '{reference}' uses missing library symbol '{}'",
                    current_symbol.lib_id
                ))
            })?;
        let selected_unit = unit.or(current_symbol.unit).unwrap_or(1);
        let selected_body_style = body_style.unwrap_or(current_symbol.body_style);
        let selected_alternates = pin_alternates.unwrap_or_else(|| {
            current_symbol
                .pins
                .iter()
                .filter_map(|pin| Some((pin.number.clone()?, pin.alternate.clone()?)))
                .collect()
        });
        let pins = self.configured_symbol_pin_refs(
            &current_symbol,
            &definition,
            selected_unit,
            selected_body_style,
            &selected_alternates,
        )?;

        let symbol = &mut self.symbols[index];
        if unit.is_some() {
            symbol.unit = Some(selected_unit);
        }
        if body_style.is_some() {
            symbol.body_style = selected_body_style;
        }
        if let Some(mirror) = normalized_mirror {
            symbol.mirror = mirror;
        }
        symbol.pins = pins;

        Ok(KicadEditSummary {
            operation: "configure-symbol".to_string(),
            target: reference.to_string(),
        })
    }

    pub fn set_symbol_property(
        &mut self,
        reference: &str,
        name: &str,
        value: &str,
        at: Option<KicadAt>,
    ) -> OslResult<KicadEditSummary> {
        if name.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad symbol property name must not be empty".to_string(),
            ));
        }
        if let Some(at) = at {
            validate_at(at, "symbol property")?;
        }

        let index = self.symbol_index_by_reference(reference)?;
        let symbol = &mut self.symbols[index];
        if let Some(property) = symbol
            .properties
            .iter_mut()
            .find(|property| property.name == name)
        {
            property.value = value.to_string();
            if let Some(at) = at {
                property.at = Some(at);
            }
        } else {
            symbol.properties.push(KicadProperty {
                name: name.to_string(),
                value: value.to_string(),
                id: None,
                at,
                hide: None,
                show_name: None,
                do_not_autoplace: None,
                effects: None,
            });
        }

        Ok(KicadEditSummary {
            operation: "set-property".to_string(),
            target: format!("{reference}.{name}"),
        })
    }

    pub fn place_symbol(&mut self, placement: KicadSymbolPlacement) -> OslResult<KicadEditSummary> {
        let KicadSymbolPlacement {
            definition,
            library_symbols,
            reference,
            value,
            at,
            unit,
            body_style,
            pin_alternates,
            uuid,
        } = placement;
        validate_at(at, "symbol placement")?;
        if unit == Some(0) {
            return Err(OslError::InvalidInput(
                "KiCad symbol placement unit must be positive".to_string(),
            ));
        }
        if body_style == Some(0) {
            return Err(OslError::InvalidInput(
                "KiCad symbol placement body style must be positive".to_string(),
            ));
        }
        if reference.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad placed symbol reference must not be empty".to_string(),
            ));
        }
        if self
            .symbols
            .iter()
            .any(|symbol| symbol.reference() == Some(reference.as_str()))
        {
            return Err(OslError::InvalidInput(format!(
                "KiCad symbol reference '{reference}' already exists"
            )));
        }

        let lib_id = definition.name.clone();
        self.merge_symbol_placement_library_symbol(&definition)?;
        for dependency in library_symbols {
            if dependency.name == lib_id {
                continue;
            }
            self.merge_symbol_placement_library_symbol(&dependency)?;
        }

        let resolved_definition = resolve_symbol_definition(&definition, &self.library_symbols)
            .unwrap_or_else(|| KicadResolvedSymbolDef::from_symbol(&definition));
        let instance_payload = format!(
            "{}:{}:{}@{},{},{}",
            lib_id, reference, value, at.x, at.y, at.rotation
        );
        let instance_uuid = self.edit_uuid(uuid, "symbol", &instance_payload)?;
        let properties = symbol_instance_properties(&definition, &reference, &value, at);
        let unit = unit.unwrap_or(1);
        let mut sorted_pins = resolved_definition
            .scoped_pins(Some(unit), body_style)
            .collect::<Vec<_>>();
        sorted_pins.sort_by(compare_pin_numbers);
        for pin_number in pin_alternates.keys() {
            let Some(pin) = sorted_pins
                .iter()
                .find(|pin| pin.number() == pin_number.as_str())
            else {
                return Err(OslError::InvalidInput(format!(
                    "KiCad symbol placement pin '{pin_number}' is not present in selected unit/body style"
                )));
            };
            let alternate = pin_alternates
                .get(pin_number)
                .expect("pin alternate was just looked up");
            if !pin
                .alternates
                .iter()
                .any(|candidate| candidate.name == *alternate)
            {
                return Err(OslError::InvalidInput(format!(
                    "KiCad symbol placement pin '{pin_number}' has no alternate '{alternate}'"
                )));
            }
        }
        let mut generated_pin_uuids = BTreeSet::new();
        let mut pins = Vec::new();
        for (index, pin) in sorted_pins.into_iter().enumerate() {
            let pin_number = pin.number().to_string();
            let pin_uuid = self.edit_uuid_excluding(
                None,
                "symbol-pin",
                &format!("{instance_uuid}:{pin_number}:{index}"),
                &generated_pin_uuids,
            )?;
            generated_pin_uuids.insert(pin_uuid.clone());
            pins.push(KicadSymbolPinRef {
                number: Some(pin_number.clone()),
                uuid: Some(pin_uuid),
                alternate: pin_alternates.get(&pin_number).cloned(),
            });
        }

        self.symbols.push(KicadSymbolInstance {
            lib_id: lib_id.clone(),
            at: Some(at),
            mirror: None,
            unit: Some(unit),
            body_style,
            uuid: Some(instance_uuid),
            exclude_from_sim: None,
            in_bom: None,
            on_board: None,
            dnp: None,
            fields_autoplaced: None,
            properties,
            pins,
            instances: Vec::new(),
        });

        Ok(KicadEditSummary {
            operation: "place-symbol".to_string(),
            target: format!("{reference} {lib_id}"),
        })
    }

    pub fn add_wire(
        &mut self,
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        if points.len() < 2 {
            return Err(OslError::InvalidInput(
                "KiCad wire edit requires at least two points".to_string(),
            ));
        }
        for point in &points {
            validate_point(*point, "wire point")?;
        }

        let payload = points_payload(&points);
        let uuid = Some(self.edit_uuid(uuid, "wire", &payload)?);
        self.wires.push(KicadWire {
            points,
            stroke: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-wire".to_string(),
            target: payload,
        })
    }

    pub fn add_bus(
        &mut self,
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        if points.len() < 2 {
            return Err(OslError::InvalidInput(
                "KiCad bus edit requires at least two points".to_string(),
            ));
        }
        for point in &points {
            validate_point(*point, "bus point")?;
        }

        let payload = points_payload(&points);
        let uuid = Some(self.edit_uuid(uuid, "bus", &payload)?);
        self.buses.push(KicadBus {
            points,
            stroke: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-bus".to_string(),
            target: payload,
        })
    }

    pub fn add_bus_entry(
        &mut self,
        at: KicadPoint,
        size: KicadSize,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(at, "bus entry")?;
        validate_bus_entry_size(size, "bus entry")?;
        if self
            .bus_entries
            .iter()
            .any(|entry| same_point(entry.at, at) && same_size(entry.size, size))
        {
            return Err(OslError::InvalidInput(format!(
                "KiCad bus entry already exists at {},{} with size {},{}",
                at.x, at.y, size.width, size.height
            )));
        }

        let payload = format!(
            "{},{}:{},{}",
            format_number(at.x),
            format_number(at.y),
            format_number(size.width),
            format_number(size.height)
        );
        let uuid = Some(self.edit_uuid(uuid, "bus-entry", &payload)?);
        self.bus_entries.push(KicadBusEntry {
            at,
            size,
            stroke: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-bus-entry".to_string(),
            target: payload,
        })
    }

    pub fn add_junction(
        &mut self,
        at: KicadPoint,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(at, "junction")?;
        if self.junctions.iter().any(|junction| {
            coordinate_key(junction.at.x) == coordinate_key(at.x)
                && coordinate_key(junction.at.y) == coordinate_key(at.y)
        }) {
            return Err(OslError::InvalidInput(format!(
                "KiCad junction already exists at {},{}",
                at.x, at.y
            )));
        }

        let payload = format!("{},{}", at.x, at.y);
        let uuid = Some(self.edit_uuid(uuid, "junction", &payload)?);
        self.junctions.push(KicadJunction {
            at,
            diameter: None,
            color: None,
            uuid,
        });

        Ok(KicadEditSummary {
            operation: "add-junction".to_string(),
            target: payload,
        })
    }

    pub fn add_no_connect(
        &mut self,
        at: KicadPoint,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_point(at, "no-connect")?;
        if self.no_connects.iter().any(|marker| {
            coordinate_key(marker.at.x) == coordinate_key(at.x)
                && coordinate_key(marker.at.y) == coordinate_key(at.y)
        }) {
            return Err(OslError::InvalidInput(format!(
                "KiCad no-connect marker already exists at {},{}",
                at.x, at.y
            )));
        }

        let payload = format!("{},{}", at.x, at.y);
        let uuid = Some(self.edit_uuid(uuid, "no-connect", &payload)?);
        self.no_connects.push(KicadNoConnect { at, uuid });

        Ok(KicadEditSummary {
            operation: "add-no-connect".to_string(),
            target: payload,
        })
    }

    pub fn add_label(
        &mut self,
        text: impl Into<String>,
        kind: KicadLabelKind,
        at: KicadAt,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "label")?;
        let text = text.into();
        if text.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad label text must not be empty".to_string(),
            ));
        }

        let payload = format!("{}@{},{},{}", text, at.x, at.y, at.rotation);
        let uuid = Some(self.edit_uuid(uuid, kind.sexpr_name(), &payload)?);
        self.labels.push(KicadLabel {
            text: text.clone(),
            kind,
            at: Some(at),
            uuid,
            shape: None,
            fields_autoplaced: None,
            effects: None,
            properties: Vec::new(),
        });

        Ok(KicadEditSummary {
            operation: "add-label".to_string(),
            target: text,
        })
    }

    pub fn add_text(
        &mut self,
        text: impl Into<String>,
        at: KicadAt,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "text")?;
        let text = text.into();
        if text.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad text item must not be empty".to_string(),
            ));
        }

        let payload = format!("{}@{},{},{}", text, at.x, at.y, at.rotation);
        let uuid = Some(self.edit_uuid(uuid, "text", &payload)?);
        self.text_items.push(KicadTextItem {
            text: text.clone(),
            at: Some(at),
            uuid,
            effects: None,
        });

        Ok(KicadEditSummary {
            operation: "add-text".to_string(),
            target: text,
        })
    }

    pub fn add_sheet(
        &mut self,
        name: &str,
        file: &str,
        at: KicadAt,
        size: KicadSize,
        pins: Vec<KicadSheetPin>,
        uuid: Option<String>,
    ) -> OslResult<KicadEditSummary> {
        validate_at(at, "sheet")?;
        validate_size(size, "sheet")?;
        let name = name.trim();
        let file = file.trim();
        if name.is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad sheet name must not be empty".to_string(),
            ));
        }
        if file.is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad sheet file must not be empty".to_string(),
            ));
        }
        if self
            .sheets
            .iter()
            .any(|sheet| sheet.sheet_name() == Some(name))
        {
            return Err(OslError::InvalidInput(format!(
                "KiCad sheet name '{name}' already exists"
            )));
        }

        let sheet_payload = format!(
            "{}:{}@{},{},{}:{}x{}",
            name, file, at.x, at.y, at.rotation, size.width, size.height
        );
        let sheet_uuid = self.edit_uuid(uuid, "sheet", &sheet_payload)?;
        let mut reserved_uuids = BTreeSet::from([sheet_uuid.clone()]);
        let mut checked_pins = Vec::new();
        for (index, pin) in pins.into_iter().enumerate() {
            let pin_name = pin.name.trim();
            if pin_name.is_empty() {
                return Err(OslError::InvalidInput(
                    "KiCad sheet pin name must not be empty".to_string(),
                ));
            }
            let pin_type = pin.pin_type.trim();
            if pin_type.is_empty() {
                return Err(OslError::InvalidInput(format!(
                    "KiCad sheet pin '{pin_name}' type must not be empty"
                )));
            }
            let at = pin.at.ok_or_else(|| {
                OslError::InvalidInput(format!("KiCad sheet pin '{pin_name}' requires a position"))
            })?;
            validate_at(at, "sheet pin")?;
            let pin_payload = format!(
                "{}:{}:{}@{},{},{}",
                sheet_uuid, pin_name, pin_type, at.x, at.y, at.rotation
            );
            let pin_uuid =
                self.edit_uuid_excluding(pin.uuid, "sheet-pin", &pin_payload, &reserved_uuids)?;
            reserved_uuids.insert(pin_uuid.clone());
            checked_pins.push(KicadSheetPin {
                name: pin_name.to_string(),
                pin_type: pin_type.to_string(),
                at: Some(at),
                uuid: Some(pin_uuid),
                effects: pin.effects.clone(),
            });
            if checked_pins[..index]
                .iter()
                .any(|existing| existing.name == pin_name)
            {
                return Err(OslError::InvalidInput(format!(
                    "KiCad sheet pin '{pin_name}' is duplicated"
                )));
            }
        }

        self.sheets.push(KicadSheet {
            at: Some(at),
            size: Some(size),
            uuid: Some(sheet_uuid),
            exclude_from_sim: None,
            in_bom: None,
            on_board: None,
            dnp: None,
            fields_autoplaced: None,
            stroke: None,
            fill: None,
            properties: sheet_properties(name, file, at, size),
            pins: checked_pins,
            instances: Vec::new(),
        });

        Ok(KicadEditSummary {
            operation: "add-sheet".to_string(),
            target: format!("{name} {file}"),
        })
    }

    fn configured_symbol_pin_refs(
        &self,
        current_symbol: &KicadSymbolInstance,
        definition: &KicadResolvedSymbolDef,
        unit: u32,
        body_style: Option<u32>,
        pin_alternates: &BTreeMap<String, String>,
    ) -> OslResult<Vec<KicadSymbolPinRef>> {
        let mut sorted_pins = definition
            .scoped_pins(Some(unit), body_style)
            .collect::<Vec<_>>();
        sorted_pins.sort_by(compare_pin_numbers);
        for pin_number in pin_alternates.keys() {
            let Some(pin) = sorted_pins
                .iter()
                .find(|pin| pin.number() == pin_number.as_str())
            else {
                return Err(OslError::InvalidInput(format!(
                    "KiCad symbol pin '{pin_number}' is not present in selected unit/body style"
                )));
            };
            let alternate = pin_alternates
                .get(pin_number)
                .expect("pin alternate was just looked up");
            if !pin
                .alternates
                .iter()
                .any(|candidate| candidate.name == *alternate)
            {
                return Err(OslError::InvalidInput(format!(
                    "KiCad symbol pin '{pin_number}' has no alternate '{alternate}'"
                )));
            }
        }

        let mut existing_by_number = current_symbol
            .pins
            .iter()
            .filter_map(|pin| Some((pin.number.clone()?, pin.uuid.clone())))
            .collect::<BTreeMap<_, _>>();
        let instance_uuid = current_symbol
            .uuid
            .as_deref()
            .unwrap_or(current_symbol.lib_id.as_str());
        let mut generated_pin_uuids = BTreeSet::new();
        let mut pins = Vec::new();
        for (index, pin) in sorted_pins.into_iter().enumerate() {
            let pin_number = pin.number().to_string();
            let pin_uuid = existing_by_number.remove(&pin_number).flatten();
            let pin_uuid = match pin_uuid {
                Some(pin_uuid) if generated_pin_uuids.insert(pin_uuid.clone()) => pin_uuid,
                _ => {
                    let pin_uuid = self.edit_uuid_excluding(
                        None,
                        "symbol-pin",
                        &format!("{instance_uuid}:{pin_number}:{index}"),
                        &generated_pin_uuids,
                    )?;
                    generated_pin_uuids.insert(pin_uuid.clone());
                    pin_uuid
                }
            };
            pins.push(KicadSymbolPinRef {
                number: Some(pin_number.clone()),
                uuid: Some(pin_uuid),
                alternate: pin_alternates.get(&pin_number).cloned(),
            });
        }

        Ok(pins)
    }

    pub fn connectivity_graph(&self) -> KicadNetGraph {
        KicadNetGraph::build(self)
    }

    pub fn canvas_scene(&self) -> KicadCanvasScene {
        KicadCanvasScene::from_schematic(self)
    }

    pub fn check_report(&self) -> KicadSchematicCheckReport {
        let graph = self.connectivity_graph();
        let mut diagnostics = Vec::new();

        self.check_duplicate_references(&mut diagnostics);
        self.check_symbols(&graph, &mut diagnostics);
        self.check_wires(&mut diagnostics);
        self.check_buses(&mut diagnostics);
        self.check_labels(&graph, &mut diagnostics);
        self.check_sheets(&mut diagnostics);
        self.check_no_connects(&mut diagnostics);
        self.check_spice_directives(&mut diagnostics);
        if !graph.nets.iter().any(|net| net.name == "0") {
            diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Error,
                "missing-ground",
                "schematic has no net labelled 0 or ground",
                None,
                None,
                None,
            ));
        }

        KicadSchematicCheckReport {
            source: self.source.clone(),
            symbol_count: self.symbols.len(),
            sheet_count: self.sheets.len(),
            net_count: graph.nets.len(),
            spice_directive_count: self.spice_directives().len(),
            diagnostics,
        }
    }

    pub fn check_report_with_hierarchy(
        &self,
        base_dir: &Path,
    ) -> OslResult<KicadSchematicCheckReport> {
        let graph = self.connectivity_graph();
        let exported = self.to_spice_netlist_with_hierarchy(base_dir)?;
        Ok(KicadSchematicCheckReport {
            source: self.source.clone(),
            symbol_count: self.symbols.len(),
            sheet_count: self.sheets.len(),
            net_count: graph.nets.len(),
            spice_directive_count: count_spice_directive_lines(&exported.netlist),
            diagnostics: exported.diagnostics,
        })
    }

    pub fn to_spice_netlist(&self) -> OslResult<String> {
        let graph = self.connectivity_graph();
        let mut lines = vec![format!("* Imported from KiCad schematic: {}", self.source)];

        lines.extend(self.spice_include_directives());

        for sheet in &self.sheets {
            if sheet.exclude_from_sim == Some(true) {
                continue;
            }
            lines.push(format!(
                "* Unsupported KiCad hierarchical sheet {} {}",
                sheet.sheet_name().unwrap_or("<unnamed-sheet>"),
                sheet.sheet_file().unwrap_or("<no-sheetfile>")
            ));
        }

        for symbol in &self.symbols {
            let definition = self.resolved_symbol_definition(&symbol.lib_id);
            match self.symbol_to_spice_line(symbol, &graph) {
                Some(line) => lines.push(line),
                None if symbol.sim_enabled(definition.as_ref()) == Some(false) => {}
                None => {
                    if let Some(line) = self.symbol_to_spice_line_legacy(symbol, &graph) {
                        lines.push(line);
                    } else {
                        lines.push(format!(
                            "* Unsupported KiCad symbol {} {}",
                            symbol.reference().unwrap_or("<no-reference>"),
                            symbol.lib_id
                        ));
                    }
                }
            }
        }

        let mut has_end = false;
        for directive in self.spice_directives() {
            let directive = directive.text.trim();
            if directive.eq_ignore_ascii_case(".end") {
                has_end = true;
            }
            lines.push(directive.to_string());
        }
        if !has_end {
            lines.push(".end".to_string());
        }
        Ok(format!("{}\n", lines.join("\n")))
    }

    pub fn to_spice_netlist_with_hierarchy(
        &self,
        base_dir: &Path,
    ) -> OslResult<KicadHierarchyNetlist> {
        let mut export = KicadHierarchyExport::new();
        let root_diagnostics = self.check_report().diagnostics;
        export.export_schematic(self, base_dir, "root", &BTreeMap::new())?;

        let has_spice_directive = !export.directives.is_empty();
        let has_analysis_directive = export
            .directives
            .iter()
            .any(|directive| is_spice_analysis_directive(directive));
        let mut lines = vec![format!("* Imported from KiCad schematic: {}", self.source)];
        lines.extend(export.includes);
        lines.extend(export.components);
        lines.extend(export.directives);
        if !lines
            .iter()
            .any(|line| line.trim().eq_ignore_ascii_case(".end"))
        {
            lines.push(".end".to_string());
        }

        let mut diagnostics = root_diagnostics
            .into_iter()
            .filter(|diagnostic| {
                !is_hierarchy_root_nonfatal_diagnostic(
                    diagnostic,
                    has_spice_directive,
                    has_analysis_directive,
                )
            })
            .collect::<Vec<_>>();
        diagnostics.extend(export.diagnostics);

        Ok(KicadHierarchyNetlist {
            netlist: format!("{}\n", lines.join("\n")),
            diagnostics,
        })
    }

    pub fn spice_directives(&self) -> Vec<&KicadTextItem> {
        self.text_items
            .iter()
            .filter(|item| item.text.trim_start().starts_with('.'))
            .collect()
    }

    pub fn to_kicad_schematic_sexpr(&self) -> String {
        let mut output = String::new();
        output.push_str("(kicad_sch\n");
        if let Some(version) = &self.version {
            output.push_str(&format!("  (version {})\n", sexpr_atom_or_string(version)));
        }
        if let Some(generator) = &self.generator {
            output.push_str(&format!("  (generator {})\n", sexpr_string(generator)));
        }
        if let Some(generator_version) = &self.generator_version {
            output.push_str(&format!(
                "  (generator_version {})\n",
                sexpr_string(generator_version)
            ));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("  (uuid {})\n", sexpr_string(uuid)));
        }
        output.push_str(&format!(
            "  (paper {})\n",
            sexpr_string(self.paper.as_deref().unwrap_or("A4"))
        ));
        if let Some(title_block) = &self.title_block {
            title_block.write_title_block_sexpr(&mut output, 2);
        }
        output.push_str("  (lib_symbols\n");
        for symbol in &self.library_symbols {
            symbol.write_symbol_sexpr(&mut output, 4);
        }
        output.push_str("  )\n");
        for alias in &self.bus_aliases {
            alias.write_bus_alias_sexpr(&mut output, 2);
        }
        for wire in &self.wires {
            wire.write_wire_sexpr(&mut output, 2);
        }
        for bus in &self.buses {
            bus.write_bus_sexpr(&mut output, 2);
        }
        for entry in &self.bus_entries {
            entry.write_bus_entry_sexpr(&mut output, 2);
        }
        for net_chain in &self.net_chains {
            net_chain.write_net_chain_sexpr(&mut output, 2);
        }
        for graphic in &self.graphics {
            graphic.write_schematic_graphic_sexpr(&mut output, 2);
        }
        for image in &self.images {
            image.write_image_sexpr(&mut output, 2);
        }
        for table in &self.tables {
            table.write_table_sexpr(&mut output, 2);
        }
        for rule_area in &self.rule_areas {
            rule_area.write_rule_area_sexpr(&mut output, 2);
        }
        for group in &self.groups {
            group.write_group_sexpr(&mut output, 2);
        }
        for junction in &self.junctions {
            junction.write_junction_sexpr(&mut output, 2);
        }
        for no_connect in &self.no_connects {
            no_connect.write_no_connect_sexpr(&mut output, 2);
        }
        for label in &self.labels {
            label.write_label_sexpr(&mut output, 2);
        }
        for label in &self.directive_labels {
            label.write_directive_label_sexpr(&mut output, 2);
        }
        for sheet in &self.sheets {
            sheet.write_sheet_sexpr(&mut output, 2);
        }
        for text in &self.text_items {
            text.write_text_sexpr(&mut output, 2);
        }
        for text_box in &self.text_boxes {
            text_box.write_text_box_sexpr(&mut output, 2);
        }
        for symbol in &self.symbols {
            symbol.write_instance_sexpr(&mut output, 2);
        }
        if !self.sheet_instances.is_empty() {
            write_sheet_instances_sexpr(&mut output, &self.sheet_instances, 2);
        }
        if !self.symbol_instances.is_empty() {
            write_symbol_path_instances_sexpr(&mut output, &self.symbol_instances, 2);
        }
        if let Some(embedded_fonts) = self.embedded_fonts {
            output.push_str(&format!(
                "  (embedded_fonts {})\n",
                if embedded_fonts { "yes" } else { "no" }
            ));
        }
        output.push_str(")\n");
        output
    }

    fn symbol_to_spice_line(
        &self,
        symbol: &KicadSymbolInstance,
        graph: &KicadNetGraph,
    ) -> Option<String> {
        let nodes = self.symbol_pin_nets(symbol, graph)?;
        self.symbol_to_spice_line_with_nodes(symbol, &nodes)
    }

    fn symbol_to_spice_line_with_nodes(
        &self,
        symbol: &KicadSymbolInstance,
        nodes: &[String],
    ) -> Option<String> {
        let definition = self.resolved_symbol_definition(&symbol.lib_id);
        if symbol.sim_enabled(definition.as_ref()) == Some(false) {
            return None;
        }

        let reference = symbol.reference()?.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }

        let has_explicit_sim_model = symbol.has_explicit_sim_model(definition.as_ref());
        let model = symbol.sim_model_value(definition.as_ref());
        let params = symbol.sim_params_value(definition.as_ref());
        let value = compose_spice_model_value(
            model.as_deref(),
            params.as_deref(),
            has_explicit_sim_model.then(|| symbol.value().unwrap_or_default().trim()),
        );
        let explicit_device = symbol.sim_device(definition.as_ref());
        let device = explicit_device
            .clone()
            .or_else(|| {
                has_explicit_sim_model.then(|| {
                    reference
                        .chars()
                        .next()
                        .map(|character| character.to_ascii_uppercase().to_string())
                        .unwrap_or_default()
                })
            })?
            .to_ascii_uppercase();
        let primitive = if explicit_device.is_some() {
            spice_primitive_for_device(&device)?
        } else {
            reference
                .chars()
                .next()
                .map(|character| character.to_ascii_uppercase().to_string())
                .unwrap_or_default()
        };
        if primitive.is_empty() {
            return None;
        }
        let spice_reference = spice_item_name(reference, &primitive);

        if primitive == "X" || device == "SUBCKT" {
            if nodes.is_empty() || value.is_empty() {
                return None;
            }
            return Some(format!("{} {} {}", spice_reference, nodes.join(" "), value));
        }
        if primitive == "SPICE" {
            if value.is_empty() {
                return None;
            }
            return Some(expand_spice_template(&value, &spice_reference, nodes));
        }

        match primitive.as_str() {
            "R" | "C" | "L" | "V" | "I" | "D" if nodes.len() >= 2 && !value.is_empty() => Some(
                format!("{spice_reference} {} {} {value}", nodes[0], nodes[1]),
            ),
            "Q" | "J" if nodes.len() >= 3 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2]
            )),
            "M" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "S" | "W" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "E" | "F" | "G" | "H" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "T" if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{spice_reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            "K" if !value.is_empty() => Some(format!("{spice_reference} {value}")),
            _ => None,
        }
    }

    pub fn spice_include_directives(&self) -> Vec<String> {
        let mut includes = BTreeSet::new();
        for symbol in &self.symbols {
            let definition = self.resolved_symbol_definition(&symbol.lib_id);
            if symbol.sim_enabled(definition.as_ref()) == Some(false) {
                continue;
            }
            if let Some(path) = symbol
                .sim_library(definition.as_ref())
                .filter(|path| !path.trim().is_empty())
            {
                includes.insert(path.trim().to_string());
            }
        }
        includes
            .into_iter()
            .map(|path| format!(".include {}", quote_spice_path(&path)))
            .collect()
    }

    fn symbol_to_spice_line_legacy(
        &self,
        symbol: &KicadSymbolInstance,
        graph: &KicadNetGraph,
    ) -> Option<String> {
        let nodes = self.symbol_pin_nets(symbol, graph)?;
        self.symbol_to_spice_line_legacy_with_nodes(symbol, &nodes)
    }

    fn symbol_to_spice_line_legacy_with_nodes(
        &self,
        symbol: &KicadSymbolInstance,
        nodes: &[String],
    ) -> Option<String> {
        let reference = symbol.reference()?.trim();
        if reference.is_empty() || reference.starts_with('#') {
            return None;
        }

        let value = symbol.value().unwrap_or_default().trim();
        let designator = reference
            .chars()
            .next()
            .map(|character| character.to_ascii_uppercase())?;

        match designator {
            'R' | 'C' | 'L' | 'V' | 'I' if nodes.len() >= 2 && !value.is_empty() => {
                Some(format!("{reference} {} {} {value}", nodes[0], nodes[1]))
            }
            'D' if nodes.len() >= 2 && !value.is_empty() => {
                Some(format!("{reference} {} {} {value}", nodes[0], nodes[1]))
            }
            'Q' | 'J' if nodes.len() >= 3 && !value.is_empty() => Some(format!(
                "{reference} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2]
            )),
            'M' if nodes.len() >= 4 && !value.is_empty() => Some(format!(
                "{reference} {} {} {} {} {value}",
                nodes[0], nodes[1], nodes[2], nodes[3]
            )),
            'X' if !nodes.is_empty() && !value.is_empty() => {
                Some(format!("{reference} {} {value}", nodes.join(" ")))
            }
            _ => None,
        }
    }

    fn symbol_pin_nets(
        &self,
        symbol: &KicadSymbolInstance,
        graph: &KicadNetGraph,
    ) -> Option<Vec<String>> {
        let symbol_at = symbol.at?;
        let definition = self.resolved_symbol_definition(&symbol.lib_id)?;
        let pins = symbol_ordered_pins(symbol, &definition);

        Some(
            pins.into_iter()
                .map(|pin| {
                    pin.at
                        .map(|pin_at| {
                            transform_symbol_point(pin_at, symbol_at, symbol.mirror.as_deref())
                        })
                        .and_then(|point| graph.net_at(point).map(str::to_string))
                        .unwrap_or_else(|| "unconnected".to_string())
                })
                .collect(),
        )
    }

    fn symbol_definition(&self, lib_id: &str) -> Option<&KicadSymbolDef> {
        self.library_symbols
            .iter()
            .find(|symbol| symbol.name == lib_id)
    }

    fn resolved_symbol_definition(&self, lib_id: &str) -> Option<KicadResolvedSymbolDef> {
        let definition = self.symbol_definition(lib_id)?;
        resolve_symbol_definition(definition, &self.library_symbols)
    }

    pub fn resolve_project_symbol_libraries(
        &mut self,
        project_dir: &Path,
    ) -> OslResult<Vec<KicadLibraryDiagnostic>> {
        let table_path = project_dir.join("sym-lib-table");
        if !table_path.exists() {
            return Ok(Vec::new());
        }
        self.resolve_missing_symbol_definitions_from_table(&table_path)
    }

    pub fn resolve_missing_symbol_definitions_from_table(
        &mut self,
        table_path: &Path,
    ) -> OslResult<Vec<KicadLibraryDiagnostic>> {
        let table = read_kicad_symbol_library_table(table_path)?;
        let base_dir = table_path.parent().unwrap_or_else(|| Path::new("."));
        let mut diagnostics = Vec::new();
        let mut missing = self.missing_symbol_lib_ids();

        for row in table.libraries {
            if missing.is_empty() {
                break;
            }
            if row.disabled {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Info,
                    message: "library row is disabled".to_string(),
                });
                continue;
            }
            if !row.library_type.eq_ignore_ascii_case("KiCad") {
                diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name.clone(),
                    severity: KicadDiagnosticSeverity::Warning,
                    message: format!("unsupported symbol library type '{}'", row.library_type),
                });
                continue;
            }

            let resolved_path = resolve_kicad_uri(&row.uri, base_dir);
            match read_kicad_symbol_library(&resolved_path) {
                Ok(library) => {
                    let mut resolved = Vec::new();
                    for lib_id in &missing {
                        if let Some(definition) =
                            library_symbol_definition_for_lib_id(&library, &row.name, lib_id)
                        {
                            self.merge_library_symbol_with_parents(definition, &library, &row.name);
                            resolved.push(lib_id.clone());
                        }
                    }
                    for lib_id in resolved {
                        missing.remove(&lib_id);
                    }
                }
                Err(error) => diagnostics.push(KicadLibraryDiagnostic {
                    library: row.name,
                    severity: KicadDiagnosticSeverity::Error,
                    message: format!("failed to load {}: {}", resolved_path.display(), error),
                }),
            }
        }

        Ok(diagnostics)
    }

    fn missing_symbol_lib_ids(&self) -> BTreeSet<String> {
        self.symbols
            .iter()
            .map(|symbol| symbol.lib_id.clone())
            .filter(|lib_id| self.symbol_definition(lib_id).is_none())
            .collect()
    }

    fn merge_library_symbol(&mut self, definition: KicadSymbolDef) -> bool {
        if self.symbol_definition(&definition.name).is_some() {
            return false;
        }
        self.library_symbols.push(definition);
        true
    }

    fn merge_symbol_placement_library_symbol(
        &mut self,
        definition: &KicadSymbolDef,
    ) -> OslResult<()> {
        match self
            .library_symbols
            .iter()
            .find(|symbol| symbol.name == definition.name)
        {
            Some(existing) if !library_symbol_definitions_are_compatible(existing, definition) => {
                Err(OslError::InvalidInput(format!(
                    "KiCad embedded library symbol '{}' already exists with different content",
                    definition.name
                )))
            }
            Some(_) => Ok(()),
            None => {
                self.library_symbols.push(definition.clone());
                Ok(())
            }
        }
    }

    fn merge_library_symbol_with_parents(
        &mut self,
        mut definition: KicadSymbolDef,
        library: &KicadSymbolLibrary,
        library_name: &str,
    ) {
        qualify_library_symbol_name(&mut definition, library_name);
        let mut pending = vec![definition];
        let mut visited = BTreeSet::new();
        while let Some(definition) = pending.pop() {
            if !visited.insert(definition.name.clone()) {
                continue;
            }
            if let Some(parent_name) = definition.extends.as_deref()
                && let Some(parent) =
                    find_symbol_inheritance_parent(&definition, parent_name, &library.symbols)
            {
                let mut parent = parent.clone();
                qualify_library_symbol_name(&mut parent, library_name);
                pending.push(parent);
            }
            self.merge_library_symbol(definition);
        }
    }

    fn check_duplicate_references(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        let mut counts = BTreeMap::<String, usize>::new();
        for symbol in &self.symbols {
            if let Some(reference) = symbol.reference()
                && !reference.trim().is_empty()
                && !reference.starts_with('#')
            {
                *counts.entry(reference.to_string()).or_default() += 1;
            }
        }
        for (reference, count) in counts {
            if count > 1 {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "duplicate-reference",
                    &format!("symbol reference '{reference}' appears {count} times"),
                    Some(reference),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_symbols(
        &self,
        graph: &KicadNetGraph,
        diagnostics: &mut Vec<KicadSchematicDiagnostic>,
    ) {
        for symbol in &self.symbols {
            let reference = symbol.reference().unwrap_or("<no-reference>").to_string();
            if symbol
                .reference()
                .is_none_or(|reference| reference.trim().is_empty())
            {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-reference",
                    "symbol has no Reference property",
                    Some(symbol.lib_id.clone()),
                    None,
                    None,
                ));
            }
            if symbol.value().is_none_or(|value| value.trim().is_empty()) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "missing-value",
                    &format!("symbol '{reference}' has no Value property"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }

            let Some(definition) = self.symbol_definition(&symbol.lib_id) else {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-symbol-definition",
                    &format!(
                        "symbol '{reference}' uses missing library symbol '{}'",
                        symbol.lib_id
                    ),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            };
            let definition = self
                .resolved_symbol_definition(&symbol.lib_id)
                .unwrap_or_else(|| KicadResolvedSymbolDef::from_symbol(definition));
            if symbol.sim_enabled(Some(&definition)) == Some(false) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Info,
                    "simulation-disabled",
                    &format!("symbol '{reference}' is excluded from simulation"),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            }
            if let Some(device) = symbol.sim_device(Some(&definition))
                && spice_primitive_for_device(&device).is_none()
            {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "unsupported-sim-device",
                    &format!("symbol '{reference}' uses unsupported Sim.Device '{device}'"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }
            let Some(symbol_at) = symbol.at else {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-symbol-position",
                    &format!("symbol '{reference}' has no placement"),
                    Some(reference),
                    None,
                    None,
                ));
                continue;
            };

            let mut definition_pins = definition
                .scoped_pins(symbol.unit, symbol.body_style)
                .collect::<Vec<_>>();
            definition_pins.sort_by(compare_pin_numbers);
            if !definition_pins.is_empty() && symbol.pins.is_empty() {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "missing-pin-refs",
                    &format!("symbol '{reference}' has no instance pin UUID references"),
                    Some(reference.clone()),
                    None,
                    None,
                ));
            }
            let sim_pin_order = symbol_sim_pin_order(symbol, &definition);
            for pin_number in &sim_pin_order {
                if !definition
                    .pins
                    .iter()
                    .filter(|pin| {
                        symbol_item_scope_matches(
                            pin.unit,
                            pin.body_style,
                            symbol.unit.unwrap_or(1),
                            symbol.body_style.unwrap_or(1),
                        )
                    })
                    .any(|pin| pin.number() == pin_number || pin.name() == pin_number)
                {
                    diagnostics.push(kicad_schematic_diagnostic(
                        KicadDiagnosticSeverity::Error,
                        "invalid-sim-pin",
                        &format!(
                            "symbol '{reference}' Sim.Pins entry '{pin_number}' does not match a library pin"
                        ),
                        Some(reference.clone()),
                        None,
                        Some(pin_number.clone()),
                    ));
                }
            }
            for pin in definition_pins {
                let pin_label = format!("{}:{}", reference, pin.number());
                let Some(pin_at) = pin.at else {
                    diagnostics.push(kicad_schematic_diagnostic(
                        KicadDiagnosticSeverity::Warning,
                        "missing-pin-position",
                        &format!(
                            "symbol '{reference}' pin '{}' has no position",
                            pin.number()
                        ),
                        Some(reference.clone()),
                        None,
                        Some(pin.number().to_string()),
                    ));
                    continue;
                };
                let point = transform_symbol_point(pin_at, symbol_at, symbol.mirror.as_deref());
                if self.has_no_connect_at(point) {
                    continue;
                }
                match graph.net_at(point) {
                    Some("unconnected") | None => diagnostics.push(kicad_schematic_diagnostic(
                        KicadDiagnosticSeverity::Warning,
                        "unconnected-pin",
                        &format!("symbol pin '{pin_label}' is not connected to a named net"),
                        Some(reference.clone()),
                        None,
                        Some(pin.number().to_string()),
                    )),
                    Some(net) if net.starts_with('n') => {
                        diagnostics.push(kicad_schematic_diagnostic(
                            KicadDiagnosticSeverity::Info,
                            "generated-net-name",
                            &format!("symbol pin '{pin_label}' is on generated net '{net}'"),
                            Some(reference.clone()),
                            Some(net.to_string()),
                            Some(pin.number().to_string()),
                        ))
                    }
                    Some(_) => {}
                }
            }
        }
    }

    fn check_wires(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        for (index, wire) in self.wires.iter().enumerate() {
            if wire.points.len() < 2 {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "invalid-wire",
                    &format!("wire #{index} has fewer than two points"),
                    Some(format!("wire:{index}")),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_labels(&self, graph: &KicadNetGraph, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        for label in &self.labels {
            if label.text.trim().is_empty() {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "empty-label",
                    "label text is empty",
                    None,
                    None,
                    None,
                ));
            }
            if let Some(at) = label.at
                && graph.net_at(at.point()).is_none()
            {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "floating-label",
                    &format!("label '{}' is not attached to any net", label.text),
                    Some(label.text.clone()),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_sheets(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        for (index, sheet) in self.sheets.iter().enumerate() {
            let item = sheet
                .sheet_name()
                .or_else(|| sheet.sheet_file())
                .map(str::to_string)
                .unwrap_or_else(|| format!("sheet:{index}"));
            if sheet.sheet_name().is_none_or(|name| name.trim().is_empty()) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-sheet-name",
                    &format!("hierarchical sheet #{index} has no Sheetname property"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.sheet_file().is_none_or(|file| file.trim().is_empty()) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-sheet-file",
                    &format!("hierarchical sheet '{item}' has no Sheetfile property"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.at.is_none() || sheet.size.is_none() {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "missing-sheet-geometry",
                    &format!("hierarchical sheet '{item}' has incomplete placement geometry"),
                    Some(item.clone()),
                    None,
                    None,
                ));
            }
            if sheet.exclude_from_sim == Some(true) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Info,
                    "simulation-disabled-sheet",
                    &format!("hierarchical sheet '{item}' is excluded from simulation"),
                    Some(item),
                    None,
                    None,
                ));
            } else {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "hierarchical-sheet-unsupported",
                    &format!(
                        "hierarchical sheet '{item}' is parsed but child sheet expansion is not implemented yet"
                    ),
                    Some(item),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_no_connects(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        let pin_points = self.symbol_pin_points();
        for marker in &self.no_connects {
            if !pin_points.iter().any(|point| same_point(*point, marker.at)) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "floating-no-connect",
                    &format!(
                        "no-connect marker at {},{} is not attached to a symbol pin",
                        marker.at.x, marker.at.y
                    ),
                    None,
                    None,
                    None,
                ));
            }
        }
    }

    fn check_buses(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        for (index, bus) in self.buses.iter().enumerate() {
            if bus.points.len() < 2 {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "empty-bus",
                    &format!("bus #{index} has fewer than two points"),
                    Some(format!("bus:{index}")),
                    None,
                    None,
                ));
            }
        }
        for (index, entry) in self.bus_entries.iter().enumerate() {
            if !is_valid_bus_entry_size(entry.size) {
                diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "invalid-bus-entry-size",
                    &format!(
                        "bus entry #{index} has invalid size {},{}",
                        entry.size.width, entry.size.height
                    ),
                    Some(format!("bus-entry:{index}")),
                    None,
                    None,
                ));
            }
        }
    }

    fn check_spice_directives(&self, diagnostics: &mut Vec<KicadSchematicDiagnostic>) {
        let directives = self.spice_directives();
        if directives.is_empty() {
            diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Warning,
                "missing-spice-directive",
                "schematic has no SPICE directives such as .tran, .ac, .dc, or .op",
                None,
                None,
                None,
            ));
            return;
        }
        if !directives
            .iter()
            .any(|directive| is_spice_analysis_directive(&directive.text))
        {
            diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Warning,
                "missing-analysis-directive",
                "schematic has SPICE text but no analysis directive (.tran, .ac, .dc, .op)",
                None,
                None,
                None,
            ));
        }
    }

    fn symbol_index_by_reference(&self, reference: &str) -> OslResult<usize> {
        if reference.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "KiCad symbol reference must not be empty".to_string(),
            ));
        }
        self.symbols
            .iter()
            .position(|symbol| symbol.reference() == Some(reference))
            .ok_or_else(|| {
                OslError::InvalidInput(format!(
                    "KiCad symbol reference '{reference}' was not found"
                ))
            })
    }

    fn edit_uuid(&self, uuid: Option<String>, namespace: &str, payload: &str) -> OslResult<String> {
        self.edit_uuid_excluding(uuid, namespace, payload, &BTreeSet::new())
    }

    fn edit_uuid_excluding(
        &self,
        uuid: Option<String>,
        namespace: &str,
        payload: &str,
        reserved: &BTreeSet<String>,
    ) -> OslResult<String> {
        let used = self.used_uuids();
        if let Some(uuid) = uuid.filter(|uuid| !uuid.trim().is_empty()) {
            if used.contains(&uuid) || reserved.contains(&uuid) {
                return Err(OslError::InvalidInput(format!(
                    "KiCad UUID '{uuid}' is already used in this schematic"
                )));
            }
            return Ok(uuid);
        }

        for counter in 0.. {
            let seed = format!(
                "{}:{namespace}:{payload}:{}:{}:{}:{counter}",
                self.source,
                self.symbols.len(),
                self.wires.len(),
                self.labels.len()
            );
            let candidate = uuid_from_hashes(fnv1a64(&seed), fnv1a64(&format!("{seed}:b")));
            if !used.contains(&candidate) && !reserved.contains(&candidate) {
                return Ok(candidate);
            }
        }
        unreachable!("unbounded UUID search should always find a free candidate")
    }

    fn used_uuids(&self) -> BTreeSet<String> {
        let mut uuids = BTreeSet::new();
        if let Some(uuid) = &self.uuid {
            uuids.insert(uuid.clone());
        }
        for wire in &self.wires {
            if let Some(uuid) = &wire.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for bus in &self.buses {
            if let Some(uuid) = &bus.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for entry in &self.bus_entries {
            if let Some(uuid) = &entry.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for graphic in &self.graphics {
            if let Some(uuid) = &graphic.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for image in &self.images {
            if let Some(uuid) = &image.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for table in &self.tables {
            if let Some(uuid) = &table.uuid {
                uuids.insert(uuid.clone());
            }
            for cell in &table.cells {
                if let Some(uuid) = &cell.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        for rule_area in &self.rule_areas {
            if let Some(uuid) = &rule_area.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for group in &self.groups {
            if let Some(uuid) = &group.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for label in &self.labels {
            if let Some(uuid) = &label.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for label in &self.directive_labels {
            if let Some(uuid) = &label.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for junction in &self.junctions {
            if let Some(uuid) = &junction.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for marker in &self.no_connects {
            if let Some(uuid) = &marker.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for sheet in &self.sheets {
            if let Some(uuid) = &sheet.uuid {
                uuids.insert(uuid.clone());
            }
            for pin in &sheet.pins {
                if let Some(uuid) = &pin.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        for text in &self.text_items {
            if let Some(uuid) = &text.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for text_box in &self.text_boxes {
            if let Some(uuid) = &text_box.uuid {
                uuids.insert(uuid.clone());
            }
        }
        for symbol in &self.symbols {
            if let Some(uuid) = &symbol.uuid {
                uuids.insert(uuid.clone());
            }
            for pin in &symbol.pins {
                if let Some(uuid) = &pin.uuid {
                    uuids.insert(uuid.clone());
                }
            }
        }
        uuids
    }

    fn symbol_pin_points(&self) -> Vec<KicadPoint> {
        self.symbols
            .iter()
            .flat_map(|symbol| {
                let Some(symbol_at) = symbol.at else {
                    return Vec::new();
                };
                self.resolved_symbol_definition(&symbol.lib_id)
                    .map(|definition| {
                        definition
                            .scoped_pins(symbol.unit, symbol.body_style)
                            .filter_map(|pin| pin.at)
                            .map(|pin_at| {
                                transform_symbol_point(pin_at, symbol_at, symbol.mirror.as_deref())
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    fn sheet_pin_points(&self) -> Vec<KicadPoint> {
        self.sheets
            .iter()
            .flat_map(|sheet| {
                sheet
                    .pins
                    .iter()
                    .filter_map(|pin| pin.at.map(|at| at.point()))
            })
            .collect()
    }

    fn has_no_connect_at(&self, point: KicadPoint) -> bool {
        self.no_connects
            .iter()
            .any(|marker| same_point(marker.at, point))
    }

    pub fn to_summary_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"generator\": {},\n",
                "  \"generator_version\": {},\n",
                "  \"has_title_block\": {},\n",
                "  \"title_comment_count\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"library_symbol_count\": {},\n",
                "  \"bus_alias_count\": {},\n",
                "  \"wire_count\": {},\n",
                "  \"styled_wire_count\": {},\n",
                "  \"bus_count\": {},\n",
                "  \"styled_bus_count\": {},\n",
                "  \"bus_entry_count\": {},\n",
                "  \"styled_bus_entry_count\": {},\n",
                "  \"net_chain_count\": {},\n",
                "  \"net_chain_member_net_count\": {},\n",
                "  \"schematic_graphic_count\": {},\n",
                "  \"styled_schematic_graphic_count\": {},\n",
                "  \"locked_schematic_graphic_count\": {},\n",
                "  \"image_count\": {},\n",
                "  \"table_count\": {},\n",
                "  \"styled_table_count\": {},\n",
                "  \"table_cell_count\": {},\n",
                "  \"styled_table_cell_count\": {},\n",
                "  \"locked_table_cell_count\": {},\n",
                "  \"rule_area_count\": {},\n",
                "  \"styled_rule_area_count\": {},\n",
                "  \"locked_rule_area_count\": {},\n",
                "  \"group_count\": {},\n",
                "  \"group_member_count\": {},\n",
                "  \"label_count\": {},\n",
                "  \"directive_label_count\": {},\n",
                "  \"directive_label_property_count\": {},\n",
                "  \"junction_count\": {},\n",
                "  \"styled_junction_count\": {},\n",
                "  \"no_connect_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"styled_sheet_count\": {},\n",
                "  \"sheet_pin_count\": {},\n",
                "  \"text_count\": {},\n",
                "  \"text_box_count\": {},\n",
                "  \"styled_text_box_count\": {},\n",
                "  \"locked_text_box_count\": {},\n",
                "  \"spice_directive_count\": {},\n",
                "  \"sheet_instance_count\": {},\n",
                "  \"symbol_instance_count\": {},\n",
                "  \"symbol_pin_alternate_count\": {},\n",
                "  \"embedded_project_instance_count\": {},\n",
                "  \"embedded_instance_path_count\": {},\n",
                "  \"variant_instance_count\": {},\n",
                "  \"dnp_item_count\": {},\n",
                "  \"bom_excluded_count\": {},\n",
                "  \"board_excluded_count\": {},\n",
                "  \"mirrored_symbol_count\": {},\n",
                "  \"symbol_body_style_count\": {},\n",
                "  \"fields_autoplaced_count\": {},\n",
                "  \"shaped_label_count\": {},\n",
                "  \"label_property_count\": {},\n",
                "  \"hidden_property_count\": {},\n",
                "  \"property_effect_count\": {},\n",
                "  \"embedded_fonts\": {},\n",
                "  \"library_unit_name_count\": {},\n",
                "  \"library_graphic_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            json_option(self.generator.as_deref()),
            json_option(self.generator_version.as_deref()),
            self.title_block.is_some(),
            self.title_block
                .as_ref()
                .map(|title_block| title_block.comments.len())
                .unwrap_or(0),
            self.symbols.len(),
            self.library_symbols.len(),
            self.bus_aliases.len(),
            self.wires.len(),
            self.styled_wire_count(),
            self.buses.len(),
            self.styled_bus_count(),
            self.bus_entries.len(),
            self.styled_bus_entry_count(),
            self.net_chains.len(),
            self.net_chain_member_net_count(),
            self.graphics.len(),
            self.styled_schematic_graphic_count(),
            self.locked_schematic_graphic_count(),
            self.images.len(),
            self.tables.len(),
            self.styled_table_count(),
            self.tables
                .iter()
                .map(|table| table.cells.len())
                .sum::<usize>(),
            self.styled_table_cell_count(),
            self.locked_table_cell_count(),
            self.rule_areas.len(),
            self.styled_rule_area_count(),
            self.locked_rule_area_count(),
            self.groups.len(),
            self.groups
                .iter()
                .map(|group| group.members.len())
                .sum::<usize>(),
            self.labels.len(),
            self.directive_labels.len(),
            self.directive_label_property_count(),
            self.junctions.len(),
            self.styled_junction_count(),
            self.no_connects.len(),
            self.sheets.len(),
            self.styled_sheet_count(),
            self.sheets
                .iter()
                .map(|sheet| sheet.pins.len())
                .sum::<usize>(),
            self.text_items.len(),
            self.text_boxes.len(),
            self.styled_text_box_count(),
            self.locked_text_box_count(),
            self.spice_directives().len(),
            self.sheet_instances.len(),
            self.symbol_instances.len(),
            self.symbol_pin_alternate_count(),
            self.embedded_project_instance_count(),
            self.embedded_instance_path_count(),
            self.variant_instance_count(),
            self.dnp_item_count(),
            self.bom_excluded_count(),
            self.board_excluded_count(),
            self.mirrored_symbol_count(),
            self.symbol_body_style_count(),
            self.fields_autoplaced_count(),
            self.shaped_label_count(),
            self.label_property_count(),
            self.hidden_property_count(),
            self.property_effect_count(),
            json_bool_option(self.embedded_fonts),
            self.library_unit_name_count(),
            self.library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>()
        )
    }

    fn library_unit_name_count(&self) -> usize {
        self.library_symbols
            .iter()
            .map(|symbol| symbol.unit_names.len())
            .sum()
    }

    fn embedded_project_instance_count(&self) -> usize {
        self.symbols
            .iter()
            .map(|symbol| symbol.instances.len())
            .sum::<usize>()
            + self
                .sheets
                .iter()
                .map(|sheet| sheet.instances.len())
                .sum::<usize>()
    }

    fn symbol_pin_alternate_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.pins)
            .filter(|pin| pin.alternate.is_some())
            .count()
    }

    fn embedded_instance_path_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.instances)
            .map(|instance| instance.paths.len())
            .sum::<usize>()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.instances)
                .map(|instance| instance.paths.len())
                .sum::<usize>()
    }

    fn variant_instance_count(&self) -> usize {
        let embedded_variants = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.instances)
            .flat_map(|instance| &instance.paths)
            .map(|path| path.variants.len())
            .sum::<usize>()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.instances)
                .flat_map(|instance| &instance.paths)
                .map(|path| path.variants.len())
                .sum::<usize>();
        let top_level_variants = self
            .symbol_instances
            .iter()
            .map(|instance| instance.variants.len())
            .sum::<usize>();
        embedded_variants + top_level_variants
    }

    fn styled_schematic_graphic_count(&self) -> usize {
        self.graphics
            .iter()
            .filter(|graphic| graphic.stroke.is_some() || graphic.fill.is_some())
            .count()
    }

    fn styled_wire_count(&self) -> usize {
        self.wires
            .iter()
            .filter(|wire| wire.stroke.is_some())
            .count()
    }

    fn styled_bus_count(&self) -> usize {
        self.buses.iter().filter(|bus| bus.stroke.is_some()).count()
    }

    fn styled_bus_entry_count(&self) -> usize {
        self.bus_entries
            .iter()
            .filter(|entry| entry.stroke.is_some())
            .count()
    }

    fn net_chain_member_net_count(&self) -> usize {
        self.net_chains
            .iter()
            .map(|net_chain| net_chain.member_nets.len())
            .sum()
    }

    fn locked_schematic_graphic_count(&self) -> usize {
        self.graphics
            .iter()
            .filter(|graphic| graphic.locked == Some(true))
            .count()
    }

    fn styled_text_box_count(&self) -> usize {
        self.text_boxes
            .iter()
            .filter(|text_box| text_box.stroke.is_some() || text_box.fill.is_some())
            .count()
    }

    fn locked_text_box_count(&self) -> usize {
        self.text_boxes
            .iter()
            .filter(|text_box| text_box.locked == Some(true))
            .count()
    }

    fn styled_table_count(&self) -> usize {
        self.tables
            .iter()
            .filter(|table| table.border.is_some() || table.separators.is_some())
            .count()
    }

    fn styled_table_cell_count(&self) -> usize {
        self.tables
            .iter()
            .flat_map(|table| &table.cells)
            .filter(|cell| cell.fill.is_some() || cell.effects.is_some())
            .count()
    }

    fn locked_table_cell_count(&self) -> usize {
        self.tables
            .iter()
            .flat_map(|table| &table.cells)
            .filter(|cell| cell.locked == Some(true))
            .count()
    }

    fn styled_rule_area_count(&self) -> usize {
        self.rule_areas
            .iter()
            .filter(|rule_area| rule_area.stroke.is_some() || rule_area.fill.is_some())
            .count()
    }

    fn locked_rule_area_count(&self) -> usize {
        self.rule_areas
            .iter()
            .filter(|rule_area| rule_area.locked == Some(true))
            .count()
    }

    fn styled_junction_count(&self) -> usize {
        self.junctions
            .iter()
            .filter(|junction| junction.diameter.is_some() || junction.color.is_some())
            .count()
    }

    fn styled_sheet_count(&self) -> usize {
        self.sheets
            .iter()
            .filter(|sheet| sheet.stroke.is_some() || sheet.fill.is_some())
            .count()
    }

    fn dnp_item_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.dnp == Some(true))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.dnp == Some(true))
                .count()
            + self
                .rule_areas
                .iter()
                .filter(|rule_area| rule_area.dnp == Some(true))
                .count()
    }

    fn bom_excluded_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.in_bom == Some(false))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.in_bom == Some(false))
                .count()
            + self
                .rule_areas
                .iter()
                .filter(|rule_area| rule_area.in_bom == Some(false))
                .count()
    }

    fn board_excluded_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.on_board == Some(false))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.on_board == Some(false))
                .count()
            + self
                .rule_areas
                .iter()
                .filter(|rule_area| rule_area.on_board == Some(false))
                .count()
    }

    fn mirrored_symbol_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.mirror.is_some())
            .count()
    }

    fn symbol_body_style_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.body_style.is_some())
            .count()
    }

    fn fields_autoplaced_count(&self) -> usize {
        self.symbols
            .iter()
            .filter(|symbol| symbol.fields_autoplaced == Some(true))
            .count()
            + self
                .sheets
                .iter()
                .filter(|sheet| sheet.fields_autoplaced == Some(true))
                .count()
            + self
                .labels
                .iter()
                .filter(|label| label.fields_autoplaced == Some(true))
                .count()
            + self
                .directive_labels
                .iter()
                .filter(|label| label.fields_autoplaced == Some(true))
                .count()
    }

    fn shaped_label_count(&self) -> usize {
        self.labels
            .iter()
            .filter(|label| label.shape.is_some())
            .count()
    }

    fn label_property_count(&self) -> usize {
        self.labels.iter().map(|label| label.properties.len()).sum()
    }

    fn directive_label_property_count(&self) -> usize {
        self.directive_labels
            .iter()
            .map(|label| label.properties.len())
            .sum()
    }

    fn hidden_property_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.properties)
            .filter(|property| property_is_hidden(property))
            .count()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.properties)
                .filter(|property| property_is_hidden(property))
                .count()
            + self
                .labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property_is_hidden(property))
                .count()
            + self
                .directive_labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property_is_hidden(property))
                .count()
    }

    fn property_effect_count(&self) -> usize {
        self.symbols
            .iter()
            .flat_map(|symbol| &symbol.properties)
            .filter(|property| property.effects.is_some())
            .count()
            + self
                .sheets
                .iter()
                .flat_map(|sheet| &sheet.properties)
                .filter(|property| property.effects.is_some())
                .count()
            + self
                .labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property.effects.is_some())
                .count()
            + self
                .directive_labels
                .iter()
                .flat_map(|label| &label.properties)
                .filter(|property| property.effects.is_some())
                .count()
    }
}

struct KicadHierarchyExport {
    includes: BTreeSet<String>,
    components: Vec<String>,
    directives: Vec<String>,
    diagnostics: Vec<KicadSchematicDiagnostic>,
    visited: BTreeSet<PathBuf>,
}

impl KicadHierarchyExport {
    fn new() -> Self {
        Self {
            includes: BTreeSet::new(),
            components: Vec::new(),
            directives: Vec::new(),
            diagnostics: Vec::new(),
            visited: BTreeSet::new(),
        }
    }

    fn export_schematic(
        &mut self,
        schematic: &KicadSchematic,
        base_dir: &Path,
        scope: &str,
        net_aliases: &BTreeMap<String, String>,
    ) -> OslResult<()> {
        let graph = schematic.connectivity_graph();
        self.includes.extend(schematic.spice_include_directives());
        for symbol in &schematic.symbols {
            let Some(nodes) = schematic.symbol_pin_nets(symbol, &graph) else {
                continue;
            };
            let mapped_nodes = nodes
                .iter()
                .map(|node| scoped_net_name(scope, node, net_aliases))
                .collect::<Vec<_>>();
            let scoped_symbol = scoped_symbol_instance(symbol, scope);
            let definition = schematic.resolved_symbol_definition(&symbol.lib_id);
            match schematic.symbol_to_spice_line_with_nodes(&scoped_symbol, &mapped_nodes) {
                Some(line) => self.components.push(line),
                None if scoped_symbol.sim_enabled(definition.as_ref()) == Some(false) => {}
                None => {
                    if let Some(line) = schematic
                        .symbol_to_spice_line_legacy_with_nodes(&scoped_symbol, &mapped_nodes)
                    {
                        self.components.push(line);
                    } else {
                        self.components.push(format!(
                            "* Unsupported KiCad symbol {} {}",
                            scoped_symbol.reference().unwrap_or("<no-reference>"),
                            scoped_symbol.lib_id
                        ));
                    }
                }
            }
        }

        for sheet in &schematic.sheets {
            if sheet.exclude_from_sim == Some(true) {
                continue;
            }
            let Some(sheet_file) = sheet.sheet_file().filter(|file| !file.trim().is_empty()) else {
                continue;
            };
            let sheet_path = base_dir.join(sheet_file);
            let visit_key = fs::canonicalize(&sheet_path).unwrap_or_else(|_| sheet_path.clone());
            if !self.visited.insert(visit_key.clone()) {
                self.diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "hierarchical-sheet-cycle",
                    &format!(
                        "hierarchical sheet '{}' was already visited",
                        sheet_path.display()
                    ),
                    sheet.sheet_name().map(str::to_string),
                    None,
                    None,
                ));
                continue;
            }

            match read_kicad_schematic_with_libraries(&sheet_path) {
                Ok(child) => {
                    self.diagnostics.extend(
                        child
                            .check_report()
                            .diagnostics
                            .into_iter()
                            .filter(|diagnostic| !is_child_sheet_nonfatal_diagnostic(diagnostic)),
                    );
                    let aliases =
                        self.sheet_net_aliases(schematic, sheet, &graph, scope, net_aliases);
                    let child_scope = child_sheet_scope(scope, sheet);
                    let child_base_dir = sheet_path.parent().unwrap_or(base_dir);
                    self.export_schematic(&child, child_base_dir, &child_scope, &aliases)?;
                }
                Err(error) => self.diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Error,
                    "missing-child-sheet",
                    &format!(
                        "failed to load hierarchical sheet {}: {}",
                        sheet_path.display(),
                        error
                    ),
                    sheet.sheet_name().map(str::to_string),
                    None,
                    None,
                )),
            }
            self.visited.remove(&visit_key);
        }

        for directive in schematic.spice_directives() {
            let directive = directive.text.trim();
            if directive.eq_ignore_ascii_case(".end") {
                continue;
            }
            if !self
                .directives
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(directive))
            {
                self.directives.push(directive.to_string());
            }
        }

        Ok(())
    }

    fn sheet_net_aliases(
        &mut self,
        schematic: &KicadSchematic,
        sheet: &KicadSheet,
        graph: &KicadNetGraph,
        scope: &str,
        parent_aliases: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        let mut aliases = BTreeMap::new();
        for pin in &sheet.pins {
            let Some(at) = pin.at else {
                continue;
            };
            match graph.net_at(at.point()) {
                Some(net) => {
                    aliases.insert(
                        normalize_net_name(&pin.name),
                        scoped_net_name(scope, net, parent_aliases),
                    );
                }
                None => self.diagnostics.push(kicad_schematic_diagnostic(
                    KicadDiagnosticSeverity::Warning,
                    "unconnected-sheet-pin",
                    &format!(
                        "hierarchical sheet '{}' pin '{}' is not connected to a parent net",
                        sheet.sheet_name().unwrap_or("<unnamed-sheet>"),
                        pin.name
                    ),
                    sheet.sheet_name().map(str::to_string),
                    None,
                    Some(pin.name.clone()),
                )),
            }
        }
        if aliases.is_empty() && !sheet.pins.is_empty() {
            self.diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Warning,
                "unmapped-sheet-pins",
                &format!(
                    "hierarchical sheet '{}' has pins but no parent net aliases were mapped",
                    sheet.sheet_name().unwrap_or("<unnamed-sheet>")
                ),
                sheet.sheet_name().map(str::to_string),
                None,
                None,
            ));
        }
        if sheet.pins.is_empty() && !schematic.sheets.is_empty() {
            self.diagnostics.push(kicad_schematic_diagnostic(
                KicadDiagnosticSeverity::Info,
                "sheet-without-pins",
                &format!(
                    "hierarchical sheet '{}' has no sheet pins",
                    sheet.sheet_name().unwrap_or("<unnamed-sheet>")
                ),
                sheet.sheet_name().map(str::to_string),
                None,
                None,
            ));
        }
        aliases
    }
}

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

pub(crate) fn default_kicad_text_effects() -> KicadTextEffects {
    KicadTextEffects {
        font_size: Some(KicadSize {
            width: 1.27,
            height: 1.27,
        }),
        font_thickness: None,
        font_bold: None,
        font_italic: None,
        font_color: None,
        justify: Vec::new(),
        hide: false,
        href: None,
    }
}

fn is_child_sheet_nonfatal_diagnostic(diagnostic: &KicadSchematicDiagnostic) -> bool {
    matches!(
        diagnostic.code.as_str(),
        "hierarchical-sheet-unsupported"
            | "simulation-disabled-sheet"
            | "missing-spice-directive"
            | "missing-analysis-directive"
            | "missing-ground"
    )
}

fn is_hierarchy_root_nonfatal_diagnostic(
    diagnostic: &KicadSchematicDiagnostic,
    has_spice_directive: bool,
    has_analysis_directive: bool,
) -> bool {
    matches!(
        diagnostic.code.as_str(),
        "hierarchical-sheet-unsupported" | "simulation-disabled-sheet"
    ) || (diagnostic.code == "missing-spice-directive" && has_spice_directive)
        || (diagnostic.code == "missing-analysis-directive" && has_analysis_directive)
}

fn is_spice_analysis_directive(text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    text.starts_with(".tran")
        || text.starts_with(".ac")
        || text.starts_with(".dc")
        || text.starts_with(".op")
}

fn count_spice_directive_lines(netlist: &str) -> usize {
    netlist
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with('.') && !line.eq_ignore_ascii_case(".end")
        })
        .count()
}

fn property_is_hidden(property: &KicadProperty) -> bool {
    property.hide == Some(true)
        || property
            .effects
            .as_ref()
            .is_some_and(|effects| effects.hide)
}

fn scoped_net_name(scope: &str, net: &str, aliases: &BTreeMap<String, String>) -> String {
    if net == "0" || net.eq_ignore_ascii_case("gnd") {
        return "0".to_string();
    }
    if let Some(alias) = aliases.get(&normalize_net_name(net)) {
        return alias.clone();
    }
    if scope == "root" || net == "unconnected" {
        return net.to_string();
    }
    format!(
        "{}_{}",
        sanitize_spice_identifier(scope),
        sanitize_spice_identifier(net)
    )
}

fn child_sheet_scope(parent_scope: &str, sheet: &KicadSheet) -> String {
    let sheet_name = sheet
        .sheet_name()
        .or_else(|| sheet.sheet_file())
        .unwrap_or("sheet");
    let sheet_name = sanitize_spice_identifier(sheet_name);
    if parent_scope == "root" {
        sheet_name
    } else {
        format!("{}_{}", sanitize_spice_identifier(parent_scope), sheet_name)
    }
}

fn scoped_symbol_instance(symbol: &KicadSymbolInstance, scope: &str) -> KicadSymbolInstance {
    if scope == "root" {
        return symbol.clone();
    }

    let mut symbol = symbol.clone();
    if let Some(reference) = symbol.reference().map(str::to_string)
        && !reference.trim().is_empty()
        && !reference.starts_with('#')
    {
        let scoped_reference = scoped_reference(&reference, scope);
        if let Some(property) = symbol
            .properties
            .iter_mut()
            .find(|property| property.name == "Reference")
        {
            property.value = scoped_reference;
        }
    }
    symbol
}

fn scoped_reference(reference: &str, scope: &str) -> String {
    let mut chars = reference.chars();
    let Some(prefix) = chars.next() else {
        return reference.to_string();
    };
    let suffix = chars.collect::<String>();
    format!(
        "{}{}_{}",
        prefix,
        sanitize_spice_identifier(scope),
        sanitize_spice_identifier(&suffix)
    )
}

fn sanitize_spice_identifier(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "item".to_string()
    } else {
        sanitized
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNetGraph {
    pub nets: Vec<KicadNet>,
    nets_by_point: BTreeMap<PointKey, String>,
}

impl KicadNetGraph {
    fn build(schematic: &KicadSchematic) -> Self {
        let mut points = BTreeMap::<PointKey, KicadPoint>::new();
        for wire in &schematic.wires {
            for point in &wire.points {
                insert_point(&mut points, *point);
            }
        }
        for label in &schematic.labels {
            if let Some(at) = label.at {
                insert_point(&mut points, at.point());
            }
        }
        for junction in &schematic.junctions {
            insert_point(&mut points, junction.at);
        }
        for point in schematic.symbol_pin_points() {
            insert_point(&mut points, point);
        }
        for point in schematic.sheet_pin_points() {
            insert_point(&mut points, point);
        }

        let ordered_keys = points.keys().copied().collect::<Vec<_>>();
        let indexes = ordered_keys
            .iter()
            .enumerate()
            .map(|(index, key)| (*key, index))
            .collect::<BTreeMap<_, _>>();
        let mut graph = DisjointSet::new(ordered_keys.len());

        for wire in &schematic.wires {
            for segment in wire.points.windows(2) {
                let mut segment_indexes = ordered_keys
                    .iter()
                    .filter(|key| {
                        points.get(key).is_some_and(|point| {
                            segment_contains_point(segment[0], segment[1], *point)
                        })
                    })
                    .filter_map(|key| indexes.get(key).copied())
                    .collect::<Vec<_>>();
                segment_indexes.sort_unstable();
                if let Some(first) = segment_indexes.first().copied() {
                    for index in segment_indexes.into_iter().skip(1) {
                        graph.union(first, index);
                    }
                }
            }
        }

        let mut labels_by_name = BTreeMap::<String, Vec<usize>>::new();
        for label in &schematic.labels {
            if let Some(at) = label.at
                && let Some(index) = indexes.get(&PointKey::from(at.point())).copied()
            {
                labels_by_name
                    .entry(normalize_net_name(&label.text))
                    .or_default()
                    .push(index);
            }
        }
        for label_indexes in labels_by_name.values() {
            if let Some(first) = label_indexes.first().copied() {
                for index in label_indexes.iter().copied().skip(1) {
                    graph.union(first, index);
                }
            }
        }

        let mut labels_by_root = BTreeMap::<usize, BTreeSet<String>>::new();
        for label in &schematic.labels {
            if let Some(at) = label.at
                && let Some(index) = indexes.get(&PointKey::from(at.point())).copied()
            {
                labels_by_root
                    .entry(graph.find(index))
                    .or_default()
                    .insert(normalize_net_name(&label.text));
            }
        }

        let mut names_by_root = BTreeMap::<usize, String>::new();
        let mut generated_index = 1;
        for index in 0..ordered_keys.len() {
            let root = graph.find(index);
            names_by_root.entry(root).or_insert_with(|| {
                preferred_net_label(labels_by_root.get(&root)).unwrap_or_else(|| {
                    let name = format!("n{generated_index:03}");
                    generated_index += 1;
                    name
                })
            });
        }

        let mut nets_by_point = BTreeMap::new();
        let mut points_by_net = BTreeMap::<String, Vec<KicadPoint>>::new();
        for (index, key) in ordered_keys.iter().enumerate() {
            let root = graph.find(index);
            let name = names_by_root
                .get(&root)
                .cloned()
                .unwrap_or_else(|| "n000".to_string());
            nets_by_point.insert(*key, name.clone());
            if let Some(point) = points.get(key).copied() {
                points_by_net.entry(name).or_default().push(point);
            }
        }

        let nets = points_by_net
            .into_iter()
            .map(|(name, points)| KicadNet { name, points })
            .collect();

        Self {
            nets,
            nets_by_point,
        }
    }

    pub fn net_at(&self, point: KicadPoint) -> Option<&str> {
        self.nets_by_point
            .get(&PointKey::from(point))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNet {
    pub name: String,
    pub points: Vec<KicadPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibrary {
    pub source: String,
    pub version: Option<String>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub symbols: Vec<KicadSymbolDef>,
}

impl KicadSymbolLibrary {
    pub fn symbol(&self, name: &str) -> Option<&KicadSymbolDef> {
        self.symbols.iter().find(|symbol| symbol.name == name)
    }

    pub fn symbol_by_name_or_local_name(&self, name: &str) -> Option<&KicadSymbolDef> {
        self.symbols
            .iter()
            .find(|symbol| symbol.name == name || symbol.local_name() == name)
    }

    pub fn to_kicad_symbol_library_sexpr(&self) -> String {
        let mut output = String::new();
        output.push_str("(kicad_symbol_lib\n");
        if let Some(version) = &self.version {
            output.push_str(&format!("  (version {})\n", sexpr_atom_or_string(version)));
        }
        if let Some(generator) = &self.generator {
            output.push_str(&format!("  (generator {})\n", sexpr_string(generator)));
        }
        if let Some(generator_version) = &self.generator_version {
            output.push_str(&format!(
                "  (generator_version {})\n",
                sexpr_string(generator_version)
            ));
        }
        for symbol in &self.symbols {
            symbol.write_symbol_sexpr(&mut output, 2);
        }
        output.push_str(")\n");
        output
    }

    pub fn to_summary_json(&self) -> String {
        let pin_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.pins.len())
            .sum::<usize>();
        let pin_display_setting_count = self
            .symbols
            .iter()
            .map(|symbol| {
                usize::from(symbol.pin_names.is_some()) + usize::from(symbol.pin_numbers.is_some())
            })
            .sum::<usize>();
        let unit_name_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.unit_names.len())
            .sum::<usize>();
        let pin_text_effect_count = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.pins)
            .map(|pin| {
                usize::from(pin.name_effects().is_some())
                    + usize::from(pin.number_effects().is_some())
            })
            .sum::<usize>();
        let pin_alternate_count = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.pins)
            .map(|pin| pin.alternates.len())
            .sum::<usize>();
        let power_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.power.is_some())
            .count();
        let symbol_in_bom_setting_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.in_bom.is_some())
            .count();
        let symbol_on_board_setting_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.on_board.is_some())
            .count();
        let symbol_in_pos_files_setting_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.in_pos_files.is_some())
            .count();
        let duplicate_pin_numbers_are_jumpers_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.duplicate_pin_numbers_are_jumpers == Some(true))
            .count();
        let extended_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.extends.is_some())
            .count();
        let described_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.description().is_some())
            .count();
        let keyword_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.keywords().is_some())
            .count();
        let footprint_filter_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.footprint_filters().len())
            .sum::<usize>();
        let body_style_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.body_styles.is_some())
            .count();
        let jumper_pin_group_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.jumper_pin_groups.len())
            .sum::<usize>();
        let embedded_font_symbol_count = self
            .symbols
            .iter()
            .filter(|symbol| symbol.embedded_fonts.is_some())
            .count();
        let symbol_graphic_text_effect_count = self
            .symbols
            .iter()
            .flat_map(|symbol| &symbol.graphics)
            .filter(|graphic| {
                matches!(
                    &graphic.graphic,
                    KicadGraphic::Text {
                        effects: Some(_),
                        ..
                    }
                )
            })
            .count();
        let unit_scoped_item_count = self
            .symbols
            .iter()
            .map(|symbol| {
                symbol
                    .graphics
                    .iter()
                    .filter(|graphic| graphic.unit != 0)
                    .count()
                    + symbol.pins.iter().filter(|pin| pin.unit != 0).count()
            })
            .sum::<usize>();
        let body_style_scoped_item_count = self
            .symbols
            .iter()
            .map(|symbol| {
                symbol
                    .graphics
                    .iter()
                    .filter(|graphic| graphic.body_style != 0)
                    .count()
                    + symbol.pins.iter().filter(|pin| pin.body_style != 0).count()
            })
            .sum::<usize>();

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"generator\": {},\n",
                "  \"generator_version\": {},\n",
                "  \"symbol_count\": {},\n",
                "  \"graphic_count\": {},\n",
                "  \"symbol_graphic_text_effect_count\": {},\n",
                "  \"unit_scoped_item_count\": {},\n",
                "  \"body_style_scoped_item_count\": {},\n",
                "  \"pin_count\": {},\n",
                "  \"pin_display_setting_count\": {},\n",
                "  \"unit_name_count\": {},\n",
                "  \"pin_text_effect_count\": {},\n",
                "  \"pin_alternate_count\": {},\n",
                "  \"power_symbol_count\": {},\n",
                "  \"symbol_in_bom_setting_count\": {},\n",
                "  \"symbol_on_board_setting_count\": {},\n",
                "  \"symbol_in_pos_files_setting_count\": {},\n",
                "  \"duplicate_pin_numbers_are_jumpers_count\": {},\n",
                "  \"extended_symbol_count\": {},\n",
                "  \"described_symbol_count\": {},\n",
                "  \"keyword_symbol_count\": {},\n",
                "  \"footprint_filter_count\": {},\n",
                "  \"body_style_symbol_count\": {},\n",
                "  \"jumper_pin_group_count\": {},\n",
                "  \"embedded_font_symbol_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            json_option(self.generator.as_deref()),
            json_option(self.generator_version.as_deref()),
            self.symbols.len(),
            self.symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            symbol_graphic_text_effect_count,
            unit_scoped_item_count,
            body_style_scoped_item_count,
            pin_count,
            pin_display_setting_count,
            unit_name_count,
            pin_text_effect_count,
            pin_alternate_count,
            power_symbol_count,
            symbol_in_bom_setting_count,
            symbol_on_board_setting_count,
            symbol_in_pos_files_setting_count,
            duplicate_pin_numbers_are_jumpers_count,
            extended_symbol_count,
            described_symbol_count,
            keyword_symbol_count,
            footprint_filter_count,
            body_style_symbol_count,
            jumper_pin_group_count,
            embedded_font_symbol_count
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibraryTable {
    pub source: String,
    pub version: Option<String>,
    pub libraries: Vec<KicadSymbolLibraryTableRow>,
}

impl KicadSymbolLibraryTable {
    pub fn enabled_kicad_libraries(&self) -> impl Iterator<Item = &KicadSymbolLibraryTableRow> {
        self.libraries
            .iter()
            .filter(|row| !row.disabled && row.library_type.eq_ignore_ascii_case("KiCad"))
    }

    pub fn to_summary_json(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"version\": {},\n",
                "  \"library_count\": {},\n",
                "  \"enabled_kicad_library_count\": {},\n",
                "  \"disabled_library_count\": {},\n",
                "  \"hidden_library_count\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            json_option(self.version.as_deref()),
            self.libraries.len(),
            self.enabled_kicad_libraries().count(),
            self.libraries.iter().filter(|row| row.disabled).count(),
            self.libraries.iter().filter(|row| row.hidden).count(),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolLibraryTableRow {
    pub name: String,
    pub library_type: String,
    pub uri: String,
    pub options: Option<String>,
    pub description: Option<String>,
    pub hidden: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl KicadDiagnosticSeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolInstance {
    pub lib_id: String,
    pub at: Option<KicadAt>,
    pub mirror: Option<String>,
    pub unit: Option<u32>,
    pub body_style: Option<u32>,
    pub uuid: Option<String>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
    pub fields_autoplaced: Option<bool>,
    pub properties: Vec<KicadProperty>,
    pub pins: Vec<KicadSymbolPinRef>,
    pub instances: Vec<KicadProjectInstance>,
}

impl KicadSymbolInstance {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn reference(&self) -> Option<&str> {
        self.property("Reference")
    }

    pub fn value(&self) -> Option<&str> {
        self.property("Value")
    }

    fn inherited_property<'a>(
        &'a self,
        definition: Option<&'a impl KicadSymbolPropertySource>,
        name: &str,
    ) -> Option<&'a str> {
        self.property(name)
            .or_else(|| definition.and_then(|definition| definition.property_value(name)))
    }

    fn sim_enabled(&self, definition: Option<&impl KicadSymbolPropertySource>) -> Option<bool> {
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            return Some(!exclude_from_sim);
        }
        if let Some(exclude_from_sim) =
            definition.and_then(|definition| definition.exclude_from_sim_value())
        {
            return Some(!exclude_from_sim);
        }
        self.inherited_property(definition, "Sim.Enable")
            .or_else(|| self.inherited_property(definition, "Spice_Netlist_Enabled"))
            .and_then(parse_kicad_enable_value)
    }

    fn sim_device(&self, definition: Option<&impl KicadSymbolPropertySource>) -> Option<String> {
        self.inherited_property(definition, "Sim.Device")
            .or_else(|| self.inherited_property(definition, "Spice_Primitive"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn sim_model_value(
        &self,
        definition: Option<&impl KicadSymbolPropertySource>,
    ) -> Option<String> {
        if let Some(value) = self
            .inherited_property(definition, "Sim.Name")
            .or_else(|| self.inherited_property(definition, "Spice_Model"))
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_string());
        }
        self.inherited_property(definition, "Sim.Params")
            .and_then(|value| extract_named_sim_param(value, "model"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn sim_params_value(
        &self,
        definition: Option<&impl KicadSymbolPropertySource>,
    ) -> Option<String> {
        self.inherited_property(definition, "Sim.Params")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(strip_kicad_sim_model_params)
            .filter(|value| !value.is_empty())
    }

    fn sim_library<'a>(
        &'a self,
        definition: Option<&'a impl KicadSymbolPropertySource>,
    ) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Library")
            .or_else(|| self.inherited_property(definition, "Spice_Lib_File"))
    }

    fn sim_pins<'a>(
        &'a self,
        definition: Option<&'a impl KicadSymbolPropertySource>,
    ) -> Option<&'a str> {
        self.inherited_property(definition, "Sim.Pins")
            .or_else(|| self.inherited_property(definition, "Spice_Node_Sequence"))
    }

    fn has_explicit_sim_model(&self, definition: Option<&impl KicadSymbolPropertySource>) -> bool {
        self.inherited_property(definition, "Sim.Device").is_some()
            || self.inherited_property(definition, "Sim.Params").is_some()
            || self.inherited_property(definition, "Sim.Name").is_some()
            || self
                .inherited_property(definition, "Spice_Primitive")
                .is_some()
            || self.inherited_property(definition, "Spice_Model").is_some()
    }

    fn write_instance_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(symbol\n", pad));
        output.push_str(&format!(
            "{}  (lib_id {})\n",
            pad,
            sexpr_string(&self.lib_id)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(mirror) = &self.mirror {
            output.push_str(&format!("{}  (mirror", pad));
            for axis in mirror.split_whitespace() {
                output.push(' ');
                output.push_str(&sexpr_atom_or_string(axis));
            }
            output.push_str(")\n");
        }
        if let Some(unit) = self.unit {
            output.push_str(&format!("{}  (unit {})\n", pad, unit));
        }
        if let Some(body_style) = self.body_style {
            output.push_str(&format!("{}  (body_style {})\n", pad, body_style));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "dnp", self.dnp);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "fields_autoplaced",
            self.fields_autoplaced,
        );
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for pin in &self.pins {
            pin.write_pin_ref_sexpr(output, indent + 2);
        }
        write_project_instances_sexpr(output, &self.instances, indent + 2);
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolPinRef {
    pub number: Option<String>,
    pub uuid: Option<String>,
    pub alternate: Option<String>,
}

impl KicadSymbolPinRef {
    fn write_pin_ref_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        let number = self
            .number
            .as_deref()
            .or(self.uuid.as_deref())
            .unwrap_or("?");
        output.push_str(&format!("{}(pin {}", pad, sexpr_string(number)));
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        if let Some(alternate) = &self.alternate {
            output.push_str(&format!(" (alternate {})", sexpr_string(alternate)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolDef {
    pub name: String,
    pub extends: Option<String>,
    pub power: Option<KicadSymbolPower>,
    pub body_styles: Option<KicadSymbolBodyStyles>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub in_pos_files: Option<bool>,
    pub duplicate_pin_numbers_are_jumpers: Option<bool>,
    pub jumper_pin_groups: Vec<Vec<String>>,
    pub embedded_fonts: Option<bool>,
    pub pin_names: Option<KicadPinDisplay>,
    pub pin_numbers: Option<KicadPinDisplay>,
    pub unit_names: BTreeMap<u32, String>,
    pub properties: Vec<KicadProperty>,
    pub graphics: Vec<KicadSymbolGraphic>,
    pub pins: Vec<KicadPinDef>,
}

impl KicadSymbolDef {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn description(&self) -> Option<&str> {
        self.property("Description")
            .filter(|value| !value.is_empty())
            .or_else(|| {
                self.property("ki_description")
                    .filter(|value| !value.is_empty())
            })
    }

    pub fn keywords(&self) -> Option<&str> {
        self.property("ki_keywords")
            .filter(|value| !value.is_empty())
    }

    pub fn footprint_filters(&self) -> Vec<String> {
        self.property("ki_fp_filters")
            .map(parse_kicad_footprint_filters)
            .unwrap_or_default()
    }

    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let mut bounds = KicadBoundingBoxBuilder::default();
        for graphic in scoped_definition_graphics(self, Some(1), None) {
            graphic.include_in_bounds(&mut bounds);
        }
        for pin in scoped_symbol_pins(self, Some(1), None) {
            if let Some(at) = pin.at {
                bounds.include(at.point());
                if let Some(length) = pin.length {
                    bounds.include(pin_body_end(at, length));
                }
            }
        }
        bounds.finish()
    }

    pub fn local_name(&self) -> &str {
        self.name
            .rsplit_once(':')
            .map(|(_, local_name)| local_name)
            .unwrap_or(&self.name)
    }

    fn write_symbol_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(symbol {}\n", pad, sexpr_string(&self.name)));
        if let Some(extends) = &self.extends {
            output.push_str(&format!("{}  (extends {})\n", pad, sexpr_string(extends)));
        }
        if let Some(power) = self.power {
            match power {
                KicadSymbolPower::Bare => output.push_str(&format!("{}  (power)\n", pad)),
                KicadSymbolPower::Global => output.push_str(&format!("{}  (power global)\n", pad)),
                KicadSymbolPower::Local => output.push_str(&format!("{}  (power local)\n", pad)),
            }
        }
        if let Some(body_styles) = &self.body_styles {
            body_styles.write_body_styles_sexpr(output, indent + 2);
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "in_pos_files", self.in_pos_files);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "duplicate_pin_numbers_are_jumpers",
            self.duplicate_pin_numbers_are_jumpers,
        );
        if !self.jumper_pin_groups.is_empty() {
            output.push_str(&format!("{}  (jumper_pin_groups", pad));
            for group in &self.jumper_pin_groups {
                output.push('\n');
                output.push_str(&format!("{}    (", pad));
                for (index, pin_name) in group.iter().enumerate() {
                    if index > 0 {
                        output.push(' ');
                    }
                    output.push_str(&sexpr_string(pin_name));
                }
                output.push(')');
            }
            output.push_str(&format!("\n{}  )\n", pad));
        }
        write_optional_bool_sexpr(output, indent + 2, "embedded_fonts", self.embedded_fonts);
        if let Some(pin_numbers) = &self.pin_numbers {
            pin_numbers.write_pin_numbers_sexpr(output, indent + 2);
        }
        if let Some(pin_names) = &self.pin_names {
            pin_names.write_pin_names_sexpr(output, indent + 2);
        }
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for scope in self.item_scopes() {
            output.push_str(&format!(
                "{}  (symbol {}\n",
                pad,
                sexpr_string(&format!(
                    "{}_{}_{}",
                    self.local_name(),
                    scope.unit,
                    scope.body_style
                ))
            ));
            if let Some(unit_name) = self.unit_names.get(&scope.unit) {
                output.push_str(&format!(
                    "{}    (unit_name {})\n",
                    pad,
                    sexpr_string(unit_name)
                ));
            }
            for graphic in self.graphics.iter().filter(|graphic| {
                graphic.unit == scope.unit && graphic.body_style == scope.body_style
            }) {
                graphic.write_symbol_graphic_sexpr(output, indent + 4);
            }
            for pin in self
                .pins
                .iter()
                .filter(|pin| pin.unit == scope.unit && pin.body_style == scope.body_style)
            {
                pin.write_pin_sexpr(output, indent + 4);
            }
            output.push_str(&format!("{}  )\n", pad));
        }
        output.push_str(&format!("{})\n", pad));
    }

    fn item_scopes(&self) -> Vec<KicadSymbolItemScope> {
        let mut scopes = self
            .graphics
            .iter()
            .map(|graphic| KicadSymbolItemScope {
                unit: graphic.unit,
                body_style: graphic.body_style,
            })
            .chain(self.pins.iter().map(|pin| KicadSymbolItemScope {
                unit: pin.unit,
                body_style: pin.body_style,
            }))
            .chain(self.unit_names.keys().map(|unit| KicadSymbolItemScope {
                unit: *unit,
                body_style: 1,
            }))
            .collect::<BTreeSet<_>>();
        if scopes.is_empty() && self.extends.is_none() {
            scopes.insert(KicadSymbolItemScope {
                unit: 0,
                body_style: 1,
            });
        }
        scopes.into_iter().collect()
    }
}

trait KicadSymbolPropertySource {
    fn property_value(&self, name: &str) -> Option<&str>;
    fn exclude_from_sim_value(&self) -> Option<bool>;
}

impl KicadSymbolPropertySource for KicadSymbolDef {
    fn property_value(&self, name: &str) -> Option<&str> {
        self.property(name)
    }

    fn exclude_from_sim_value(&self) -> Option<bool> {
        self.exclude_from_sim
    }
}

#[derive(Debug, Clone, PartialEq)]
struct KicadResolvedSymbolDef {
    name: String,
    exclude_from_sim: Option<bool>,
    body_styles: Option<KicadSymbolBodyStyles>,
    pin_names: Option<KicadPinDisplay>,
    pin_numbers: Option<KicadPinDisplay>,
    unit_names: BTreeMap<u32, String>,
    properties: Vec<KicadProperty>,
    graphics: Vec<KicadSymbolGraphic>,
    pins: Vec<KicadPinDef>,
}

impl KicadResolvedSymbolDef {
    fn from_symbol(symbol: &KicadSymbolDef) -> Self {
        Self {
            name: symbol.name.clone(),
            exclude_from_sim: symbol.exclude_from_sim,
            body_styles: symbol.body_styles.clone(),
            pin_names: symbol.pin_names.clone(),
            pin_numbers: symbol.pin_numbers.clone(),
            unit_names: symbol.unit_names.clone(),
            properties: symbol.properties.clone(),
            graphics: symbol.graphics.clone(),
            pins: symbol.pins.clone(),
        }
    }

    fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    fn description(&self) -> Option<&str> {
        self.property("Description")
            .filter(|value| !value.is_empty())
            .or_else(|| {
                self.property("ki_description")
                    .filter(|value| !value.is_empty())
            })
    }

    fn keywords(&self) -> Option<&str> {
        self.property("ki_keywords")
            .filter(|value| !value.is_empty())
    }

    fn footprint_filters(&self) -> Vec<String> {
        self.property("ki_fp_filters")
            .map(parse_kicad_footprint_filters)
            .unwrap_or_default()
    }

    fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let mut bounds = KicadBoundingBoxBuilder::default();
        for graphic in self.scoped_graphics(Some(1), None) {
            graphic.include_in_bounds(&mut bounds);
        }
        for pin in self.scoped_pins(Some(1), None) {
            if let Some(at) = pin.at {
                bounds.include(at.point());
                if let Some(length) = pin.length {
                    bounds.include(pin_body_end(at, length));
                }
            }
        }
        bounds.finish()
    }

    fn indexed_units(&self) -> Vec<KicadIndexedSymbolUnit> {
        let mut units = self
            .pins
            .iter()
            .map(|pin| pin.unit)
            .chain(self.graphics.iter().map(|graphic| graphic.unit))
            .chain(self.unit_names.keys().copied())
            .filter(|unit| *unit != 0)
            .collect::<BTreeSet<_>>();
        if units.is_empty() {
            units.insert(1);
        }
        units
            .into_iter()
            .map(|unit| KicadIndexedSymbolUnit {
                unit,
                name: self.unit_names.get(&unit).cloned(),
            })
            .collect()
    }

    fn unit_count(&self) -> usize {
        self.indexed_units().len()
    }

    fn indexed_body_styles(&self) -> Vec<KicadIndexedSymbolBodyStyle> {
        let mut body_styles = self
            .pins
            .iter()
            .map(|pin| pin.body_style)
            .chain(self.graphics.iter().map(|graphic| graphic.body_style))
            .filter(|body_style| *body_style != 0)
            .collect::<BTreeSet<_>>();
        if let Some(declared_body_styles) = &self.body_styles {
            body_styles.extend(declared_body_styles.body_style_numbers());
        }

        body_styles
            .into_iter()
            .map(|body_style| KicadIndexedSymbolBodyStyle {
                body_style,
                name: self.body_style_name(body_style),
            })
            .collect()
    }

    fn body_style_name(&self, body_style: u32) -> Option<String> {
        match &self.body_styles {
            Some(KicadSymbolBodyStyles::Demorgan) => match body_style {
                1 => Some("normal".to_string()),
                2 => Some("demorgan".to_string()),
                _ => None,
            },
            Some(KicadSymbolBodyStyles::Names(names)) => {
                names.get(body_style.saturating_sub(1) as usize).cloned()
            }
            None => None,
        }
    }

    fn indexed_pins(&self) -> Vec<KicadIndexedSymbolPin> {
        self.pins
            .iter()
            .map(|pin| KicadIndexedSymbolPin {
                number: pin.number().to_string(),
                name: pin.name().to_string(),
                electrical_type: pin.electrical_type.clone(),
                shape: pin.shape.clone(),
                unit: pin.unit,
                body_style: pin.body_style,
                alternates: pin.alternates.clone(),
            })
            .collect()
    }

    fn scoped_graphics(
        &self,
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> impl Iterator<Item = &KicadSymbolGraphic> {
        scoped_symbol_items(&self.graphics, unit, body_style, |graphic| {
            (graphic.unit, graphic.body_style)
        })
    }

    fn scoped_pins(
        &self,
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> impl Iterator<Item = &KicadPinDef> {
        scoped_symbol_items(&self.pins, unit, body_style, |pin| {
            (pin.unit, pin.body_style)
        })
    }
}

impl KicadSymbolPropertySource for KicadResolvedSymbolDef {
    fn property_value(&self, name: &str) -> Option<&str> {
        self.property(name)
    }

    fn exclude_from_sim_value(&self) -> Option<bool> {
        self.exclude_from_sim
    }
}

pub(crate) fn resolve_symbol_definition(
    symbol: &KicadSymbolDef,
    library_symbols: &[KicadSymbolDef],
) -> Option<KicadResolvedSymbolDef> {
    let mut chain = Vec::new();
    let mut current = symbol;
    let mut visited = BTreeSet::new();

    loop {
        if !visited.insert(current.name.clone()) {
            return Some(KicadResolvedSymbolDef::from_symbol(symbol));
        }
        chain.push(current);
        let Some(parent_name) = current.extends.as_deref() else {
            break;
        };
        let Some(parent) = find_symbol_inheritance_parent(current, parent_name, library_symbols)
        else {
            return Some(KicadResolvedSymbolDef::from_symbol(symbol));
        };
        current = parent;
    }

    let mut chain = chain.into_iter().rev();
    let root = chain.next()?;
    let mut resolved = KicadResolvedSymbolDef::from_symbol(root);
    for derived in chain {
        apply_symbol_inheritance_overrides(&mut resolved, derived);
    }
    resolved.name = symbol.name.clone();
    Some(resolved)
}

fn find_symbol_inheritance_parent<'a>(
    symbol: &KicadSymbolDef,
    parent_name: &str,
    library_symbols: &'a [KicadSymbolDef],
) -> Option<&'a KicadSymbolDef> {
    library_symbols
        .iter()
        .find(|candidate| candidate.name == parent_name)
        .or_else(|| {
            symbol
                .name
                .rsplit_once(':')
                .map(|(library, _)| format!("{library}:{parent_name}"))
                .and_then(|qualified_parent| {
                    library_symbols
                        .iter()
                        .find(|candidate| candidate.name == qualified_parent)
                })
        })
        .or_else(|| {
            library_symbols
                .iter()
                .find(|candidate| candidate.local_name() == parent_name)
        })
}

fn apply_symbol_inheritance_overrides(
    resolved: &mut KicadResolvedSymbolDef,
    derived: &KicadSymbolDef,
) {
    resolved.exclude_from_sim = derived.exclude_from_sim.or(resolved.exclude_from_sim);
    resolved.pin_names = derived
        .pin_names
        .clone()
        .or_else(|| resolved.pin_names.clone());
    resolved.pin_numbers = derived
        .pin_numbers
        .clone()
        .or_else(|| resolved.pin_numbers.clone());
    resolved.body_styles = derived
        .body_styles
        .clone()
        .or_else(|| resolved.body_styles.clone());
    for (unit, name) in &derived.unit_names {
        resolved.unit_names.insert(*unit, name.clone());
    }
    for property in &derived.properties {
        if is_inherited_symbol_browser_property(&property.name)
            && property.value.trim().is_empty()
            && resolved.property(&property.name).is_some()
        {
            continue;
        }
        if is_effective_symbol_property_override(property) {
            upsert_symbol_property(&mut resolved.properties, property.clone());
        }
    }
    if !derived.graphics.is_empty() {
        resolved.graphics.extend(derived.graphics.clone());
    }
    if !derived.pins.is_empty() {
        resolved.pins.extend(derived.pins.clone());
    }
}

fn is_effective_symbol_property_override(property: &KicadProperty) -> bool {
    !matches!(property.name.as_str(), "Reference" | "Value") || !property.value.trim().is_empty()
}

fn upsert_symbol_property(properties: &mut Vec<KicadProperty>, property: KicadProperty) {
    if let Some(existing) = properties
        .iter_mut()
        .find(|existing| existing.name == property.name)
    {
        *existing = property;
    } else {
        properties.push(property);
    }
}

fn is_inherited_symbol_browser_property(name: &str) -> bool {
    matches!(
        name,
        "Description" | "ki_description" | "ki_keywords" | "ki_fp_filters"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadSymbolPower {
    Bare,
    Global,
    Local,
}

impl KicadSymbolPower {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bare => "bare",
            Self::Global => "global",
            Self::Local => "local",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KicadSymbolBodyStyles {
    Demorgan,
    Names(Vec<String>),
}

impl KicadSymbolBodyStyles {
    fn body_style_numbers(&self) -> Vec<u32> {
        match self {
            Self::Demorgan => vec![1, 2],
            Self::Names(names) => (1..=names.len() as u32).collect(),
        }
    }

    fn write_body_styles_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(body_styles", pad));
        match self {
            Self::Demorgan => output.push_str(" demorgan"),
            Self::Names(names) => {
                for name in names {
                    output.push(' ');
                    output.push_str(&sexpr_atom_or_string(name));
                }
            }
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadPinDisplay {
    pub offset: Option<f64>,
    pub hide: Option<bool>,
}

impl KicadPinDisplay {
    fn write_pin_names_sexpr(&self, output: &mut String, indent: usize) {
        self.write_pin_display_sexpr(output, indent, "pin_names", true);
    }

    fn write_pin_numbers_sexpr(&self, output: &mut String, indent: usize) {
        self.write_pin_display_sexpr(output, indent, "pin_numbers", false);
    }

    fn write_pin_display_sexpr(
        &self,
        output: &mut String,
        indent: usize,
        name: &str,
        include_offset: bool,
    ) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}({}\n", pad, name));
        if include_offset && let Some(offset) = self.offset {
            output.push_str(&format!("{}  (offset {})\n", pad, format_number(offset)));
        }
        write_optional_bool_sexpr(output, indent + 2, "hide", self.hide);
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolGraphic {
    pub graphic: KicadGraphic,
    pub unit: u32,
    pub body_style: u32,
    pub private: bool,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl KicadSymbolGraphic {
    fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        self.graphic.include_in_bounds(bounds);
    }

    fn transformed(&self, symbol_at: KicadAt, mirror: Option<&str>) -> KicadCanvasGraphic {
        self.graphic
            .transformed(symbol_at, mirror)
            .with_uuid(self.uuid.clone())
            .with_style(self.stroke.clone(), self.fill.clone())
    }

    fn write_symbol_graphic_sexpr(&self, output: &mut String, indent: usize) {
        self.graphic.write_symbol_graphic_sexpr(
            output,
            indent,
            KicadSymbolGraphicFormat {
                private: self.private,
                stroke: self.stroke.as_ref(),
                fill: self.fill.as_ref(),
                uuid: self.uuid.as_deref(),
                locked: self.locked,
            },
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct KicadSymbolGraphicFormat<'a> {
    private: bool,
    stroke: Option<&'a KicadStroke>,
    fill: Option<&'a KicadFill>,
    uuid: Option<&'a str>,
    locked: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KicadGraphic {
    Polyline {
        points: Vec<KicadPoint>,
    },
    Bezier {
        points: Vec<KicadPoint>,
    },
    Rectangle {
        start: KicadPoint,
        end: KicadPoint,
    },
    Circle {
        center: KicadPoint,
        radius: f64,
    },
    Arc {
        start: KicadPoint,
        mid: Option<KicadPoint>,
        end: KicadPoint,
    },
    Text {
        text: String,
        at: Option<KicadAt>,
        effects: Option<KicadTextEffects>,
    },
}

impl KicadGraphic {
    fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        match self {
            Self::Polyline { points } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Bezier { points } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Rectangle { start, end } => {
                bounds.include(*start);
                bounds.include(*end);
            }
            Self::Circle { center, radius } => {
                bounds.include(KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc { start, mid, end } => {
                bounds.include(*start);
                if let Some(mid) = mid {
                    bounds.include(*mid);
                }
                bounds.include(*end);
            }
            Self::Text { at, .. } => {
                if let Some(at) = at {
                    bounds.include(at.point());
                }
            }
        }
    }

    fn transformed(&self, symbol_at: KicadAt, mirror: Option<&str>) -> KicadCanvasGraphic {
        match self {
            Self::Polyline { points } => KicadCanvasGraphic::Polyline {
                uuid: None,
                points: points
                    .iter()
                    .map(|point| transform_local_point(*point, symbol_at, mirror))
                    .collect(),
                stroke: None,
                fill: None,
            },
            Self::Bezier { points } => KicadCanvasGraphic::Bezier {
                uuid: None,
                points: points
                    .iter()
                    .map(|point| transform_local_point(*point, symbol_at, mirror))
                    .collect(),
                stroke: None,
                fill: None,
            },
            Self::Rectangle { start, end } => KicadCanvasGraphic::Rectangle {
                uuid: None,
                start: transform_local_point(*start, symbol_at, mirror),
                end: transform_local_point(*end, symbol_at, mirror),
                stroke: None,
                fill: None,
            },
            Self::Circle { center, radius } => KicadCanvasGraphic::Circle {
                uuid: None,
                center: transform_local_point(*center, symbol_at, mirror),
                radius: *radius,
                stroke: None,
                fill: None,
            },
            Self::Arc { start, mid, end } => KicadCanvasGraphic::Arc {
                uuid: None,
                start: transform_local_point(*start, symbol_at, mirror),
                mid: mid.map(|point| transform_local_point(point, symbol_at, mirror)),
                end: transform_local_point(*end, symbol_at, mirror),
                stroke: None,
                fill: None,
            },
            Self::Text { text, at, effects } => KicadCanvasGraphic::Text {
                uuid: None,
                text: text.clone(),
                at: at.map(|at| transform_local_at(at, symbol_at, mirror)),
                effects: effects.clone(),
                stroke: None,
                fill: None,
            },
        }
    }

    fn to_canvas_graphic(&self) -> KicadCanvasGraphic {
        match self {
            Self::Polyline { points } => KicadCanvasGraphic::Polyline {
                uuid: None,
                points: points.clone(),
                stroke: None,
                fill: None,
            },
            Self::Bezier { points } => KicadCanvasGraphic::Bezier {
                uuid: None,
                points: points.clone(),
                stroke: None,
                fill: None,
            },
            Self::Rectangle { start, end } => KicadCanvasGraphic::Rectangle {
                uuid: None,
                start: *start,
                end: *end,
                stroke: None,
                fill: None,
            },
            Self::Circle { center, radius } => KicadCanvasGraphic::Circle {
                uuid: None,
                center: *center,
                radius: *radius,
                stroke: None,
                fill: None,
            },
            Self::Arc { start, mid, end } => KicadCanvasGraphic::Arc {
                uuid: None,
                start: *start,
                mid: *mid,
                end: *end,
                stroke: None,
                fill: None,
            },
            Self::Text { text, at, effects } => KicadCanvasGraphic::Text {
                uuid: None,
                text: text.clone(),
                at: *at,
                effects: effects.clone(),
                stroke: None,
                fill: None,
            },
        }
    }

    fn write_symbol_graphic_sexpr(
        &self,
        output: &mut String,
        indent: usize,
        format: KicadSymbolGraphicFormat<'_>,
    ) {
        let pad = " ".repeat(indent);
        let private = if format.private { " private" } else { "" };
        match self {
            Self::Polyline { points } => {
                output.push_str(&format!("{}(polyline{}", pad, private));
                write_points_sexpr(output, points);
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Bezier { points } => {
                output.push_str(&format!("{}(bezier{}", pad, private));
                write_points_sexpr(output, points);
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Rectangle { start, end } => {
                output.push_str(&format!(
                    "{}(rectangle{} (start {} {}) (end {} {})",
                    pad,
                    private,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Circle { center, radius } => {
                output.push_str(&format!(
                    "{}(circle{} (center {} {}) (radius {})",
                    pad,
                    private,
                    format_number(center.x),
                    format_number(center.y),
                    format_number(*radius)
                ));
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Arc { start, mid, end } => {
                let mid = mid.unwrap_or(KicadPoint {
                    x: (start.x + end.x) / 2.0,
                    y: (start.y + end.y) / 2.0,
                });
                output.push_str(&format!(
                    "{}(arc{} (start {} {}) (mid {} {}) (end {} {})",
                    pad,
                    private,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(mid.x),
                    format_number(mid.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, format.stroke, 0.254);
                write_inline_fill(output, format.fill);
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
            Self::Text { text, at, effects } => {
                output.push_str(&format!("{}(text{} {}", pad, private, sexpr_string(text)));
                if let Some(at) = at {
                    output.push_str(&format!(
                        " (at {} {} {})",
                        format_number(at.x),
                        format_number(at.y),
                        format_number(at.rotation)
                    ));
                }
                write_inline_text_effects(output, effects.as_ref());
                write_symbol_graphic_metadata(output, format.uuid, format.locked);
                output.push_str(")\n");
            }
        }
    }
}

fn write_symbol_graphic_metadata(output: &mut String, uuid: Option<&str>, locked: Option<bool>) {
    if let Some(uuid) = uuid {
        output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
    }
    if locked == Some(true) {
        output.push_str(" (locked yes)");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSchematicGraphic {
    pub graphic: KicadGraphic,
    pub uuid: Option<String>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub locked: Option<bool>,
}

impl KicadSchematicGraphic {
    fn to_canvas_graphic(&self) -> KicadCanvasGraphic {
        self.graphic
            .to_canvas_graphic()
            .with_uuid(self.uuid.clone())
            .with_style(self.stroke.clone(), self.fill.clone())
    }

    fn write_schematic_graphic_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        match &self.graphic {
            KicadGraphic::Polyline { points } => {
                output.push_str(&format!("{}(polyline", pad));
                write_points_sexpr(output, points);
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_optional_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Bezier { points } => {
                output.push_str(&format!("{}(bezier", pad));
                write_points_sexpr(output, points);
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Rectangle { start, end } => {
                output.push_str(&format!(
                    "{}(rectangle (start {} {}) (end {} {})",
                    pad,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Circle { center, radius } => {
                output.push_str(&format!(
                    "{}(circle (center {} {}) (radius {})",
                    pad,
                    format_number(center.x),
                    format_number(center.y),
                    format_number(*radius)
                ));
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Arc { start, mid, end } => {
                let mid = mid.unwrap_or(KicadPoint {
                    x: (start.x + end.x) / 2.0,
                    y: (start.y + end.y) / 2.0,
                });
                output.push_str(&format!(
                    "{}(arc (start {} {}) (mid {} {}) (end {} {})",
                    pad,
                    format_number(start.x),
                    format_number(start.y),
                    format_number(mid.x),
                    format_number(mid.y),
                    format_number(end.x),
                    format_number(end.y)
                ));
                write_inline_stroke(output, self.stroke.as_ref(), 0.0);
                write_inline_fill(output, self.fill.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
            KicadGraphic::Text { text, at, effects } => {
                output.push_str(&format!("{}(text {}", pad, sexpr_string(text)));
                if let Some(at) = at {
                    output.push_str(&format!(
                        " (at {} {} {})",
                        format_number(at.x),
                        format_number(at.y),
                        format_number(at.rotation)
                    ));
                }
                write_inline_text_effects(output, effects.as_ref());
                self.write_uuid(output);
                self.write_locked(output);
                output.push_str(")\n");
            }
        }
    }

    fn write_uuid(&self, output: &mut String) {
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
    }

    fn write_locked(&self, output: &mut String) {
        if self.locked == Some(true) {
            output.push_str(" (locked yes)");
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadRuleArea {
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
}

impl KicadRuleArea {
    fn write_rule_area_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(rule_area\n", pad));
        if let Some(locked) = self.locked {
            output.push_str(&format!(
                "{}  (locked {})\n",
                pad,
                if locked { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "exclude_from_sim",
            self.exclude_from_sim,
        );
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "dnp", self.dnp);
        output.push_str(&format!("{}  (polyline", pad));
        write_points_sexpr(output, &self.points);
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        write_inline_fill(output, self.fill.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadStroke {
    pub width: Option<f64>,
    pub stroke_type: Option<String>,
    pub color: Option<KicadColor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadFill {
    pub fill_type: Option<String>,
    pub color: Option<KicadColor>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadImage {
    pub at: Option<KicadPoint>,
    pub scale: f64,
    pub data_base64: String,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl KicadImage {
    pub fn image_size_mm(&self) -> Option<KicadSize> {
        png_size_from_base64(&self.data_base64).map(|(width_px, height_px)| {
            let scale = if self.scale.is_finite() && self.scale > 0.0 {
                self.scale
            } else {
                1.0
            };
            KicadSize {
                width: f64::from(width_px) / 300.0 * 25.4 * scale,
                height: f64::from(height_px) / 300.0 * 25.4 * scale,
            }
        })
    }

    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let at = self.at?;
        let size = self.image_size_mm()?;
        Some(KicadBoundingBox {
            min: KicadPoint {
                x: at.x - size.width / 2.0,
                y: at.y - size.height / 2.0,
            },
            max: KicadPoint {
                x: at.x + size.width / 2.0,
                y: at.y + size.height / 2.0,
            },
        })
    }

    pub fn mime_type(&self) -> &'static str {
        if base64_starts_with(&self.data_base64, b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if base64_starts_with(&self.data_base64, b"\xff\xd8\xff") {
            "image/jpeg"
        } else {
            "application/octet-stream"
        }
    }

    fn write_image_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(image", pad));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {})",
                format_number(at.x),
                format_number(at.y)
            ));
        }
        if self.scale != 1.0 {
            output.push_str(&format!(" (scale {})", format_number(self.scale)));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(" (locked yes)");
        }
        output.push('\n');
        write_base64_data_sexpr(output, &self.data_base64, indent + 2);
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTable {
    pub column_count: usize,
    pub border: Option<KicadTableBorder>,
    pub separators: Option<KicadTableSeparators>,
    pub column_widths: Vec<f64>,
    pub row_heights: Vec<f64>,
    pub cells: Vec<KicadTableCell>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl KicadTable {
    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let mut bounds = KicadBoundingBoxBuilder::default();
        for cell in &self.cells {
            if let Some(cell_bounds) = cell.bounding_box() {
                bounds.include_box(cell_bounds);
            }
        }
        bounds.finish()
    }

    fn write_table_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(table\n{}  (column_count {})\n",
            pad, pad, self.column_count
        ));
        write_table_border_sexpr(output, indent + 2, self.border.as_ref());
        write_table_separators_sexpr(output, indent + 2, self.separators.as_ref());
        output.push_str(&format!("{}  (column_widths", pad));
        for width in &self.column_widths {
            output.push_str(&format!(" {}", format_number(*width)));
        }
        output.push_str(")\n");
        output.push_str(&format!("{}  (row_heights", pad));
        for height in &self.row_heights {
            output.push_str(&format!(" {}", format_number(*height)));
        }
        output.push_str(")\n");
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{}  (cells\n", pad));
        for cell in &self.cells {
            cell.write_table_cell_sexpr(output, indent + 4);
        }
        output.push_str(&format!("{}  )\n", pad));
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTableBorder {
    pub external: Option<bool>,
    pub header: Option<bool>,
    pub stroke: Option<KicadStroke>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTableSeparators {
    pub rows: Option<bool>,
    pub cols: Option<bool>,
    pub stroke: Option<KicadStroke>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTableCell {
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub column_span: usize,
    pub row_span: usize,
    pub fill: Option<KicadFill>,
    pub effects: Option<KicadTextEffects>,
    pub exclude_from_sim: Option<bool>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
}

impl KicadTableCell {
    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        kicad_rotated_rect_bounds(self.at?, self.size?)
    }

    fn write_table_cell_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(table_cell {}\n",
            pad,
            sexpr_string(&self.text)
        ));
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(size) = self.size {
            output.push_str(&format!(
                "{}  (size {} {})\n",
                pad,
                format_number(size.width),
                format_number(size.height)
            ));
        }
        if let Some(margins) = self.margins {
            output.push_str(&format!(
                "{}  (margins {} {} {} {})\n",
                pad,
                format_number(margins.left),
                format_number(margins.top),
                format_number(margins.right),
                format_number(margins.bottom)
            ));
        }
        output.push_str(&format!(
            "{}  (span {} {})\n",
            pad, self.column_span, self.row_span
        ));
        output.push_str(&format!("{} ", pad));
        write_inline_fill(output, self.fill.as_ref());
        output.push('\n');
        write_text_effects_line(output, indent + 2, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadGroup {
    pub name: String,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
    pub members: Vec<String>,
}

impl KicadGroup {
    fn write_group_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(group {}\n", pad, sexpr_string(&self.name)));
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{}  (members", pad));
        for member in &self.members {
            output.push_str(&format!(" {}", sexpr_string(member)));
        }
        output.push_str(")\n");
        output.push_str(&format!("{})\n", pad));
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct KicadPinDef {
    pub number: KicadPinText,
    pub name: KicadPinText,
    pub electrical_type: String,
    pub shape: String,
    pub unit: u32,
    pub body_style: u32,
    pub at: Option<KicadAt>,
    pub length: Option<f64>,
    pub alternates: Vec<KicadPinAlternate>,
}

impl KicadPinDef {
    pub fn number(&self) -> &str {
        &self.number.text
    }

    pub fn name(&self) -> &str {
        &self.name.text
    }

    pub fn number_effects(&self) -> Option<&KicadTextEffects> {
        self.number.effects.as_ref()
    }

    pub fn name_effects(&self) -> Option<&KicadTextEffects> {
        self.name.effects.as_ref()
    }

    fn write_pin_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(pin {} {}",
            pad,
            sexpr_atom_or_string(&self.electrical_type),
            sexpr_atom_or_string(&self.shape)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(length) = self.length {
            output.push_str(&format!(" (length {})", format_number(length)));
        }
        self.name.write_inline_pin_text_sexpr(output, "name");
        self.number.write_inline_pin_text_sexpr(output, "number");
        for alternate in &self.alternates {
            alternate.write_inline_alternate_sexpr(output);
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KicadPinAlternate {
    pub name: String,
    pub electrical_type: String,
    pub shape: String,
}

impl KicadPinAlternate {
    fn write_inline_alternate_sexpr(&self, output: &mut String) {
        output.push_str(&format!(
            " (alternate {} {} {})",
            sexpr_string(&self.name),
            sexpr_atom_or_string(&self.electrical_type),
            sexpr_atom_or_string(&self.shape)
        ));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadPinText {
    pub text: String,
    pub effects: Option<KicadTextEffects>,
}

impl KicadPinText {
    fn new(text: String, effects: Option<KicadTextEffects>) -> Self {
        Self { text, effects }
    }

    fn write_inline_pin_text_sexpr(&self, output: &mut String, name: &str) {
        output.push_str(&format!(" ({} {}", name, sexpr_string(&self.text)));
        match &self.effects {
            Some(effects) => effects.write_inline_effects_sexpr(output),
            None => output.push_str(" (effects (font (size 1.27 1.27)))"),
        }
        output.push(')');
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProperty {
    pub name: String,
    pub value: String,
    pub id: Option<u32>,
    pub at: Option<KicadAt>,
    pub hide: Option<bool>,
    pub show_name: Option<bool>,
    pub do_not_autoplace: Option<bool>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadProperty {
    fn write_property_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(property {} {}",
            pad,
            sexpr_string(&self.name),
            sexpr_string(&self.value)
        ));
        if let Some(id) = self.id {
            output.push_str(&format!(" (id {})", id));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        output.push('\n');
        write_optional_bool_sexpr(output, indent + 2, "hide", self.hide);
        write_optional_bool_sexpr(output, indent + 2, "show_name", self.show_name);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "do_not_autoplace",
            self.do_not_autoplace,
        );
        match &self.effects {
            Some(effects) => effects.write_effects_sexpr(output, indent + 2),
            None => output.push_str(&format!("{}  (effects (font (size 1.27 1.27)))\n", pad)),
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextEffects {
    pub font_size: Option<KicadSize>,
    pub font_thickness: Option<f64>,
    pub font_bold: Option<bool>,
    pub font_italic: Option<bool>,
    pub font_color: Option<KicadColor>,
    pub justify: Vec<String>,
    pub hide: bool,
    pub href: Option<String>,
}

impl KicadTextEffects {
    fn write_effects_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(effects", pad));
        self.write_font_sexpr(output);
        self.write_effect_tail(output);
        output.push_str(")\n");
    }

    fn write_inline_effects_sexpr(&self, output: &mut String) {
        output.push_str(" (effects");
        self.write_font_sexpr(output);
        self.write_effect_tail(output);
        output.push(')');
    }

    fn write_font_sexpr(&self, output: &mut String) {
        output.push_str(" (font");
        if let Some(size) = self.font_size {
            output.push_str(&format!(
                " (size {} {})",
                format_number(size.width),
                format_number(size.height)
            ));
        } else {
            output.push_str(" (size 1.27 1.27)");
        }
        if let Some(thickness) = self.font_thickness {
            output.push_str(&format!(" (thickness {})", format_number(thickness)));
        }
        write_inline_optional_bool_sexpr(output, "bold", self.font_bold);
        write_inline_optional_bool_sexpr(output, "italic", self.font_italic);
        if let Some(color) = self.font_color {
            color.write_inline_color_sexpr(output);
        }
        output.push(')');
    }

    fn write_effect_tail(&self, output: &mut String) {
        if !self.justify.is_empty() {
            output.push_str(" (justify");
            for token in &self.justify {
                output.push_str(&format!(" {}", sexpr_atom_or_string(token)));
            }
            output.push(')');
        }
        if self.hide {
            output.push_str(" hide");
        }
        if let Some(href) = &self.href {
            output.push_str(&format!(" (href {})", sexpr_string(href)));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

impl KicadColor {
    fn write_inline_color_sexpr(self, output: &mut String) {
        output.push_str(&format!(
            " (color {} {} {} {})",
            format_number(self.red),
            format_number(self.green),
            format_number(self.blue),
            format_number(self.alpha)
        ));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTitleBlock {
    pub title: Option<String>,
    pub date: Option<String>,
    pub revision: Option<String>,
    pub company: Option<String>,
    pub comments: Vec<KicadTitleComment>,
}

impl KicadTitleBlock {
    fn write_title_block_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(title_block\n", pad));
        if let Some(title) = &self.title {
            output.push_str(&format!("{}  (title {})\n", pad, sexpr_string(title)));
        }
        if let Some(date) = &self.date {
            output.push_str(&format!("{}  (date {})\n", pad, sexpr_string(date)));
        }
        if let Some(revision) = &self.revision {
            output.push_str(&format!("{}  (rev {})\n", pad, sexpr_string(revision)));
        }
        if let Some(company) = &self.company {
            output.push_str(&format!("{}  (company {})\n", pad, sexpr_string(company)));
        }
        for comment in &self.comments {
            output.push_str(&format!(
                "{}  (comment {} {})\n",
                pad,
                comment.index,
                sexpr_string(&comment.text)
            ));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTitleComment {
    pub index: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheetInstance {
    pub path: String,
    pub page: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolPathInstance {
    pub path: String,
    pub reference: Option<String>,
    pub unit: Option<u32>,
    pub value: Option<String>,
    pub footprint: Option<String>,
    pub variants: Vec<KicadVariantInstance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadProjectInstance {
    pub name: String,
    pub paths: Vec<KicadInstancePath>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadInstancePath {
    pub path: String,
    pub page: Option<String>,
    pub reference: Option<String>,
    pub unit: Option<u32>,
    pub value: Option<String>,
    pub footprint: Option<String>,
    pub variants: Vec<KicadVariantInstance>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadVariantInstance {
    pub name: Option<String>,
    pub dnp: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadWire {
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub uuid: Option<String>,
}

impl KicadWire {
    fn write_wire_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(wire", pad));
        write_points_sexpr(output, &self.points);
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadBusAlias {
    pub name: String,
    pub members: Vec<String>,
}

impl KicadBusAlias {
    fn write_bus_alias_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        let members = self
            .members
            .iter()
            .map(|member| sexpr_string(member))
            .collect::<Vec<_>>()
            .join(" ");
        output.push_str(&format!(
            "{}(bus_alias {} (members {}))\n",
            pad,
            sexpr_string(&self.name),
            members
        ));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadBus {
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub uuid: Option<String>,
}

impl KicadBus {
    fn write_bus_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(bus", pad));
        write_points_sexpr(output, &self.points);
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadBusEntry {
    pub at: KicadPoint,
    pub size: KicadSize,
    pub stroke: Option<KicadStroke>,
    pub uuid: Option<String>,
}

impl KicadBusEntry {
    pub fn end(&self) -> KicadPoint {
        KicadPoint {
            x: self.at.x + self.size.width,
            y: self.at.y + self.size.height,
        }
    }

    fn write_bus_entry_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(bus_entry\n{}  (at {} {})\n{}  (size {} {})\n",
            pad,
            pad,
            format_number(self.at.x),
            format_number(self.at.y),
            pad,
            format_number(self.size.width),
            format_number(self.size.height)
        ));
        output.push_str(&format!("{}  ", pad));
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        output.push('\n');
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNetChain {
    pub name: String,
    pub from: Option<KicadNetChainEndpoint>,
    pub to: Option<KicadNetChainEndpoint>,
    pub net_class: Option<String>,
    pub color: Option<KicadColor>,
    pub member_nets: Vec<String>,
    pub extra: Vec<Sexp>,
}

impl KicadNetChain {
    fn write_net_chain_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(net_chain {}", pad, sexpr_string(&self.name)));
        if let Some(from) = &self.from {
            output.push_str(&format!(
                " (from {} {})",
                sexpr_string(&from.reference),
                sexpr_string(&from.pin)
            ));
        }
        if let Some(to) = &self.to {
            output.push_str(&format!(
                " (to {} {})",
                sexpr_string(&to.reference),
                sexpr_string(&to.pin)
            ));
        }
        if let Some(net_class) = &self.net_class {
            output.push_str(&format!(" (net_class {})", sexpr_string(net_class)));
        }
        if let Some(color) = self.color {
            color.write_inline_color_sexpr(output);
        }
        if !self.member_nets.is_empty() {
            output.push_str(" (nets");
            for net in &self.member_nets {
                output.push_str(&format!(" {}", sexpr_string(net)));
            }
            output.push(')');
        }
        for item in &self.extra {
            output.push(' ');
            write_sexpr_inline(output, item);
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNetChainEndpoint {
    pub reference: String,
    pub pin: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadLabel {
    pub text: String,
    pub kind: KicadLabelKind,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
    pub shape: Option<String>,
    pub fields_autoplaced: Option<bool>,
    pub effects: Option<KicadTextEffects>,
    pub properties: Vec<KicadProperty>,
}

impl KicadLabel {
    fn write_label_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}({} {}",
            pad,
            self.kind.sexpr_name(),
            sexpr_string(&self.text)
        ));
        if let Some(shape) = &self.shape {
            output.push_str(&format!(" (shape {})", sexpr_atom_or_string(shape)));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(fields_autoplaced) = self.fields_autoplaced {
            output.push_str(&format!(
                " (fields_autoplaced {})",
                if fields_autoplaced { "yes" } else { "no" }
            ));
        }
        write_inline_text_effects(output, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        if self.properties.is_empty() {
            output.push_str(")\n");
        } else {
            output.push('\n');
            for property in &self.properties {
                property.write_property_sexpr(output, indent + 2);
            }
            output.push_str(&format!("{})\n", pad));
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadDirectiveLabel {
    pub text: String,
    pub length: Option<f64>,
    pub shape: Option<String>,
    pub at: Option<KicadAt>,
    pub fields_autoplaced: Option<bool>,
    pub effects: Option<KicadTextEffects>,
    pub uuid: Option<String>,
    pub properties: Vec<KicadProperty>,
}

impl KicadDirectiveLabel {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn display_text(&self) -> &str {
        ["Netclass", "Net Class", "Component Class"]
            .into_iter()
            .find_map(|name| self.property(name).filter(|value| !value.is_empty()))
            .unwrap_or(&self.text)
    }

    fn write_directive_label_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(netclass_flag {}",
            pad,
            sexpr_string(&self.text)
        ));
        if let Some(length) = self.length {
            output.push_str(&format!(" (length {})", format_number(length)));
        }
        if let Some(shape) = &self.shape {
            output.push_str(&format!(" (shape {})", sexpr_atom_or_string(shape)));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(fields_autoplaced) = self.fields_autoplaced {
            output.push_str(&format!(
                " (fields_autoplaced {})",
                if fields_autoplaced { "yes" } else { "no" }
            ));
        }
        write_inline_text_effects(output, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        if self.properties.is_empty() {
            output.push_str(")\n");
        } else {
            output.push('\n');
            for property in &self.properties {
                property.write_property_sexpr(output, indent + 2);
            }
            output.push_str(&format!("{})\n", pad));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KicadLabelKind {
    Local,
    Global,
    Hierarchical,
}

impl KicadLabelKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
            Self::Hierarchical => "hierarchical",
        }
    }

    fn sexpr_name(self) -> &'static str {
        match self {
            Self::Local => "label",
            Self::Global => "global_label",
            Self::Hierarchical => "hierarchical_label",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheet {
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub uuid: Option<String>,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: Option<bool>,
    pub fields_autoplaced: Option<bool>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub properties: Vec<KicadProperty>,
    pub pins: Vec<KicadSheetPin>,
    pub instances: Vec<KicadProjectInstance>,
}

impl KicadSheet {
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.name == name)
            .map(|property| property.value.as_str())
    }

    pub fn sheet_name(&self) -> Option<&str> {
        self.property("Sheetname")
    }

    pub fn sheet_file(&self) -> Option<&str> {
        self.property("Sheetfile")
    }

    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        let at = self.at?;
        let size = self.size?;
        Some(KicadBoundingBox {
            min: at.point(),
            max: KicadPoint {
                x: at.x + size.width,
                y: at.y + size.height,
            },
        })
    }

    fn write_sheet_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(sheet\n", pad));
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y)
            ));
        }
        if let Some(size) = self.size {
            output.push_str(&format!(
                "{}  (size {} {})\n",
                pad,
                format_number(size.width),
                format_number(size.height)
            ));
        }
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        write_optional_bool_sexpr(output, indent + 2, "in_bom", self.in_bom);
        write_optional_bool_sexpr(output, indent + 2, "on_board", self.on_board);
        write_optional_bool_sexpr(output, indent + 2, "dnp", self.dnp);
        write_optional_bool_sexpr(
            output,
            indent + 2,
            "fields_autoplaced",
            self.fields_autoplaced,
        );
        if self.stroke.is_some() {
            output.push_str(&format!("{} ", pad));
            write_inline_stroke(output, self.stroke.as_ref(), 0.0);
            output.push('\n');
        }
        if self.fill.is_some() {
            output.push_str(&format!("{} ", pad));
            write_inline_fill(output, self.fill.as_ref());
            output.push('\n');
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        for property in &self.properties {
            property.write_property_sexpr(output, indent + 2);
        }
        for pin in &self.pins {
            pin.write_sheet_pin_sexpr(output, indent + 2);
        }
        write_project_instances_sexpr(output, &self.instances, indent + 2);
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSheetPin {
    pub name: String,
    pub pin_type: String,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadSheetPin {
    fn write_sheet_pin_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(pin {} {}",
            pad,
            sexpr_string(&self.name),
            sexpr_atom_or_string(&self.pin_type)
        ));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        write_inline_text_effects(output, self.effects.as_ref());
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextItem {
    pub text: String,
    pub at: Option<KicadAt>,
    pub uuid: Option<String>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadTextItem {
    fn write_text_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(text {}", pad, sexpr_string(&self.text)));
        if let Some(at) = self.at {
            output.push_str(&format!(
                " (at {} {} {})",
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        write_inline_text_effects(output, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!(" (uuid {})", sexpr_string(uuid)));
        }
        output.push_str(")\n");
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadTextBox {
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub exclude_from_sim: Option<bool>,
    pub uuid: Option<String>,
    pub locked: Option<bool>,
    pub effects: Option<KicadTextEffects>,
}

impl KicadTextBox {
    pub fn bounding_box(&self) -> Option<KicadBoundingBox> {
        kicad_rotated_rect_bounds(self.at?, self.size?)
    }

    fn write_text_box_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!("{}(text_box {}\n", pad, sexpr_string(&self.text)));
        if let Some(exclude_from_sim) = self.exclude_from_sim {
            output.push_str(&format!(
                "{}  (exclude_from_sim {})\n",
                pad,
                if exclude_from_sim { "yes" } else { "no" }
            ));
        }
        if let Some(at) = self.at {
            output.push_str(&format!(
                "{}  (at {} {} {})\n",
                pad,
                format_number(at.x),
                format_number(at.y),
                format_number(at.rotation)
            ));
        }
        if let Some(size) = self.size {
            output.push_str(&format!(
                "{}  (size {} {})\n",
                pad,
                format_number(size.width),
                format_number(size.height)
            ));
        }
        if let Some(margins) = self.margins {
            output.push_str(&format!(
                "{}  (margins {} {} {} {})\n",
                pad,
                format_number(margins.left),
                format_number(margins.top),
                format_number(margins.right),
                format_number(margins.bottom)
            ));
        }
        output.push_str(&format!("{} ", pad));
        write_inline_stroke(output, self.stroke.as_ref(), 0.0);
        output.push('\n');
        output.push_str(&format!("{} ", pad));
        write_inline_fill(output, self.fill.as_ref());
        output.push('\n');
        write_text_effects_line(output, indent + 2, self.effects.as_ref());
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        if self.locked == Some(true) {
            output.push_str(&format!("{}  (locked yes)\n", pad));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadMargins {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadJunction {
    pub at: KicadPoint,
    pub diameter: Option<f64>,
    pub color: Option<KicadColor>,
    pub uuid: Option<String>,
}

impl KicadJunction {
    fn write_junction_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(junction\n{}  (at {} {})\n{}  (diameter {})\n{} ",
            pad,
            pad,
            format_number(self.at.x),
            format_number(self.at.y),
            pad,
            format_number(self.diameter.unwrap_or(0.0)),
            pad
        ));
        self.color
            .unwrap_or(KicadColor {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.0,
            })
            .write_inline_color_sexpr(output);
        output.push('\n');
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNoConnect {
    pub at: KicadPoint,
    pub uuid: Option<String>,
}

impl KicadNoConnect {
    fn write_no_connect_sexpr(&self, output: &mut String, indent: usize) {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}(no_connect\n{}  (at {} {})\n",
            pad,
            pad,
            format_number(self.at.x),
            format_number(self.at.y)
        ));
        if let Some(uuid) = &self.uuid {
            output.push_str(&format!("{}  (uuid {})\n", pad, sexpr_string(uuid)));
        }
        output.push_str(&format!("{})\n", pad));
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadAt {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
}

impl KicadAt {
    fn point(self) -> KicadPoint {
        KicadPoint {
            x: self.x,
            y: self.y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PointKey {
    x: i64,
    y: i64,
}

impl From<KicadPoint> for PointKey {
    fn from(point: KicadPoint) -> Self {
        Self {
            x: coordinate_key(point.x),
            y: coordinate_key(point.y),
        }
    }
}

#[derive(Debug)]
struct DisjointSet {
    parents: Vec<usize>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parents: (0..len).collect(),
        }
    }

    fn find(&mut self, item: usize) -> usize {
        let parent = self.parents[item];
        if parent == item {
            item
        } else {
            let root = self.find(parent);
            self.parents[item] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents[right_root] = left_root;
        }
    }
}
fn parse_symbol_instance(node: &Sexp) -> Option<KicadSymbolInstance> {
    let items = list_items(node);
    Some(KicadSymbolInstance {
        lib_id: child_value(items, "lib_id")?,
        at: child(items, "at").and_then(parse_at),
        mirror: child(items, "mirror").and_then(parse_symbol_mirror),
        unit: child_value(items, "unit").and_then(|value| value.parse().ok()),
        body_style: child_value(items, "body_style")
            .or_else(|| child_value(items, "convert"))
            .and_then(|value| value.parse().ok()),
        uuid: child_value(items, "uuid"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        pins: direct_children(items, "pin")
            .filter_map(parse_symbol_pin_ref)
            .collect(),
        instances: child(items, "instances")
            .map(parse_project_instances)
            .unwrap_or_default(),
    })
}

fn parse_symbol_mirror(node: &Sexp) -> Option<String> {
    let mut axes = BTreeSet::new();
    for axis in list_items(node).iter().skip(1).filter_map(atom_text) {
        match axis {
            "x" | "y" => {
                axes.insert(axis);
            }
            _ => return None,
        }
    }
    symbol_mirror_from_axes(axes)
}

fn parse_symbol_pin_ref(node: &Sexp) -> Option<KicadSymbolPinRef> {
    let items = list_items(node);
    Some(KicadSymbolPinRef {
        number: list_value(node, 1),
        uuid: child_value(items, "uuid"),
        alternate: child_value(items, "alternate"),
    })
}

fn parse_title_block(node: &Sexp) -> KicadTitleBlock {
    let items = list_items(node);
    KicadTitleBlock {
        title: child_value(items, "title"),
        date: child_value(items, "date"),
        revision: child_value(items, "rev"),
        company: child_value(items, "company"),
        comments: direct_children(items, "comment")
            .filter_map(parse_title_comment)
            .collect(),
    }
}

fn parse_title_comment(node: &Sexp) -> Option<KicadTitleComment> {
    Some(KicadTitleComment {
        index: list_value(node, 1)?.parse().ok()?,
        text: list_value(node, 2)?,
    })
}

fn parse_sheet_instances(node: &Sexp) -> Vec<KicadSheetInstance> {
    direct_children(list_items(node), "path")
        .filter_map(parse_sheet_instance)
        .collect()
}

fn parse_sheet_instance(node: &Sexp) -> Option<KicadSheetInstance> {
    let items = list_items(node);
    Some(KicadSheetInstance {
        path: list_value(node, 1)?,
        page: child_value(items, "page"),
    })
}

fn parse_symbol_path_instances(node: &Sexp) -> Vec<KicadSymbolPathInstance> {
    direct_children(list_items(node), "path")
        .filter_map(parse_symbol_path_instance)
        .collect()
}

fn parse_symbol_path_instance(node: &Sexp) -> Option<KicadSymbolPathInstance> {
    let path = parse_instance_path(node)?;
    Some(KicadSymbolPathInstance {
        path: path.path,
        reference: path.reference,
        unit: path.unit,
        value: path.value,
        footprint: path.footprint,
        variants: path.variants,
    })
}

fn parse_project_instances(node: &Sexp) -> Vec<KicadProjectInstance> {
    direct_children(list_items(node), "project")
        .filter_map(parse_project_instance)
        .collect()
}

fn parse_project_instance(node: &Sexp) -> Option<KicadProjectInstance> {
    let items = list_items(node);
    Some(KicadProjectInstance {
        name: list_value(node, 1)?,
        paths: direct_children(items, "path")
            .filter_map(parse_instance_path)
            .collect(),
    })
}

fn parse_instance_path(node: &Sexp) -> Option<KicadInstancePath> {
    let items = list_items(node);
    Some(KicadInstancePath {
        path: list_value(node, 1)?,
        page: child_value(items, "page"),
        reference: child_value(items, "reference"),
        unit: child_value(items, "unit").and_then(|value| value.parse().ok()),
        value: child_value(items, "value"),
        footprint: child_value(items, "footprint"),
        variants: direct_children(items, "variant")
            .filter_map(parse_variant_instance)
            .collect(),
    })
}

fn parse_variant_instance(node: &Sexp) -> Option<KicadVariantInstance> {
    let items = list_items(node);
    Some(KicadVariantInstance {
        name: child_value(items, "name"),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
    })
}

fn write_sheet_instances_sexpr(
    output: &mut String,
    instances: &[KicadSheetInstance],
    indent: usize,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(sheet_instances\n", pad));
    for instance in instances {
        output.push_str(&format!("{}  (path {}", pad, sexpr_string(&instance.path)));
        if let Some(page) = &instance.page {
            output.push_str(&format!(" (page {})", sexpr_string(page)));
        }
        output.push_str(")\n");
    }
    output.push_str(&format!("{})\n", pad));
}

fn write_symbol_path_instances_sexpr(
    output: &mut String,
    instances: &[KicadSymbolPathInstance],
    indent: usize,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(symbol_instances\n", pad));
    for instance in instances {
        output.push_str(&format!(
            "{}  (path {}\n",
            pad,
            sexpr_string(&instance.path)
        ));
        if let Some(reference) = &instance.reference {
            output.push_str(&format!(
                "{}    (reference {})\n",
                pad,
                sexpr_string(reference)
            ));
        }
        if let Some(unit) = instance.unit {
            output.push_str(&format!("{}    (unit {})\n", pad, unit));
        }
        if let Some(value) = &instance.value {
            output.push_str(&format!("{}    (value {})\n", pad, sexpr_string(value)));
        }
        if let Some(footprint) = &instance.footprint {
            output.push_str(&format!(
                "{}    (footprint {})\n",
                pad,
                sexpr_string(footprint)
            ));
        }
        for variant in &instance.variants {
            write_variant_instance_sexpr(output, variant, indent + 4);
        }
        output.push_str(&format!("{}  )\n", pad));
    }
    output.push_str(&format!("{})\n", pad));
}

fn write_project_instances_sexpr(
    output: &mut String,
    instances: &[KicadProjectInstance],
    indent: usize,
) {
    if instances.is_empty() {
        return;
    }
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(instances\n", pad));
    for instance in instances {
        output.push_str(&format!(
            "{}  (project {}\n",
            pad,
            sexpr_string(&instance.name)
        ));
        for path in &instance.paths {
            write_instance_path_sexpr(output, path, indent + 4);
        }
        output.push_str(&format!("{}  )\n", pad));
    }
    output.push_str(&format!("{})\n", pad));
}

fn write_instance_path_sexpr(output: &mut String, path: &KicadInstancePath, indent: usize) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(path {}\n", pad, sexpr_string(&path.path)));
    if let Some(page) = &path.page {
        output.push_str(&format!("{}  (page {})\n", pad, sexpr_string(page)));
    }
    if let Some(reference) = &path.reference {
        output.push_str(&format!(
            "{}  (reference {})\n",
            pad,
            sexpr_string(reference)
        ));
    }
    if let Some(unit) = path.unit {
        output.push_str(&format!("{}  (unit {})\n", pad, unit));
    }
    if let Some(value) = &path.value {
        output.push_str(&format!("{}  (value {})\n", pad, sexpr_string(value)));
    }
    if let Some(footprint) = &path.footprint {
        output.push_str(&format!(
            "{}  (footprint {})\n",
            pad,
            sexpr_string(footprint)
        ));
    }
    for variant in &path.variants {
        write_variant_instance_sexpr(output, variant, indent + 2);
    }
    output.push_str(&format!("{})\n", pad));
}

fn write_variant_instance_sexpr(
    output: &mut String,
    variant: &KicadVariantInstance,
    indent: usize,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(variant\n", pad));
    if let Some(name) = &variant.name {
        output.push_str(&format!("{}  (name {})\n", pad, sexpr_string(name)));
    }
    if let Some(dnp) = variant.dnp {
        output.push_str(&format!(
            "{}  (dnp {})\n",
            pad,
            if dnp { "yes" } else { "no" }
        ));
    }
    output.push_str(&format!("{})\n", pad));
}

fn write_optional_bool_sexpr(output: &mut String, indent: usize, name: &str, value: Option<bool>) {
    if let Some(value) = value {
        let pad = " ".repeat(indent);
        output.push_str(&format!(
            "{}({} {})\n",
            pad,
            name,
            if value { "yes" } else { "no" }
        ));
    }
}

fn write_inline_optional_bool_sexpr(output: &mut String, name: &str, value: Option<bool>) {
    if let Some(value) = value {
        output.push_str(&format!(" ({} {})", name, if value { "yes" } else { "no" }));
    }
}

fn write_inline_text_effects(output: &mut String, effects: Option<&KicadTextEffects>) {
    match effects {
        Some(effects) => effects.write_inline_effects_sexpr(output),
        None => output.push_str(" (effects (font (size 1.27 1.27)))"),
    }
}

fn write_text_effects_line(output: &mut String, indent: usize, effects: Option<&KicadTextEffects>) {
    match effects {
        Some(effects) => effects.write_effects_sexpr(output, indent),
        None => {
            let pad = " ".repeat(indent);
            output.push_str(&format!("{pad}(effects (font (size 1.27 1.27)))\n"));
        }
    }
}

fn write_inline_stroke(output: &mut String, stroke: Option<&KicadStroke>, default_width: f64) {
    match stroke {
        Some(stroke) => {
            output.push_str(" (stroke");
            output.push_str(&format!(
                " (width {})",
                format_number(stroke.width.unwrap_or(default_width))
            ));
            if let Some(stroke_type) = &stroke.stroke_type {
                output.push_str(&format!(" (type {})", sexpr_atom_or_string(stroke_type)));
            } else {
                output.push_str(" (type default)");
            }
            if let Some(color) = stroke.color {
                color.write_inline_color_sexpr(output);
            }
            output.push(')');
        }
        None => output.push_str(&format!(
            " (stroke (width {}) (type default))",
            format_number(default_width)
        )),
    }
}

fn write_inline_optional_fill(output: &mut String, fill: Option<&KicadFill>) {
    if let Some(fill) = fill {
        write_inline_fill(output, Some(fill));
    }
}

fn write_inline_fill(output: &mut String, fill: Option<&KicadFill>) {
    match fill {
        Some(fill) => {
            output.push_str(" (fill");
            if let Some(fill_type) = &fill.fill_type {
                output.push_str(&format!(" (type {})", sexpr_atom_or_string(fill_type)));
            } else {
                if fill.color.is_none() {
                    output.push_str(" (type none)");
                }
            }
            if let Some(color) = fill.color {
                color.write_inline_color_sexpr(output);
            }
            output.push(')');
        }
        None => output.push_str(" (fill (type none))"),
    }
}

fn write_table_border_sexpr(output: &mut String, indent: usize, border: Option<&KicadTableBorder>) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(border", pad));
    match border {
        Some(border) => {
            write_inline_optional_bool_sexpr(output, "external", border.external);
            write_inline_optional_bool_sexpr(output, "header", border.header);
            write_inline_stroke(output, border.stroke.as_ref(), 0.0);
        }
        None => output.push_str(" (external yes) (header yes) (stroke (width 0) (type solid))"),
    }
    output.push_str(")\n");
}

fn write_table_separators_sexpr(
    output: &mut String,
    indent: usize,
    separators: Option<&KicadTableSeparators>,
) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(separators", pad));
    match separators {
        Some(separators) => {
            write_inline_optional_bool_sexpr(output, "rows", separators.rows);
            write_inline_optional_bool_sexpr(output, "cols", separators.cols);
            write_inline_stroke(output, separators.stroke.as_ref(), 0.0);
        }
        None => output.push_str(" (rows yes) (cols yes) (stroke (width 0) (type solid))"),
    }
    output.push_str(")\n");
}

fn parse_symbol_def(node: &Sexp) -> Option<KicadSymbolDef> {
    let items = list_items(node);
    Some(KicadSymbolDef {
        name: list_value(node, 1)?,
        extends: child_value(items, "extends"),
        power: child(items, "power").map(parse_symbol_power),
        body_styles: child(items, "body_styles").and_then(parse_symbol_body_styles),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        in_pos_files: child_value(items, "in_pos_files").and_then(parse_kicad_bool_value),
        duplicate_pin_numbers_are_jumpers: child_value(items, "duplicate_pin_numbers_are_jumpers")
            .and_then(parse_kicad_bool_value),
        jumper_pin_groups: child(items, "jumper_pin_groups")
            .map(parse_jumper_pin_groups)
            .unwrap_or_default(),
        embedded_fonts: child_value(items, "embedded_fonts").and_then(parse_kicad_bool_value),
        pin_names: child(items, "pin_names").map(parse_pin_display),
        pin_numbers: child(items, "pin_numbers").map(parse_pin_display),
        unit_names: collect_symbol_unit_names(node),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        graphics: collect_graphics(node),
        pins: collect_pin_defs(node),
    })
}

fn collect_symbol_unit_names(node: &Sexp) -> BTreeMap<u32, String> {
    let mut unit_names = BTreeMap::new();
    collect_symbol_unit_names_into(node, &mut unit_names);
    unit_names
}

fn collect_symbol_unit_names_into(node: &Sexp, unit_names: &mut BTreeMap<u32, String>) {
    if let Some(scope) = child_symbol_item_scope(node)
        && scope.unit != 0
        && let Some(unit_name) = child_value(list_items(node), "unit_name")
    {
        unit_names.insert(scope.unit, unit_name);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            collect_symbol_unit_names_into(child, unit_names);
        }
    }
}

fn parse_symbol_power(node: &Sexp) -> KicadSymbolPower {
    match list_value(node, 1)
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("global") => KicadSymbolPower::Global,
        Some("local") => KicadSymbolPower::Local,
        _ => KicadSymbolPower::Bare,
    }
}

fn parse_symbol_body_styles(node: &Sexp) -> Option<KicadSymbolBodyStyles> {
    let names = list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if names.iter().any(|name| name == "demorgan") {
        Some(KicadSymbolBodyStyles::Demorgan)
    } else if names.is_empty() {
        None
    } else {
        Some(KicadSymbolBodyStyles::Names(names))
    }
}

fn parse_jumper_pin_groups(node: &Sexp) -> Vec<Vec<String>> {
    list_items(node)
        .iter()
        .skip(1)
        .filter_map(|group| {
            let pins = list_items(group)
                .iter()
                .filter_map(atom_text)
                .map(str::to_string)
                .collect::<Vec<_>>();
            (!pins.is_empty()).then_some(pins)
        })
        .collect()
}

fn parse_symbol_library_table_row(node: &Sexp) -> Option<KicadSymbolLibraryTableRow> {
    let items = list_items(node);
    Some(KicadSymbolLibraryTableRow {
        name: child_value(items, "name")?,
        library_type: child_value(items, "type")?,
        uri: child_value(items, "uri")?,
        options: child_value(items, "options"),
        description: child_value(items, "descr"),
        hidden: child(items, "hidden").is_some(),
        disabled: child(items, "disabled").is_some(),
    })
}

fn parse_pin_def(node: &Sexp) -> Option<KicadPinDef> {
    let items = list_items(node);
    Some(KicadPinDef {
        number: child(items, "number").and_then(parse_pin_text)?,
        name: child(items, "name")
            .and_then(parse_pin_text)
            .unwrap_or_else(|| KicadPinText::new("~".to_string(), None)),
        electrical_type: list_value(node, 1).unwrap_or_else(|| "unspecified".to_string()),
        shape: list_value(node, 2).unwrap_or_else(|| "line".to_string()),
        unit: 0,
        body_style: 0,
        at: child(items, "at").and_then(parse_at),
        length: child_value(items, "length").and_then(|value| value.parse().ok()),
        alternates: direct_children(items, "alternate")
            .filter_map(parse_pin_alternate)
            .collect(),
    })
}

fn parse_pin_alternate(node: &Sexp) -> Option<KicadPinAlternate> {
    Some(KicadPinAlternate {
        name: list_value(node, 1)?,
        electrical_type: list_value(node, 2).unwrap_or_else(|| "unspecified".to_string()),
        shape: list_value(node, 3).unwrap_or_else(|| "line".to_string()),
    })
}

fn parse_pin_display(node: &Sexp) -> KicadPinDisplay {
    let items = list_items(node);
    KicadPinDisplay {
        offset: child_value(items, "offset").and_then(|value| value.parse().ok()),
        hide: parse_optional_bool_child(items, "hide").or_else(|| {
            items
                .iter()
                .skip(1)
                .any(|item| atom_text(item) == Some("hide"))
                .then_some(true)
        }),
    }
}

fn parse_pin_text(node: &Sexp) -> Option<KicadPinText> {
    let items = list_items(node);
    Some(KicadPinText::new(
        list_value(node, 1)?,
        child(items, "effects").map(parse_text_effects),
    ))
}

fn parse_property(node: &Sexp) -> Option<KicadProperty> {
    let items = list_items(node);
    Some(KicadProperty {
        name: list_value(node, 1)?,
        value: list_value(node, 2)?,
        id: child_value(items, "id").and_then(|value| value.parse().ok()),
        at: child(items, "at").and_then(parse_at),
        hide: child_value(items, "hide").and_then(parse_kicad_bool_value),
        show_name: child_value(items, "show_name").and_then(parse_kicad_bool_value),
        do_not_autoplace: child_value(items, "do_not_autoplace").and_then(parse_kicad_bool_value),
        effects: child(items, "effects").map(parse_text_effects),
    })
}

fn parse_text_effects(node: &Sexp) -> KicadTextEffects {
    let items = list_items(node);
    let font = child(items, "font");
    let font_items = font.map(list_items).unwrap_or_default();
    KicadTextEffects {
        font_size: child(font_items, "size").and_then(parse_size),
        font_thickness: child_value(font_items, "thickness").and_then(|value| value.parse().ok()),
        font_bold: parse_effect_bool(font_items, "bold"),
        font_italic: parse_effect_bool(font_items, "italic"),
        font_color: child(font_items, "color").and_then(parse_color),
        justify: child(items, "justify")
            .map(|justify| {
                list_items(justify)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        hide: has_effect_flag(items, "hide"),
        href: child_value(items, "href"),
    }
}

fn parse_effect_bool(items: &[Sexp], name: &str) -> Option<bool> {
    child(items, name)
        .and_then(|node| list_value(node, 1).and_then(parse_kicad_bool_value))
        .or_else(|| has_effect_flag(items, name).then_some(true))
}

fn has_effect_flag(items: &[Sexp], name: &str) -> bool {
    items
        .iter()
        .skip(1)
        .any(|item| atom_text(item) == Some(name) || head(item) == Some(name))
}

fn parse_wire(node: &Sexp) -> KicadWire {
    let items = list_items(node);
    KicadWire {
        points: child(items, "pts").map(parse_points).unwrap_or_default(),
        stroke: child(items, "stroke").map(parse_stroke),
        uuid: child_value(items, "uuid"),
    }
}

fn parse_bus_alias(node: &Sexp) -> Option<KicadBusAlias> {
    let items = list_items(node);
    Some(KicadBusAlias {
        name: list_value(node, 1)?,
        members: child(items, "members")
            .map(|members| {
                list_items(members)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    })
}

fn parse_bus(node: &Sexp) -> KicadBus {
    let items = list_items(node);
    KicadBus {
        points: child(items, "pts").map(parse_points).unwrap_or_default(),
        stroke: child(items, "stroke").map(parse_stroke),
        uuid: child_value(items, "uuid"),
    }
}

fn parse_bus_entry(node: &Sexp) -> Option<KicadBusEntry> {
    let items = list_items(node);
    let at = child(items, "at").and_then(parse_at)?;
    Some(KicadBusEntry {
        at: KicadPoint { x: at.x, y: at.y },
        size: child(items, "size").and_then(parse_size)?,
        stroke: child(items, "stroke").map(parse_stroke),
        uuid: child_value(items, "uuid"),
    })
}

fn parse_net_chain(node: &Sexp) -> Option<KicadNetChain> {
    let items = list_items(node);
    let known_heads = ["from", "to", "net_class", "color", "nets"];
    Some(KicadNetChain {
        name: list_value(node, 1)?,
        from: child(items, "from").and_then(parse_net_chain_endpoint),
        to: child(items, "to").and_then(parse_net_chain_endpoint),
        net_class: child_value(items, "net_class"),
        color: child(items, "color").and_then(parse_color),
        member_nets: child(items, "nets")
            .map(|nets| {
                list_items(nets)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        extra: list_items(node)
            .iter()
            .skip(2)
            .filter(|item| {
                matches!(item, Sexp::List(_))
                    && head(item).is_none_or(|head| !known_heads.contains(&head))
            })
            .cloned()
            .collect(),
    })
}

fn parse_net_chain_endpoint(node: &Sexp) -> Option<KicadNetChainEndpoint> {
    Some(KicadNetChainEndpoint {
        reference: list_value(node, 1)?,
        pin: list_value(node, 2)?,
    })
}

fn parse_schematic_graphic(node: &Sexp) -> Option<KicadSchematicGraphic> {
    match head(node)? {
        "polyline" | "bezier" | "rectangle" | "circle" | "arc" => {
            let items = list_items(node);
            Some(KicadSchematicGraphic {
                graphic: parse_graphic(node)?,
                uuid: child_value(items, "uuid"),
                stroke: child(items, "stroke").map(parse_stroke),
                fill: child(items, "fill").map(parse_fill),
                locked: parse_optional_bool_child(items, "locked"),
            })
        }
        _ => None,
    }
}

fn parse_rule_area(node: &Sexp) -> Option<KicadRuleArea> {
    let items = list_items(node);
    let polyline = child(items, "polyline")?;
    let polyline_items = list_items(polyline);
    Some(KicadRuleArea {
        points: child(polyline_items, "pts")
            .map(parse_points)
            .unwrap_or_default(),
        stroke: child(polyline_items, "stroke").map(parse_stroke),
        fill: child(polyline_items, "fill").map(parse_fill),
        uuid: child_value(polyline_items, "uuid"),
        locked: parse_optional_bool_child(items, "locked"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
    })
}

fn parse_image(node: &Sexp) -> Option<KicadImage> {
    let items = list_items(node);
    Some(KicadImage {
        at: child(items, "at").and_then(parse_image_at),
        scale: child_value(items, "scale")
            .and_then(|value| value.parse().ok())
            .filter(|scale: &f64| scale.is_finite() && *scale > 0.0)
            .unwrap_or(1.0),
        data_base64: child(items, "data").map(parse_data_chunks)?,
        uuid: child_value(items, "uuid"),
        locked: child_value(items, "locked").and_then(parse_kicad_bool_value),
    })
}

fn parse_table(node: &Sexp) -> Option<KicadTable> {
    let items = list_items(node);
    Some(KicadTable {
        column_count: child_value(items, "column_count")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        border: child(items, "border").map(parse_table_border),
        separators: child(items, "separators").map(parse_table_separators),
        column_widths: child(items, "column_widths")
            .map(parse_number_list)
            .unwrap_or_default(),
        row_heights: child(items, "row_heights")
            .map(parse_number_list)
            .unwrap_or_default(),
        cells: child(items, "cells")
            .map(|cells| {
                direct_children(list_items(cells), "table_cell")
                    .filter_map(parse_table_cell)
                    .collect()
            })
            .unwrap_or_default(),
        uuid: child_value(items, "uuid"),
        locked: child_value(items, "locked").and_then(parse_kicad_bool_value),
    })
}

fn parse_table_cell(node: &Sexp) -> Option<KicadTableCell> {
    let items = list_items(node);
    let (column_span, row_span) = child(items, "span").map(parse_span).unwrap_or((1, 1));
    Some(KicadTableCell {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
        size: child(items, "size").and_then(parse_size),
        margins: child(items, "margins").and_then(parse_margins),
        column_span,
        row_span,
        fill: child(items, "fill").map(parse_fill),
        effects: child(items, "effects").map(parse_text_effects),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        uuid: child_value(items, "uuid"),
        locked: parse_optional_bool_child(items, "locked"),
    })
}

fn parse_table_border(node: &Sexp) -> KicadTableBorder {
    let items = list_items(node);
    KicadTableBorder {
        external: child_value(items, "external").and_then(parse_kicad_bool_value),
        header: child_value(items, "header").and_then(parse_kicad_bool_value),
        stroke: child(items, "stroke").map(parse_stroke),
    }
}

fn parse_table_separators(node: &Sexp) -> KicadTableSeparators {
    let items = list_items(node);
    KicadTableSeparators {
        rows: child_value(items, "rows").and_then(parse_kicad_bool_value),
        cols: child_value(items, "cols").and_then(parse_kicad_bool_value),
        stroke: child(items, "stroke").map(parse_stroke),
    }
}

fn parse_group(node: &Sexp) -> Option<KicadGroup> {
    let items = list_items(node);
    Some(KicadGroup {
        name: list_value(node, 1)?,
        uuid: child_value(items, "uuid"),
        locked: child_value(items, "locked").and_then(parse_kicad_bool_value),
        members: child(items, "members")
            .map(|members| {
                list_items(members)
                    .iter()
                    .skip(1)
                    .filter_map(atom_text)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn parse_label(node: &Sexp, kind: KicadLabelKind) -> Option<KicadLabel> {
    let items = list_items(node);
    Some(KicadLabel {
        text: list_value(node, 1)?,
        kind,
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
        shape: child_value(items, "shape"),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        effects: child(items, "effects").map(parse_text_effects),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
    })
}

fn parse_directive_label(node: &Sexp) -> Option<KicadDirectiveLabel> {
    let items = list_items(node);
    Some(KicadDirectiveLabel {
        text: list_value(node, 1).unwrap_or_default(),
        length: child_value(items, "length").and_then(|value| value.parse().ok()),
        shape: child_value(items, "shape"),
        at: child(items, "at").and_then(parse_at),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        effects: child(items, "effects").map(parse_text_effects),
        uuid: child_value(items, "uuid"),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
    })
}

fn parse_sheet(node: &Sexp) -> Option<KicadSheet> {
    let items = list_items(node);
    Some(KicadSheet {
        at: child(items, "at").and_then(parse_at),
        size: child(items, "size").and_then(parse_size),
        uuid: child_value(items, "uuid"),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        in_bom: child_value(items, "in_bom").and_then(parse_kicad_bool_value),
        on_board: child_value(items, "on_board").and_then(parse_kicad_bool_value),
        dnp: child_value(items, "dnp").and_then(parse_kicad_bool_value),
        fields_autoplaced: parse_optional_bool_child(items, "fields_autoplaced"),
        stroke: child(items, "stroke").map(parse_stroke),
        fill: child(items, "fill").map(parse_fill),
        properties: direct_children(items, "property")
            .filter_map(parse_property)
            .collect(),
        pins: direct_children(items, "pin")
            .filter_map(parse_sheet_pin)
            .collect(),
        instances: child(items, "instances")
            .map(parse_project_instances)
            .unwrap_or_default(),
    })
}

fn parse_sheet_pin(node: &Sexp) -> Option<KicadSheetPin> {
    let items = list_items(node);
    Some(KicadSheetPin {
        name: list_value(node, 1)?,
        pin_type: list_value(node, 2).unwrap_or_else(|| "unspecified".to_string()),
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
        effects: child(items, "effects").map(parse_text_effects),
    })
}

fn parse_text_item(node: &Sexp) -> Option<KicadTextItem> {
    let items = list_items(node);
    Some(KicadTextItem {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
        uuid: child_value(items, "uuid"),
        effects: child(items, "effects").map(parse_text_effects),
    })
}

fn parse_text_box(node: &Sexp) -> Option<KicadTextBox> {
    let items = list_items(node);
    Some(KicadTextBox {
        text: list_value(node, 1)?,
        at: child(items, "at").and_then(parse_at),
        size: child(items, "size").and_then(parse_size),
        margins: child(items, "margins").and_then(parse_margins),
        stroke: child(items, "stroke").map(parse_stroke),
        fill: child(items, "fill").map(parse_fill),
        exclude_from_sim: child_value(items, "exclude_from_sim").and_then(parse_kicad_bool_value),
        uuid: child_value(items, "uuid"),
        locked: parse_optional_bool_child(items, "locked"),
        effects: child(items, "effects").map(parse_text_effects),
    })
}

fn parse_junction(node: &Sexp) -> Option<KicadJunction> {
    let items = list_items(node);
    let at = child(items, "at").and_then(parse_at)?;
    Some(KicadJunction {
        at: KicadPoint { x: at.x, y: at.y },
        diameter: child_value(items, "diameter").and_then(|value| value.parse().ok()),
        color: child(items, "color").and_then(parse_color),
        uuid: child_value(items, "uuid"),
    })
}

fn parse_no_connect(node: &Sexp) -> Option<KicadNoConnect> {
    let items = list_items(node);
    let at = child(items, "at").and_then(parse_at)?;
    Some(KicadNoConnect {
        at: KicadPoint { x: at.x, y: at.y },
        uuid: child_value(items, "uuid"),
    })
}

fn parse_points(node: &Sexp) -> Vec<KicadPoint> {
    direct_children(list_items(node), "xy")
        .filter_map(parse_xy)
        .collect()
}

fn parse_xy(node: &Sexp) -> Option<KicadPoint> {
    let items = list_items(node);
    Some(KicadPoint {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

fn parse_image_at(node: &Sexp) -> Option<KicadPoint> {
    let items = list_items(node);
    Some(KicadPoint {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

fn parse_size(node: &Sexp) -> Option<KicadSize> {
    let items = list_items(node);
    Some(KicadSize {
        width: atom_text(items.get(1)?)?.parse().ok()?,
        height: atom_text(items.get(2)?)?.parse().ok()?,
    })
}

fn parse_color(node: &Sexp) -> Option<KicadColor> {
    let items = list_items(node);
    Some(KicadColor {
        red: atom_text(items.get(1)?)?.parse().ok()?,
        green: atom_text(items.get(2)?)?.parse().ok()?,
        blue: atom_text(items.get(3)?)?.parse().ok()?,
        alpha: atom_text(items.get(4)?)?.parse().ok()?,
    })
}

fn parse_stroke(node: &Sexp) -> KicadStroke {
    let items = list_items(node);
    KicadStroke {
        width: child_value(items, "width").and_then(|value| value.parse().ok()),
        stroke_type: child_value(items, "type"),
        color: child(items, "color").and_then(parse_color),
    }
}

fn parse_fill(node: &Sexp) -> KicadFill {
    let items = list_items(node);
    KicadFill {
        fill_type: child_value(items, "type"),
        color: child(items, "color").and_then(parse_color),
    }
}

fn parse_margins(node: &Sexp) -> Option<KicadMargins> {
    let items = list_items(node);
    Some(KicadMargins {
        left: atom_text(items.get(1)?)?.parse().ok()?,
        top: atom_text(items.get(2)?)?.parse().ok()?,
        right: atom_text(items.get(3)?)?.parse().ok()?,
        bottom: atom_text(items.get(4)?)?.parse().ok()?,
    })
}

fn parse_span(node: &Sexp) -> (usize, usize) {
    let items = list_items(node);
    let columns = items
        .get(1)
        .and_then(atom_text)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    let rows = items
        .get(2)
        .and_then(atom_text)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    (columns, rows)
}

fn parse_at(node: &Sexp) -> Option<KicadAt> {
    let items = list_items(node);
    Some(KicadAt {
        x: atom_text(items.get(1)?)?.parse().ok()?,
        y: atom_text(items.get(2)?)?.parse().ok()?,
        rotation: items
            .get(3)
            .and_then(atom_text)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0.0),
    })
}

fn collect_pin_defs(node: &Sexp) -> Vec<KicadPinDef> {
    let mut pins = Vec::new();
    collect_pin_defs_into(node, KicadSymbolItemScope::default(), &mut pins);
    pins
}

fn collect_pin_defs_into(node: &Sexp, scope: KicadSymbolItemScope, pins: &mut Vec<KicadPinDef>) {
    if head(node) == Some("pin")
        && let Some(mut pin) = parse_pin_def(node)
    {
        pin.unit = scope.unit;
        pin.body_style = scope.body_style;
        pins.push(pin);
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            let child_scope = child_symbol_item_scope(child).unwrap_or(scope);
            collect_pin_defs_into(child, child_scope, pins);
        }
    }
}

fn collect_graphics(node: &Sexp) -> Vec<KicadSymbolGraphic> {
    let mut graphics = Vec::new();
    collect_graphics_into(node, KicadSymbolItemScope::default(), &mut graphics);
    graphics
}

fn collect_graphics_into(
    node: &Sexp,
    scope: KicadSymbolItemScope,
    graphics: &mut Vec<KicadSymbolGraphic>,
) {
    if let Some(graphic) = parse_symbol_graphic(node) {
        graphics.push(KicadSymbolGraphic {
            unit: scope.unit,
            body_style: scope.body_style,
            ..graphic
        });
    }
    for child in list_items(node) {
        if matches!(child, Sexp::List(_)) {
            let child_scope = child_symbol_item_scope(child).unwrap_or(scope);
            collect_graphics_into(child, child_scope, graphics);
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
struct KicadSymbolItemScope {
    unit: u32,
    body_style: u32,
}

fn child_symbol_item_scope(node: &Sexp) -> Option<KicadSymbolItemScope> {
    if head(node) != Some("symbol") {
        return None;
    }
    parse_symbol_item_scope(list_value(node, 1)?.as_str())
}

fn parse_symbol_item_scope(name: &str) -> Option<KicadSymbolItemScope> {
    let (_, body_style) = name.rsplit_once('_')?;
    let (base, unit) = name[..name.len() - body_style.len() - 1].rsplit_once('_')?;
    if base.is_empty() {
        return None;
    }
    Some(KicadSymbolItemScope {
        unit: unit.parse().ok()?,
        body_style: body_style.parse().ok()?,
    })
}

fn parse_symbol_graphic(node: &Sexp) -> Option<KicadSymbolGraphic> {
    match head(node)? {
        "polyline" | "bezier" | "rectangle" | "circle" | "arc" | "text" => {
            let items = list_items(node);
            Some(KicadSymbolGraphic {
                graphic: parse_graphic(node)?,
                unit: 0,
                body_style: 0,
                private: items
                    .iter()
                    .skip(1)
                    .any(|item| atom_text(item) == Some("private")),
                stroke: child(items, "stroke").map(parse_stroke),
                fill: child(items, "fill").map(parse_fill),
                uuid: child_value(items, "uuid"),
                locked: parse_optional_bool_child(items, "locked"),
            })
        }
        _ => None,
    }
}

fn parse_graphic(node: &Sexp) -> Option<KicadGraphic> {
    let items = list_items(node);
    match head(node)? {
        "polyline" => {
            let points = child(items, "pts").map(parse_points).unwrap_or_default();
            (!points.is_empty()).then_some(KicadGraphic::Polyline { points })
        }
        "bezier" => {
            let points = child(items, "pts").map(parse_points).unwrap_or_default();
            (points.len() == 4).then_some(KicadGraphic::Bezier { points })
        }
        "rectangle" => Some(KicadGraphic::Rectangle {
            start: child(items, "start").and_then(parse_xy)?,
            end: child(items, "end").and_then(parse_xy)?,
        }),
        "circle" => {
            let center = child(items, "center").and_then(parse_xy)?;
            let radius = child_value(items, "radius")
                .and_then(|value| value.parse().ok())
                .or_else(|| {
                    child(items, "end")
                        .and_then(parse_xy)
                        .map(|end| ((end.x - center.x).powi(2) + (end.y - center.y).powi(2)).sqrt())
                })?;
            Some(KicadGraphic::Circle { center, radius })
        }
        "arc" => Some(KicadGraphic::Arc {
            start: child(items, "start").and_then(parse_xy)?,
            mid: child(items, "mid").and_then(parse_xy),
            end: child(items, "end").and_then(parse_xy)?,
        }),
        "text" => Some(KicadGraphic::Text {
            text: list_value(node, 1)?,
            at: child(items, "at").and_then(parse_at),
            effects: child(items, "effects").map(parse_text_effects),
        }),
        _ => None,
    }
}
pub(crate) fn parse_kicad_footprint_filters(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(unescape_kicad_brace_string)
        .filter(|filter| !filter.is_empty())
        .collect()
}

pub(crate) fn case_insensitive_contains(value: &str, needle: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

pub(crate) fn kicad_wildcard_match(pattern: &str, value: &str) -> bool {
    wildcard_match(
        pattern.to_ascii_lowercase().as_bytes(),
        value.to_ascii_lowercase().as_bytes(),
    )
}

fn wildcard_match(pattern: &[u8], value: &[u8]) -> bool {
    let (mut pattern_index, mut value_index) = (0, 0);
    let mut star_index = None;
    let mut star_value_index = 0;

    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }

    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }

    pattern_index == pattern.len()
}

fn unescape_kicad_brace_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '{' {
            output.push(character);
            continue;
        }

        let mut token = String::new();
        let mut terminated = false;
        for token_character in characters.by_ref() {
            if token_character == '}' {
                terminated = true;
                break;
            }
            token.push(token_character);
        }

        if terminated {
            match token.as_str() {
                "dblquote" => output.push('"'),
                "quote" => output.push('\''),
                "lt" => output.push('<'),
                "gt" => output.push('>'),
                "backslash" => output.push('\\'),
                "slash" => output.push('/'),
                "bar" => output.push('|'),
                "comma" => output.push(','),
                "colon" => output.push(':'),
                "space" => output.push(' '),
                "dollar" => output.push('$'),
                "tab" => output.push('\t'),
                "return" => output.push('\n'),
                "brace" => output.push('{'),
                _ => {
                    output.push('{');
                    output.push_str(&unescape_kicad_brace_string(&token));
                    output.push('}');
                }
            }
        } else {
            output.push('{');
            output.push_str(&unescape_kicad_brace_string(&token));
        }
    }
    output
}

fn parse_data_chunks(node: &Sexp) -> String {
    list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .collect::<String>()
}

fn parse_number_list(node: &Sexp) -> Vec<f64> {
    list_items(node)
        .iter()
        .skip(1)
        .filter_map(atom_text)
        .filter_map(|value| value.parse().ok())
        .collect()
}
fn write_points_sexpr(output: &mut String, points: &[KicadPoint]) {
    let points = points
        .iter()
        .map(|point| format!("(xy {} {})", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!(" (pts {})", points));
}

fn write_base64_data_sexpr(output: &mut String, data: &str, indent: usize) {
    let pad = " ".repeat(indent);
    output.push_str(&format!("{}(data", pad));
    let mut wrote_chunk = false;
    for chunk in data.as_bytes().chunks(76) {
        wrote_chunk = true;
        output.push_str(&format!(
            "\n{}  {}",
            pad,
            sexpr_string(std::str::from_utf8(chunk).unwrap_or_default())
        ));
    }
    if wrote_chunk {
        output.push('\n');
        output.push_str(&pad);
    }
    output.push_str(")\n");
}
fn png_size_from_base64(data: &str) -> Option<(u32, u32)> {
    let header = decode_base64_prefix(data, 24)?;
    if header.len() < 24 || &header[0..8] != b"\x89PNG\r\n\x1a\n" || &header[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    (width > 0 && height > 0).then_some((width, height))
}

fn base64_starts_with(data: &str, prefix: &[u8]) -> bool {
    decode_base64_prefix(data, prefix.len())
        .map(|decoded| decoded.starts_with(prefix))
        .unwrap_or(false)
}

fn decode_base64_prefix(data: &str, wanted_len: usize) -> Option<Vec<u8>> {
    let mut decoded = Vec::with_capacity(wanted_len);
    let mut buffer = [0_u8; 4];
    let mut buffer_len = 0;

    for byte in data.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => 64,
            _ => return None,
        };
        buffer[buffer_len] = value;
        buffer_len += 1;

        if buffer_len == 4 {
            decoded.push((buffer[0] << 2) | (buffer[1] >> 4));
            if buffer[2] != 64 {
                decoded.push((buffer[1] << 4) | (buffer[2] >> 2));
            }
            if buffer[3] != 64 {
                decoded.push((buffer[2] << 6) | buffer[3]);
            }
            if decoded.len() >= wanted_len {
                decoded.truncate(wanted_len);
                return Some(decoded);
            }
            if buffer[2] == 64 || buffer[3] == 64 {
                break;
            }
            buffer_len = 0;
        }
    }

    (decoded.len() >= wanted_len).then_some(decoded)
}

pub(crate) fn kicad_bounding_box_json(bounds: KicadBoundingBox) -> String {
    format!(
        concat!(
            "{{ ",
            "\"min\": {{ \"x\": {}, \"y\": {} }}, ",
            "\"max\": {{ \"x\": {}, \"y\": {} }}, ",
            "\"width\": {}, ",
            "\"height\": {} ",
            "}}"
        ),
        bounds.min.x,
        bounds.min.y,
        bounds.max.x,
        bounds.max.y,
        bounds.width(),
        bounds.height()
    )
}

pub(crate) fn kicad_bounding_box_value(bounds: KicadBoundingBox) -> serde_json::Value {
    serde_json::json!({
        "min": {
            "x": bounds.min.x,
            "y": bounds.min.y,
        },
        "max": {
            "x": bounds.max.x,
            "y": bounds.max.y,
        },
        "width": bounds.width(),
        "height": bounds.height(),
    })
}

pub(crate) fn kicad_point_value(point: KicadPoint) -> serde_json::Value {
    serde_json::json!({
        "x": point.x,
        "y": point.y,
    })
}

pub(crate) fn kicad_points_value(points: &[KicadPoint]) -> serde_json::Value {
    serde_json::Value::Array(
        points
            .iter()
            .map(|point| kicad_point_value(*point))
            .collect(),
    )
}

pub(crate) fn kicad_size_value(size: KicadSize) -> serde_json::Value {
    serde_json::json!({
        "width": size.width,
        "height": size.height,
    })
}

pub(crate) fn kicad_margins_value(margins: KicadMargins) -> serde_json::Value {
    serde_json::json!({
        "left": margins.left,
        "top": margins.top,
        "right": margins.right,
        "bottom": margins.bottom,
    })
}

pub(crate) fn kicad_at_value(at: KicadAt) -> serde_json::Value {
    serde_json::json!({
        "x": at.x,
        "y": at.y,
        "rotation": at.rotation,
    })
}

pub(crate) fn kicad_color_value(color: KicadColor) -> serde_json::Value {
    serde_json::json!({
        "red": color.red,
        "green": color.green,
        "blue": color.blue,
        "alpha": color.alpha,
    })
}

pub(crate) fn kicad_stroke_value(stroke: &KicadStroke) -> serde_json::Value {
    serde_json::json!({
        "width": stroke.width,
        "type": stroke.stroke_type,
        "color": stroke.color.map(kicad_color_value),
    })
}

pub(crate) fn kicad_fill_value(fill: &KicadFill) -> serde_json::Value {
    serde_json::json!({
        "type": fill.fill_type,
        "color": fill.color.map(kicad_color_value),
    })
}

pub(crate) fn kicad_pin_alternate_value(alternate: &KicadPinAlternate) -> serde_json::Value {
    serde_json::json!({
        "name": alternate.name,
        "electrical_type": alternate.electrical_type,
        "shape": alternate.shape,
    })
}

pub(crate) fn kicad_text_effects_value(effects: &KicadTextEffects) -> serde_json::Value {
    serde_json::json!({
        "font_size": effects.font_size.map(kicad_size_value),
        "font_thickness": effects.font_thickness,
        "font_bold": effects.font_bold,
        "font_italic": effects.font_italic,
        "font_color": effects.font_color.map(kicad_color_value),
        "justify": effects.justify,
        "hide": effects.hide,
        "href": effects.href,
    })
}

pub(crate) fn kicad_property_value(property: &KicadProperty) -> serde_json::Value {
    serde_json::json!({
        "name": property.name,
        "value": property.value,
        "id": property.id,
        "at": property.at.map(kicad_at_value),
        "hide": property.hide,
        "show_name": property.show_name,
        "do_not_autoplace": property.do_not_autoplace,
        "effects": property.effects.as_ref().map(kicad_text_effects_value),
    })
}

pub(crate) fn kicad_pin_display_value(display: &KicadPinDisplay) -> serde_json::Value {
    serde_json::json!({
        "offset": display.offset,
        "hide": display.hide,
    })
}

pub(crate) fn resolve_kicad_uri(uri: &str, base_dir: &Path) -> PathBuf {
    let base_dir = normalize_base_dir(base_dir);
    let expanded = expand_kicad_uri(uri, &base_dir);
    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

fn normalize_base_dir(base_dir: &Path) -> PathBuf {
    if base_dir.is_absolute() {
        base_dir.to_path_buf()
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(base_dir))
            .unwrap_or_else(|_| base_dir.to_path_buf())
    }
}

fn expand_kicad_uri(uri: &str, base_dir: &Path) -> String {
    let mut expanded = String::new();
    let mut remaining = uri;

    while let Some(start) = remaining.find("${") {
        expanded.push_str(&remaining[..start]);
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find('}') else {
            expanded.push_str(&remaining[start..]);
            return expanded;
        };

        let name = &after_start[..end];
        if name == "KIPRJMOD" {
            expanded.push_str(&base_dir.display().to_string());
        } else if let Ok(value) = env::var(name) {
            expanded.push_str(&value);
        } else {
            expanded.push_str("${");
            expanded.push_str(name);
            expanded.push('}');
        }
        remaining = &after_start[end + 1..];
    }

    expanded.push_str(remaining);
    expanded
}

fn transform_symbol_point(pin_at: KicadAt, symbol_at: KicadAt, mirror: Option<&str>) -> KicadPoint {
    transform_local_point(pin_at.point(), symbol_at, mirror)
}

pub(crate) fn transform_local_point(
    local: KicadPoint,
    symbol_at: KicadAt,
    mirror: Option<&str>,
) -> KicadPoint {
    let rotated = rotate_point(mirror_point(local, mirror), symbol_at.rotation);
    KicadPoint {
        x: symbol_at.x + rotated.x,
        y: symbol_at.y + rotated.y,
    }
}

pub(crate) fn transform_local_at(
    local_at: KicadAt,
    symbol_at: KicadAt,
    mirror: Option<&str>,
) -> KicadAt {
    let point = transform_local_point(local_at.point(), symbol_at, mirror);
    KicadAt {
        x: point.x,
        y: point.y,
        rotation: normalized_rotation(
            mirror_rotation(local_at.rotation, mirror) + symbol_at.rotation,
        ),
    }
}

fn mirror_point(point: KicadPoint, mirror: Option<&str>) -> KicadPoint {
    let mut mirrored = point;
    if mirror_has_axis(mirror, "x") {
        mirrored.y = -mirrored.y;
    }
    if mirror_has_axis(mirror, "y") {
        mirrored.x = -mirrored.x;
    }
    mirrored
}

fn mirror_rotation(rotation: f64, mirror: Option<&str>) -> f64 {
    let mut mirrored = rotation;
    if mirror_has_axis(mirror, "x") {
        mirrored = -mirrored;
    }
    if mirror_has_axis(mirror, "y") {
        mirrored = 180.0 - mirrored;
    }
    normalized_rotation(mirrored)
}

fn mirror_has_axis(mirror: Option<&str>, axis: &str) -> bool {
    mirror
        .into_iter()
        .flat_map(str::split_whitespace)
        .any(|candidate| candidate == axis)
}

pub fn normalize_symbol_mirror(value: &str) -> OslResult<Option<String>> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("normal") {
        return Ok(None);
    }
    let axes = mirror_axes(trimmed)?;
    symbol_mirror_from_axes(axes).map(Some).ok_or_else(|| {
        OslError::InvalidInput("KiCad symbol mirror must be x, y, xy, or none".to_string())
    })
}

fn mirror_axes(value: &str) -> OslResult<BTreeSet<&str>> {
    let mut axes = BTreeSet::new();
    if value.contains(char::is_whitespace) {
        for axis in value.split_whitespace() {
            insert_mirror_axis(&mut axes, axis)?;
        }
    } else {
        for axis in value.split("") {
            if !axis.is_empty() {
                insert_mirror_axis(&mut axes, axis)?;
            }
        }
    }
    Ok(axes)
}

fn insert_mirror_axis<'a>(axes: &mut BTreeSet<&'a str>, axis: &'a str) -> OslResult<()> {
    match axis {
        "x" | "y" => {
            axes.insert(axis);
            Ok(())
        }
        _ => Err(OslError::InvalidInput(format!(
            "unsupported KiCad symbol mirror axis '{axis}'"
        ))),
    }
}

fn symbol_mirror_from_axes(axes: BTreeSet<&str>) -> Option<String> {
    let mirror = axes.into_iter().collect::<Vec<_>>().join(" ");
    (!mirror.is_empty()).then_some(mirror)
}
pub(crate) fn canvas_symbol_bounds(
    graphics: &[KicadCanvasGraphic],
    pins: &[KicadCanvasPin],
) -> Option<KicadBoundingBox> {
    let mut bounds = KicadBoundingBoxBuilder::default();
    for graphic in graphics {
        if let Some(graphic_bounds) = graphic.bounds() {
            bounds.include_box(graphic_bounds);
        } else {
            graphic.include_in_bounds(&mut bounds);
        }
    }
    for pin in pins {
        if let Some(pin_bounds) =
            kicad_points_bounds(&[pin.start, pin.end], KICAD_CANVAS_LINE_BOUNDS_PADDING)
        {
            bounds.include_box(pin_bounds);
        }
    }
    bounds.finish()
}
fn normalized_rotation(rotation: f64) -> f64 {
    let normalized = rotation % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}

fn compare_pin_numbers(left: &&KicadPinDef, right: &&KicadPinDef) -> Ordering {
    match (left.number().parse::<u32>(), right.number().parse::<u32>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.number().cmp(right.number()),
    }
}

fn insert_point(points: &mut BTreeMap<PointKey, KicadPoint>, point: KicadPoint) {
    points.entry(PointKey::from(point)).or_insert(point);
}

fn segment_contains_point(start: KicadPoint, end: KicadPoint, point: KicadPoint) -> bool {
    let cross = (point.y - start.y) * (end.x - start.x) - (point.x - start.x) * (end.y - start.y);
    if cross.abs() > 1e-6 {
        return false;
    }

    between_inclusive(point.x, start.x, end.x) && between_inclusive(point.y, start.y, end.y)
}

fn between_inclusive(value: f64, left: f64, right: f64) -> bool {
    let min = left.min(right) - 1e-6;
    let max = left.max(right) + 1e-6;
    value >= min && value <= max
}

fn coordinate_key(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn same_point(left: KicadPoint, right: KicadPoint) -> bool {
    coordinate_key(left.x) == coordinate_key(right.x)
        && coordinate_key(left.y) == coordinate_key(right.y)
}

fn same_size(left: KicadSize, right: KicadSize) -> bool {
    coordinate_key(left.width) == coordinate_key(right.width)
        && coordinate_key(left.height) == coordinate_key(right.height)
}

fn normalize_net_name(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "gnd" | "agnd" | "dgnd" | "earth" => "0".to_string(),
        _ => name.trim().to_string(),
    }
}

fn preferred_net_label(labels: Option<&BTreeSet<String>>) -> Option<String> {
    let labels = labels?;
    labels
        .iter()
        .find(|label| label.as_str() == "0")
        .cloned()
        .or_else(|| labels.iter().find(|label| !label.is_empty()).cloned())
}

fn kicad_schematic_diagnostic(
    severity: KicadDiagnosticSeverity,
    code: &str,
    message: &str,
    item: Option<String>,
    net: Option<String>,
    pin: Option<String>,
) -> KicadSchematicDiagnostic {
    KicadSchematicDiagnostic {
        severity,
        code: code.to_string(),
        message: message.to_string(),
        item,
        net,
        pin,
    }
}

fn library_symbol_definition_for_lib_id(
    library: &KicadSymbolLibrary,
    library_name: &str,
    lib_id: &str,
) -> Option<KicadSymbolDef> {
    if let Some(symbol) = library.symbol(lib_id) {
        return Some(symbol.clone());
    }

    let (requested_library, requested_name) = lib_id.split_once(':')?;
    if requested_library != library_name {
        return None;
    }

    library
        .symbols
        .iter()
        .find(|symbol| symbol.name == requested_name || symbol.local_name() == requested_name)
        .cloned()
        .map(|mut symbol| {
            qualify_library_symbol_name(&mut symbol, library_name);
            symbol
        })
}

fn qualify_library_symbol_name(symbol: &mut KicadSymbolDef, library_name: &str) {
    if !symbol.name.contains(':') {
        symbol.name = format!("{library_name}:{}", symbol.name);
    }
}

fn spice_primitive_for_device(device: &str) -> Option<String> {
    let device = device.to_ascii_uppercase();
    let primitive = match device.as_str() {
        "R" | "RES" | "RESISTOR" => "R",
        "C" | "CAP" | "CAPACITOR" => "C",
        "L" | "IND" | "INDUCTOR" => "L",
        "V" | "VSOURCE" | "VOLTAGE" => "V",
        "I" | "ISOURCE" | "CURRENT" => "I",
        "D" | "DIODE" => "D",
        "NPN" | "PNP" | "BJT" => "Q",
        "NJFET" | "PJFET" | "JFET" => "J",
        "NMOS" | "PMOS" | "NMES" | "PMES" | "MOSFET" => "M",
        "SW" | "SWITCH" => "S",
        "CSW" | "CURRENT_SWITCH" => "W",
        "VCVS" => "E",
        "CCCS" => "F",
        "VCCS" => "G",
        "CCVS" => "H",
        "TLINE" | "TRANSMISSION_LINE" => "T",
        "K" | "COUPLED_INDUCTOR" => "K",
        "SUBCKT" => "X",
        "SPICE" => "SPICE",
        "" => return None,
        other if other.len() == 1 => other,
        _ => return None,
    };
    Some(primitive.to_string())
}

fn symbol_ordered_pins<'a>(
    symbol: &'a KicadSymbolInstance,
    definition: &'a KicadResolvedSymbolDef,
) -> Vec<&'a KicadPinDef> {
    let scoped_pins = definition
        .scoped_pins(symbol.unit, symbol.body_style)
        .collect::<Vec<_>>();
    let mut by_number = scoped_pins
        .iter()
        .copied()
        .map(|pin| (pin.number(), pin))
        .collect::<BTreeMap<_, _>>();
    let by_name = scoped_pins
        .iter()
        .copied()
        .map(|pin| (pin.name(), pin))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = Vec::new();

    for pin_number in symbol_sim_pin_order(symbol, definition) {
        if let Some(pin) = by_number.remove(pin_number.as_str()) {
            ordered.push(pin);
        } else if let Some(pin) = by_name.get(pin_number.as_str()) {
            ordered.push(*pin);
        }
    }

    if ordered.is_empty() {
        ordered = scoped_pins;
        ordered.sort_by(compare_pin_numbers);
    }

    ordered
}

fn scoped_symbol_pins<'a>(
    definition: &'a KicadSymbolDef,
    unit: Option<u32>,
    body_style: Option<u32>,
) -> impl Iterator<Item = &'a KicadPinDef> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    definition
        .pins
        .iter()
        .filter(move |pin| symbol_item_scope_matches(pin.unit, pin.body_style, unit, body_style))
}

fn scoped_definition_graphics<'a>(
    definition: &'a KicadSymbolDef,
    unit: Option<u32>,
    body_style: Option<u32>,
) -> impl Iterator<Item = &'a KicadSymbolGraphic> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    definition.graphics.iter().filter(move |graphic| {
        symbol_item_scope_matches(graphic.unit, graphic.body_style, unit, body_style)
    })
}

fn scoped_symbol_items<'a, T>(
    items: &'a [T],
    unit: Option<u32>,
    body_style: Option<u32>,
    scope: impl Fn(&T) -> (u32, u32) + 'a,
) -> impl Iterator<Item = &'a T> + 'a {
    let unit = unit.unwrap_or(1);
    let body_style = body_style.unwrap_or(1);
    items.iter().filter(move |item| {
        let (item_unit, item_body_style) = scope(item);
        symbol_item_scope_matches(item_unit, item_body_style, unit, body_style)
    })
}

fn symbol_item_scope_matches(
    item_unit: u32,
    item_body_style: u32,
    selected_unit: u32,
    selected_body_style: u32,
) -> bool {
    (item_unit == 0 || item_unit == selected_unit)
        && (item_body_style == 0 || item_body_style == selected_body_style)
}

fn symbol_sim_pin_order(
    symbol: &KicadSymbolInstance,
    definition: &KicadResolvedSymbolDef,
) -> Vec<String> {
    let Some(pins) = symbol.sim_pins(Some(definition)) else {
        return Vec::new();
    };
    parse_sim_pin_order(pins)
}

fn parse_sim_pin_order(value: &str) -> Vec<String> {
    value
        .split(|character: char| character.is_ascii_whitespace() || character == ',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .filter_map(|token| {
            let symbol_pin = token.split_once('=').map(|(left, _)| left).unwrap_or(token);
            let symbol_pin = symbol_pin.trim();
            (!symbol_pin.is_empty()).then(|| symbol_pin.to_string())
        })
        .collect()
}

fn parse_kicad_bool_value(value: String) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

fn parse_optional_bool_child(items: &[Sexp], name: &str) -> Option<bool> {
    child(items, name).map(|node| {
        list_value(node, 1)
            .and_then(parse_kicad_bool_value)
            .unwrap_or(true)
    })
}

fn parse_kicad_enable_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" | "true" | "1" | "on" => Some(true),
        "n" | "no" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn compose_spice_model_value(
    model: Option<&str>,
    params: Option<&str>,
    fallback: Option<&str>,
) -> String {
    match (
        model.filter(|value| !value.is_empty()),
        params.filter(|value| !value.is_empty()),
    ) {
        (Some(model), Some(params)) => format!("{model} {params}"),
        (Some(model), None) => model.to_string(),
        (None, Some(params)) => params.to_string(),
        (None, None) => fallback.unwrap_or_default().to_string(),
    }
}

fn strip_kicad_sim_model_params(value: &str) -> String {
    split_spice_tokens(value)
        .into_iter()
        .filter(|token| {
            token
                .split_once('=')
                .map(|(name, _)| {
                    !matches!(name.trim().to_ascii_lowercase().as_str(), "model" | "lib")
                })
                .unwrap_or(true)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_named_sim_param(value: &str, name: &str) -> Option<String> {
    for token in split_spice_tokens(value) {
        let Some((left, right)) = token.split_once('=') else {
            continue;
        };
        if left.trim().eq_ignore_ascii_case(name) {
            return Some(unquote_spice_token(right.trim()).to_string());
        }
    }
    None
}

fn split_spice_tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escaped = false;

    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' {
            current.push(character);
            escaped = true;
            continue;
        }
        if character == '"' {
            current.push(character);
            in_quotes = !in_quotes;
            continue;
        }
        if character.is_ascii_whitespace() && !in_quotes {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(character);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn spice_item_name(reference: &str, primitive: &str) -> String {
    let Some(first) = primitive.chars().next() else {
        return reference.to_string();
    };
    if reference
        .chars()
        .next()
        .is_some_and(|character| character.eq_ignore_ascii_case(&first))
    {
        reference.to_string()
    } else {
        format!("{first}{reference}")
    }
}

fn unquote_spice_token(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

fn expand_spice_template(template: &str, reference: &str, nodes: &[String]) -> String {
    let mut expanded = template.replace("${REFERENCE}", reference);
    for (index, node) in nodes.iter().enumerate() {
        expanded = expanded.replace(&format!("${{N{}}}", index + 1), node);
    }
    expanded
}

fn quote_spice_path(path: &str) -> String {
    if path
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte == b'"')
    {
        format!("\"{}\"", path.replace('"', "\\\""))
    } else {
        format!("\"{}\"", path)
    }
}

fn symbol_instance_properties(
    definition: &KicadSymbolDef,
    reference: &str,
    value: &str,
    symbol_at: KicadAt,
) -> Vec<KicadProperty> {
    let mut properties = definition
        .properties
        .iter()
        .map(|property| KicadProperty {
            name: property.name.clone(),
            value: match property.name.as_str() {
                "Reference" => reference.to_string(),
                "Value" => value.to_string(),
                _ => property.value.clone(),
            },
            id: property.id,
            at: property
                .at
                .map(|property_at| transform_local_at(property_at, symbol_at, None)),
            hide: property.hide,
            show_name: property.show_name,
            do_not_autoplace: property.do_not_autoplace,
            effects: property.effects.clone(),
        })
        .collect::<Vec<_>>();

    if !properties
        .iter()
        .any(|property| property.name == "Reference")
    {
        properties.push(KicadProperty {
            name: "Reference".to_string(),
            value: reference.to_string(),
            id: None,
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y - 2.54,
                rotation: symbol_at.rotation,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        });
    }
    if !properties.iter().any(|property| property.name == "Value") {
        properties.push(KicadProperty {
            name: "Value".to_string(),
            value: value.to_string(),
            id: None,
            at: Some(KicadAt {
                x: symbol_at.x,
                y: symbol_at.y + 2.54,
                rotation: symbol_at.rotation,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        });
    }

    properties
}

fn sheet_properties(name: &str, file: &str, at: KicadAt, size: KicadSize) -> Vec<KicadProperty> {
    vec![
        KicadProperty {
            name: "Sheetname".to_string(),
            value: name.to_string(),
            id: None,
            at: Some(KicadAt {
                x: at.x,
                y: at.y - 1.27,
                rotation: 0.0,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        },
        KicadProperty {
            name: "Sheetfile".to_string(),
            value: file.to_string(),
            id: None,
            at: Some(KicadAt {
                x: at.x,
                y: at.y + size.height + 1.27,
                rotation: 0.0,
            }),
            hide: None,
            show_name: None,
            do_not_autoplace: None,
            effects: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        KicadAt, KicadBoundingBox, KicadCanvasScene, KicadColor, KicadDiagnosticSeverity,
        KicadGraphic, KicadIndexedSymbolBodyStyle, KicadIndexedSymbolUnit, KicadLabelKind,
        KicadPoint, KicadSchematicEdit, KicadSheetPin, KicadSize, KicadSymbolBodyStyles,
        KicadSymbolLibraryIndexQuery, KicadSymbolPlacement, KicadSymbolPower, parse_kicad_project,
        parse_kicad_schematic, parse_kicad_symbol_library, parse_kicad_symbol_library_table,
        parse_sexpr, read_kicad_project, read_kicad_schematic, read_kicad_schematic_with_libraries,
        read_kicad_symbol_library, read_kicad_symbol_library_index,
        read_kicad_symbol_library_table,
    };
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    #[test]
    fn bounding_boxes_report_intersections() {
        let bounds = KicadBoundingBox {
            min: KicadPoint { x: 10.0, y: 20.0 },
            max: KicadPoint { x: 30.0, y: 40.0 },
        };
        assert!(bounds.contains(KicadPoint { x: 20.0, y: 30.0 }));
        assert!(bounds.intersects(KicadBoundingBox {
            min: KicadPoint { x: 25.0, y: 35.0 },
            max: KicadPoint { x: 45.0, y: 55.0 },
        }));
        assert!(!bounds.intersects(KicadBoundingBox {
            min: KicadPoint { x: 31.0, y: 41.0 },
            max: KicadPoint { x: 45.0, y: 55.0 },
        }));
    }

    #[test]
    fn parses_kicad_schematic_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        assert_eq!(schematic.version.as_deref(), Some("20230121"));
        assert_eq!(schematic.paper.as_deref(), Some("A4"));
        assert_eq!(schematic.symbols.len(), 3);
        assert_eq!(schematic.library_symbols.len(), 3);
        assert_eq!(
            schematic
                .library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        assert_eq!(schematic.wires.len(), 3);
        assert_eq!(schematic.text_items.len(), 1);
        assert_eq!(
            schematic.wires[0].uuid.as_deref(),
            Some("22222222-2222-2222-2222-222222222222")
        );
        assert_eq!(schematic.labels.len(), 3);
        assert_eq!(
            schematic.labels[1].uuid.as_deref(),
            Some("66666666-6666-6666-6666-666666666666")
        );
        assert_eq!(schematic.spice_directives()[0].text, ".tran 1u 1m");
        assert_eq!(
            schematic.spice_directives()[0].uuid.as_deref(),
            Some("77777777-7777-7777-7777-777777777777")
        );
        assert_eq!(schematic.symbols[0].reference(), Some("V1"));
        assert_eq!(schematic.symbols[0].pins[0].number.as_deref(), Some("1"));
        assert_eq!(
            schematic.symbols[0].pins[0].uuid.as_deref(),
            Some("99999999-9999-9999-9999-999999999991")
        );
        assert_eq!(schematic.symbols[1].value(), Some("1k"));
        assert!(
            schematic
                .labels
                .iter()
                .any(|label| label.text == "out" && label.kind == KicadLabelKind::Local)
        );
        assert!(schematic.to_summary_json().contains("\"symbol_count\": 3"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"library_graphic_count\": 6")
        );
    }

    #[test]
    fn builds_connectivity_and_exports_spice() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let graph = schematic.connectivity_graph();
        assert_eq!(
            graph
                .nets
                .iter()
                .map(|net| net.name.as_str())
                .collect::<Vec<_>>(),
            ["0", "in", "out"]
        );

        let netlist = schematic.to_spice_netlist().unwrap();
        assert!(netlist.contains("V1 in 0 PULSE(0 1 0 1u 1u 10u 20u)"));
        assert!(netlist.contains("R1 in out 1k"));
        assert!(netlist.contains("C1 out 0 100n"));
        assert!(netlist.contains(".tran 1u 1m"));
        assert!(netlist.ends_with(".end\n"));
    }

    #[test]
    fn checks_kicad_schematic_fixture_without_errors() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let report = schematic.check_report();

        assert_eq!(report.error_count(), 0);
        assert_eq!(report.symbol_count, 3);
        assert!(report.net_count >= 3);
        assert!(report.to_json().contains("\"error_count\": 0"));
    }

    #[test]
    fn checks_kicad_schematic_structural_diagnostics() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (wire (pts (xy 10 10) (xy 20 10)))
  (label "floating" (at 40 40 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "" (at 12.54 12 0))
  )
  (symbol
    (lib_id "Missing:X")
    (at 30 30 0)
    (property "Reference" "R1" (at 30 28 0))
    (property "Value" "model" (at 30 32 0))
  )
)"#,
            "bad.kicad_sch",
        )
        .unwrap();

        let report = schematic.check_report();
        let codes = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>();

        assert!(report.error_count() >= 3);
        assert!(codes.contains(&"duplicate-reference"));
        assert!(codes.contains(&"missing-symbol-definition"));
        assert!(codes.contains(&"missing-ground"));
        assert!(codes.contains(&"missing-value"));
        assert!(codes.contains(&"missing-spice-directive"));
        assert!(report.to_json().contains("\"diagnostic_count\""));
    }

    #[test]
    fn honors_no_connect_markers_on_unconnected_symbol_pins() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (label "0" (at 15.08 10 0))
  (no_connect (at 10 10) (uuid "12121212-1212-1212-1212-121212121212"))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
    (pin "1" (uuid "abababab-0000-0000-0000-000000000001"))
    (pin "2" (uuid "abababab-0000-0000-0000-000000000002"))
  )
)"#,
            "no_connect.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.no_connects.len(), 1);
        assert_eq!(
            schematic.no_connects[0].uuid.as_deref(),
            Some("12121212-1212-1212-1212-121212121212")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"no_connect_count\": 1")
        );

        let report = schematic.check_report();
        assert_eq!(report.error_count(), 0);
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.code.as_str(),
                "unconnected-pin" | "generated-net-name" | "floating-no-connect"
            )
        }));

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(no_connect"));
        assert!(roundtrip.contains("(uuid \"12121212-1212-1212-1212-121212121212\")"));
        let reparsed = parse_kicad_schematic(&roundtrip, "roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.no_connects.len(), 1);
        assert_eq!(reparsed.canvas_scene().no_connects.len(), 1);
    }

    #[test]
    fn parses_schematic_junction_styles_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (junction
    (at 58.42 19.05)
    (diameter 0.8128)
    (color 255 0 239 1)
    (uuid "8fabedd0-c306-4e64-a286-1d33eb9a2adf")
  )
)"#,
            "junction.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.junctions.len(), 1);
        assert_close(schematic.junctions[0].diameter.unwrap(), 0.8128);
        assert_eq!(
            schematic.junctions[0].color,
            Some(KicadColor {
                red: 255.0,
                green: 0.0,
                blue: 239.0,
                alpha: 1.0,
            })
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_junction_count\": 1")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.junctions.len(), 1);
        assert_close(scene.junctions[0].diameter.unwrap(), 0.8128);
        assert_eq!(
            scene.junctions[0].color,
            Some(KicadColor {
                red: 255.0,
                green: 0.0,
                blue: 239.0,
                alpha: 1.0,
            })
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(junction"));
        assert!(roundtrip.contains("(diameter 0.8128)"));
        assert!(roundtrip.contains("(color 255 0 239 1)"));
        let reparsed = parse_kicad_schematic(&roundtrip, "junction_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.junctions.len(), 1);
        assert_close(reparsed.junctions[0].diameter.unwrap(), 0.8128);
        assert_eq!(
            reparsed.junctions[0].uuid.as_deref(),
            Some("8fabedd0-c306-4e64-a286-1d33eb9a2adf")
        );
    }

    #[test]
    fn parses_kicad_bus_items_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (bus_alias "DATA" (members "D0" "D1" "D2" "D3"))
  (bus_entry
    (at 30 10)
    (size 2.54 -2.54)
    (stroke (width 0.127) (type dot) (color 255 89 101 1))
    (uuid "31313131-3131-4131-8131-313131313131")
  )
  (bus
    (pts (xy 30 10) (xy 30 30) (xy 60 30))
    (stroke (width 0.254) (type dash) (color 58 104 255 1))
    (uuid "32323232-3232-4232-8232-323232323232")
  )
  (wire
    (pts (xy 60 30) (xy 70 30))
    (stroke (width 0.1778) (type dash_dot) (color 255 176 0 1))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
            "bus.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.bus_aliases.len(), 1);
        assert_eq!(schematic.bus_aliases[0].name, "DATA");
        assert_eq!(
            schematic.bus_aliases[0].members,
            vec![
                "D0".to_string(),
                "D1".to_string(),
                "D2".to_string(),
                "D3".to_string()
            ]
        );
        assert_eq!(schematic.buses.len(), 1);
        assert_eq!(schematic.bus_entries.len(), 1);
        assert_eq!(schematic.wires.len(), 1);
        assert_eq!(
            schematic.bus_entries[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dot")
        );
        assert_close(
            schematic.buses[0].stroke.as_ref().unwrap().width.unwrap(),
            0.254,
        );
        assert_eq!(
            schematic.wires[0].stroke.as_ref().unwrap().color,
            Some(KicadColor {
                red: 255.0,
                green: 176.0,
                blue: 0.0,
                alpha: 1.0,
            })
        );
        assert_close(schematic.bus_entries[0].end().x, 32.54);
        assert_close(schematic.bus_entries[0].end().y, 7.46);
        assert!(
            schematic
                .to_summary_json()
                .contains("\"bus_alias_count\": 1")
        );
        assert!(schematic.to_summary_json().contains("\"bus_count\": 1"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"bus_entry_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_wire_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_bus_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_bus_entry_count\": 1")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.wires.len(), 1);
        assert_eq!(scene.buses.len(), 1);
        assert_eq!(scene.bus_entries.len(), 1);
        assert_eq!(
            scene.wires[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash_dot")
        );
        assert!(scene.to_summary_json().contains("\"bus_count\": 1"));
        assert!(scene.to_summary_json().contains("\"bus_entry_count\": 1"));

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(bus_alias \"DATA\" (members \"D0\" \"D1\" \"D2\" \"D3\"))"));
        assert!(roundtrip.contains("(bus"));
        assert!(roundtrip.contains("(bus_entry"));
        assert!(roundtrip.contains("(stroke (width 0.127) (type dot) (color 255 89 101 1))"));
        assert!(roundtrip.contains("(stroke (width 0.254) (type dash) (color 58 104 255 1))"));
        assert!(roundtrip.contains("(stroke (width 0.1778) (type dash_dot) (color 255 176 0 1))"));
        assert!(roundtrip.contains("(uuid \"31313131-3131-4131-8131-313131313131\")"));
        assert!(roundtrip.contains("(uuid \"32323232-3232-4232-8232-323232323232\")"));
        let reparsed = parse_kicad_schematic(&roundtrip, "bus_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.bus_aliases.len(), 1);
        assert_eq!(reparsed.buses.len(), 1);
        assert_eq!(reparsed.bus_entries.len(), 1);
        assert_eq!(reparsed.wires.len(), 1);
        assert_eq!(
            reparsed.buses[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash")
        );
        assert_eq!(
            reparsed.bus_entries[0].uuid.as_deref(),
            Some("31313131-3131-4131-8131-313131313131")
        );
        assert_eq!(
            reparsed.buses[0].uuid.as_deref(),
            Some("32323232-3232-4232-8232-323232323232")
        );
    }

    #[test]
    fn parses_net_chains_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (net_chain "Signal1"
    (from "U1" "A1")
    (to "J1" "2")
    (net_class "USB3")
    (color 58 104 255 0.75)
    (nets "SS_TX+" "SS_TX-")
    (uuid "605e5401-cbcc-4f20-9148-b7b3bd8eecbe")
    (uuid "a878e86a-9b21-4559-9e74-a7a0e383034e")
  )
)"#,
            "net_chain.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.net_chains.len(), 1);
        let net_chain = &schematic.net_chains[0];
        assert_eq!(net_chain.name, "Signal1");
        assert_eq!(net_chain.from.as_ref().unwrap().reference, "U1");
        assert_eq!(net_chain.from.as_ref().unwrap().pin, "A1");
        assert_eq!(net_chain.to.as_ref().unwrap().reference, "J1");
        assert_eq!(net_chain.to.as_ref().unwrap().pin, "2");
        assert_eq!(net_chain.net_class.as_deref(), Some("USB3"));
        assert_eq!(
            net_chain.color,
            Some(KicadColor {
                red: 58.0,
                green: 104.0,
                blue: 255.0,
                alpha: 0.75,
            })
        );
        assert_eq!(
            net_chain.member_nets,
            vec!["SS_TX+".to_string(), "SS_TX-".to_string()]
        );
        assert_eq!(net_chain.extra.len(), 2);
        assert!(
            schematic
                .to_summary_json()
                .contains("\"net_chain_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"net_chain_member_net_count\": 2")
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(net_chain \"Signal1\""));
        assert!(roundtrip.contains("(from \"U1\" \"A1\")"));
        assert!(roundtrip.contains("(to \"J1\" \"2\")"));
        assert!(roundtrip.contains("(net_class \"USB3\")"));
        assert!(roundtrip.contains("(color 58 104 255 0.75)"));
        assert!(roundtrip.contains("(nets \"SS_TX+\" \"SS_TX-\")"));
        assert!(roundtrip.contains("(uuid \"605e5401-cbcc-4f20-9148-b7b3bd8eecbe\")"));
        assert!(roundtrip.contains("(uuid \"a878e86a-9b21-4559-9e74-a7a0e383034e\")"));

        let reparsed = parse_kicad_schematic(&roundtrip, "net_chain_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.net_chains.len(), 1);
        assert_eq!(reparsed.net_chains[0].member_nets.len(), 2);
        assert_eq!(reparsed.net_chains[0].extra.len(), 2);
        assert_eq!(reparsed.net_chains[0].net_class.as_deref(), Some("USB3"));
    }

    #[test]
    fn parses_schematic_graphics_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (polyline
    (pts (xy 10 10) (xy 20 10) (xy 20 15))
    (stroke (width 0.3556) (type dot) (color 255 89 101 1))
    (uuid "41414141-4141-4141-8141-414141414141")
  )
  (bezier
    (pts (xy 12 16) (xy 16 8) (xy 24 8) (xy 28 16))
    (stroke (width 0.2032) (type dash) (color 58 104 255 1))
    (fill (type none))
    (uuid "45454545-4545-4545-8545-454545454545")
  )
  (rectangle
    (start 30 10)
    (end 45 20)
    (stroke (width 0) (type default))
    (fill (type hatch) (color 255 64 87 1))
    (uuid "42424242-4242-4242-8242-424242424242")
    (locked yes)
  )
  (circle
    (center 60 15)
    (radius 5)
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "43434343-4343-4343-8343-434343434343")
  )
  (arc
    (start 70 20)
    (mid 75 10)
    (end 80 20)
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "44444444-4444-4444-8444-444444444444")
  )
)"#,
            "graphics.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.graphics.len(), 5);
        assert!(matches!(
            &schematic.graphics[0].graphic,
            KicadGraphic::Polyline { .. }
        ));
        assert!(matches!(
            &schematic.graphics[1].graphic,
            KicadGraphic::Bezier { .. }
        ));
        assert!(matches!(
            &schematic.graphics[2].graphic,
            KicadGraphic::Rectangle { .. }
        ));
        assert!(matches!(
            &schematic.graphics[3].graphic,
            KicadGraphic::Circle { .. }
        ));
        assert!(matches!(
            &schematic.graphics[4].graphic,
            KicadGraphic::Arc { .. }
        ));
        assert_eq!(
            schematic.graphics[0].uuid.as_deref(),
            Some("41414141-4141-4141-8141-414141414141")
        );
        assert_close(
            schematic.graphics[0]
                .stroke
                .as_ref()
                .unwrap()
                .width
                .unwrap(),
            0.3556,
        );
        assert_eq!(
            schematic.graphics[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dot")
        );
        assert_eq!(
            schematic.graphics[0].stroke.as_ref().unwrap().color,
            Some(KicadColor {
                red: 255.0,
                green: 89.0,
                blue: 101.0,
                alpha: 1.0,
            })
        );
        if let KicadGraphic::Bezier { points } = &schematic.graphics[1].graphic {
            assert_eq!(points.len(), 4);
            assert_close(points[1].x, 16.0);
            assert_close(points[2].y, 8.0);
        } else {
            panic!("expected bezier schematic graphic");
        }
        assert_eq!(
            schematic.graphics[1]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash")
        );
        assert_eq!(
            schematic.graphics[2]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("hatch")
        );
        assert_eq!(schematic.graphics[2].locked, Some(true));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"schematic_graphic_count\": 5")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_schematic_graphic_count\": 5")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"locked_schematic_graphic_count\": 1")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.graphics.len(), 5);
        assert!(matches!(
            &scene.graphics[1],
            super::KicadCanvasGraphic::Bezier {
                points,
                stroke: Some(stroke),
                ..
            } if points.len() == 4 && stroke.stroke_type.as_deref() == Some("dash")
        ));
        assert!(matches!(
            &scene.graphics[2],
            super::KicadCanvasGraphic::Rectangle {
                fill: Some(fill),
                ..
            } if fill.fill_type.as_deref() == Some("hatch")
        ));
        assert!(scene.to_summary_json().contains("\"graphic_count\": 5"));
        assert!(
            scene
                .to_summary_json()
                .contains("\"schematic_graphic_count\": 5")
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(polyline"));
        assert!(roundtrip.contains("(stroke (width 0.3556) (type dot) (color 255 89 101 1))"));
        assert!(roundtrip.contains("(bezier"));
        assert!(roundtrip.contains("(pts (xy 12 16) (xy 16 8) (xy 24 8) (xy 28 16))"));
        assert!(roundtrip.contains("(stroke (width 0.2032) (type dash) (color 58 104 255 1))"));
        assert!(roundtrip.contains("(rectangle"));
        assert!(roundtrip.contains("(fill (type hatch) (color 255 64 87 1))"));
        assert!(roundtrip.contains("(locked yes)"));
        assert!(roundtrip.contains("(circle"));
        assert!(roundtrip.contains("(arc"));
        assert!(roundtrip.contains("(uuid \"44444444-4444-4444-8444-444444444444\")"));
        let reparsed = parse_kicad_schematic(&roundtrip, "graphics_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.graphics.len(), 5);
        assert_eq!(
            reparsed.graphics[1].uuid.as_deref(),
            Some("45454545-4545-4545-8545-454545454545")
        );
        assert!(matches!(
            &reparsed.graphics[1].graphic,
            KicadGraphic::Bezier { points } if points.len() == 4
        ));
        assert_eq!(
            reparsed.graphics[4].uuid.as_deref(),
            Some("44444444-4444-4444-8444-444444444444")
        );
        assert_eq!(reparsed.graphics[2].locked, Some(true));
        assert_eq!(
            reparsed.graphics[2]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("hatch")
        );
        assert_eq!(reparsed.canvas_scene().graphics.len(), 5);
    }

    #[test]
    fn parses_rule_areas_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (rule_area
    (locked yes)
    (exclude_from_sim no)
    (in_bom no)
    (on_board no)
    (dnp yes)
    (polyline
      (pts
        (xy 120.65 30.48) (xy 100.33 30.48) (xy 100.33 53.34) (xy 104.14 57.15)
      )
      (stroke (width 0.127) (type dash) (color 10 20 30 1))
      (fill (type color) (color 20 200 170 0.25))
      (uuid "c41fc141-ff73-4a8e-9714-30fcb0d8076b")
    )
  )
)"#,
            "rule_area.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.rule_areas.len(), 1);
        let rule_area = &schematic.rule_areas[0];
        assert_eq!(rule_area.points.len(), 4);
        assert_close(rule_area.stroke.as_ref().unwrap().width.unwrap(), 0.127);
        assert_eq!(
            rule_area.stroke.as_ref().unwrap().stroke_type.as_deref(),
            Some("dash")
        );
        assert_eq!(
            rule_area.fill.as_ref().unwrap().fill_type.as_deref(),
            Some("color")
        );
        assert_eq!(rule_area.locked, Some(true));
        assert_eq!(rule_area.exclude_from_sim, Some(false));
        assert_eq!(rule_area.in_bom, Some(false));
        assert_eq!(rule_area.on_board, Some(false));
        assert_eq!(rule_area.dnp, Some(true));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"rule_area_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_rule_area_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"locked_rule_area_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"dnp_item_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"bom_excluded_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"board_excluded_count\": 1")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.rule_areas.len(), 1);
        assert_eq!(scene.rule_areas[0].points.len(), 4);
        assert!(scene.to_summary_json().contains("\"rule_area_count\": 1"));
        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(scene_json["rule_area_count"], 1);
        assert_eq!(scene_json["rule_areas"][0]["points"][0]["x"], 120.65);
        assert_eq!(scene_json["rule_areas"][0]["stroke"]["type"], "dash");
        assert_eq!(scene_json["rule_areas"][0]["fill"]["color"]["alpha"], 0.25);

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(rule_area"));
        assert!(roundtrip.contains("(locked yes)"));
        assert!(roundtrip.contains("(exclude_from_sim no)"));
        assert!(roundtrip.contains("(in_bom no)"));
        assert!(roundtrip.contains("(on_board no)"));
        assert!(roundtrip.contains("(dnp yes)"));
        assert!(roundtrip.contains("(stroke (width 0.127) (type dash) (color 10 20 30 1))"));
        assert!(roundtrip.contains("(fill (type color) (color 20 200 170 0.25))"));
        assert!(roundtrip.contains("(uuid \"c41fc141-ff73-4a8e-9714-30fcb0d8076b\")"));
        let reparsed = parse_kicad_schematic(&roundtrip, "rule_area_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.rule_areas.len(), 1);
        assert_eq!(
            reparsed.rule_areas[0].uuid.as_deref(),
            Some("c41fc141-ff73-4a8e-9714-30fcb0d8076b")
        );
        assert_eq!(
            reparsed.rule_areas[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash")
        );
    }

    #[test]
    fn parses_schematic_text_boxes_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (text_box "Bigger\nMultiline\nText"
    (exclude_from_sim no)
    (at 10 20 0)
    (size 17.78 12.7)
    (margins 0.9525 0.9525 0.9525 0.9525)
    (stroke (width 0.0508) (type dash_dot) (color 255 50 55 1))
    (fill (type color) (color 255 228 206 0.7490196078))
    (effects (font (size 1.27 1.27) (color 10 9 37 1)))
    (uuid "45454545-4545-4545-8545-454545454545")
    (locked)
  )
)"#,
            "text_box.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.text_boxes.len(), 1);
        assert_eq!(schematic.text_boxes[0].text, "Bigger\nMultiline\nText");
        assert_eq!(schematic.text_boxes[0].exclude_from_sim, Some(false));
        assert_close(schematic.text_boxes[0].size.unwrap().width, 17.78);
        assert_close(schematic.text_boxes[0].margins.unwrap().left, 0.9525);
        assert_close(
            schematic.text_boxes[0]
                .stroke
                .as_ref()
                .unwrap()
                .width
                .unwrap(),
            0.0508,
        );
        assert_eq!(
            schematic.text_boxes[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash_dot")
        );
        assert_eq!(
            schematic.text_boxes[0].stroke.as_ref().unwrap().color,
            Some(KicadColor {
                red: 255.0,
                green: 50.0,
                blue: 55.0,
                alpha: 1.0,
            })
        );
        assert_eq!(
            schematic.text_boxes[0]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("color")
        );
        assert_eq!(schematic.text_boxes[0].locked, Some(true));
        assert_eq!(
            schematic.text_boxes[0].uuid.as_deref(),
            Some("45454545-4545-4545-8545-454545454545")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"text_box_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_text_box_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"locked_text_box_count\": 1")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.text_boxes.len(), 1);
        assert_eq!(
            scene.text_boxes[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash_dot")
        );
        assert!(scene.bounds.unwrap().width() >= 17.78);
        assert!(scene.to_summary_json().contains("\"text_box_count\": 1"));
        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(scene_json["text_box_count"], 1);
        assert_eq!(
            scene_json["text_boxes"][0]["text"],
            "Bigger\nMultiline\nText"
        );
        assert_eq!(scene_json["text_boxes"][0]["margins"]["left"], 0.9525);
        assert_eq!(scene_json["text_boxes"][0]["stroke"]["type"], "dash_dot");
        assert_eq!(
            scene_json["text_boxes"][0]["effects"]["font_color"]["blue"],
            37.0
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(text_box \"Bigger\\nMultiline\\nText\""));
        assert!(roundtrip.contains("(size 17.78 12.7)"));
        assert!(roundtrip.contains("(margins 0.9525 0.9525 0.9525 0.9525)"));
        assert!(roundtrip.contains("(stroke (width 0.0508) (type dash_dot) (color 255 50 55 1))"));
        assert!(roundtrip.contains("(fill (type color) (color 255 228 206 0.7490196078))"));
        assert!(roundtrip.contains("(uuid \"45454545-4545-4545-8545-454545454545\")"));
        assert!(roundtrip.contains("(locked yes)"));
        let reparsed = parse_kicad_schematic(&roundtrip, "text_box_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.text_boxes.len(), 1);
        assert_eq!(reparsed.text_boxes[0].text, "Bigger\nMultiline\nText");
        assert_eq!(
            reparsed.text_boxes[0]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("color")
        );
        assert_eq!(reparsed.text_boxes[0].locked, Some(true));
        assert_eq!(reparsed.canvas_scene().text_boxes.len(), 1);
    }

    #[test]
    fn hit_tests_rotated_text_boxes_by_shape() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (text_box "Rotated note"
    (at 20 10 45)
    (size 10 4)
    (uuid "45454545-4545-4545-8545-454545454545")
  )
)"#,
            "rotated_text_box_hit.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        let text_box = &scene.text_boxes[0];
        assert!(text_box.bounds.unwrap().width() > 9.0);
        assert!(text_box.bounds.unwrap().height() > 9.0);

        let hit = scene.hit_test(KicadPoint { x: 22.12, y: 14.95 });
        assert!(hit.hits.iter().any(|hit| hit.kind == "text-box"
            && hit.uuid.as_deref() == Some("45454545-4545-4545-8545-454545454545")));

        let aabb_corner_miss = scene.hit_test(KicadPoint { x: 26.8, y: 10.3 });
        assert!(
            !aabb_corner_miss
                .hits
                .iter()
                .any(|hit| hit.kind == "text-box"
                    && hit.uuid.as_deref() == Some("45454545-4545-4545-8545-454545454545"))
        );
    }

    #[test]
    fn parses_schematic_images_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (image
    (at 36.83 39.37)
    (scale 1.5)
    (uuid "56565656-5656-4656-8656-565656565656")
    (data
      "iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"
    )
  )
)"#,
            "image.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.images.len(), 1);
        assert_eq!(
            schematic.images[0].uuid.as_deref(),
            Some("56565656-5656-4656-8656-565656565656")
        );
        assert_close(schematic.images[0].scale, 1.5);
        assert_eq!(schematic.images[0].mime_type(), "image/png");
        assert_close(schematic.images[0].image_size_mm().unwrap().width, 6.096);
        assert!(schematic.to_summary_json().contains("\"image_count\": 1"));

        let scene = schematic.canvas_scene();
        assert_eq!(scene.images.len(), 1);
        assert_eq!(scene.images[0].mime_type, "image/png");
        assert_close(scene.images[0].image_size.unwrap().height, 6.096);
        let bounds = scene.bounds.unwrap();
        assert_close(bounds.width(), 6.096);
        assert_close(bounds.height(), 6.096);
        assert!(scene.to_summary_json().contains("\"image_count\": 1"));
        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(scene_json["image_count"], 1);
        assert_eq!(
            scene_json["images"][0]["uuid"],
            "56565656-5656-4656-8656-565656565656"
        );
        assert_eq!(scene_json["images"][0]["mime_type"], "image/png");
        assert_eq!(scene_json["images"][0]["scale"], 1.5);
        assert_close(
            scene_json["images"][0]["bounds"]["width"].as_f64().unwrap(),
            6.096,
        );
        assert_close(
            scene_json["images"][0]["bounds"]["height"]
                .as_f64()
                .unwrap(),
            6.096,
        );
        assert_eq!(
            scene_json["images"][0]["data_base64"],
            "iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(image (at 36.83 39.37) (scale 1.5)"));
        assert!(roundtrip.contains("(data"));
        assert!(roundtrip.contains("iVBORw0KGgoAAAANSUhEUgAAADAAAAAwCAYAAABXAvmH"));
        assert!(roundtrip.contains("(uuid \"56565656-5656-4656-8656-565656565656\")"));
        let reparsed = parse_kicad_schematic(&roundtrip, "image_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.images.len(), 1);
        assert_eq!(reparsed.images[0].mime_type(), "image/png");
        assert_eq!(reparsed.canvas_scene().images.len(), 1);
    }

    #[test]
    fn parses_schematic_tables_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 2)
    (border (external yes) (header yes) (stroke (width 0.127) (type dash) (color 10 20 30 1)))
    (separators (rows yes) (cols no) (stroke (width 0.0508) (type dot) (color 40 50 60 0.5)))
    (column_widths 26.67 21.59)
    (row_heights 2.54 2.54)
    (uuid "67676767-6767-4767-8767-676767676767")
    (cells
      (table_cell "LED pin"
        (exclude_from_sim no)
        (at 122.555 29.21 0)
        (size 26.67 2.54)
        (margins 0.9525 0.9525 0.9525 0.9525)
        (span 1 1)
        (fill (type color) (color 255 228 206 0.5))
        (effects (font (size 1.27 1.27) (color 10 9 37 1)) (justify left top))
        (uuid "68686868-6868-4868-8868-686868686868")
      )
      (table_cell "Expected net"
        (exclude_from_sim no)
        (at 149.225 29.21 0)
        (size 21.59 2.54)
        (margins 0.9525 0.9525 0.9525 0.9525)
        (span 1 1)
        (fill (type none))
        (effects (font (size 1.27 1.27)) (justify left top))
        (uuid "69696969-6969-4969-8969-696969696969")
        (locked)
      )
    )
  )
)"#,
            "table.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.tables.len(), 1);
        assert_eq!(schematic.tables[0].column_count, 2);
        assert_eq!(schematic.tables[0].cells.len(), 2);
        assert_eq!(schematic.tables[0].cells[0].text, "LED pin");
        assert_close(
            schematic.tables[0]
                .border
                .as_ref()
                .unwrap()
                .stroke
                .as_ref()
                .unwrap()
                .width
                .unwrap(),
            0.127,
        );
        assert_eq!(
            schematic.tables[0].separators.as_ref().unwrap().cols,
            Some(false)
        );
        assert_eq!(
            schematic.tables[0].cells[0]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("color")
        );
        assert_eq!(
            schematic.tables[0].cells[0]
                .effects
                .as_ref()
                .unwrap()
                .justify,
            vec!["left".to_string(), "top".to_string()]
        );
        assert_eq!(schematic.tables[0].cells[1].locked, Some(true));
        assert_close(schematic.tables[0].column_widths[0], 26.67);
        assert_close(schematic.tables[0].row_heights[0], 2.54);
        assert_eq!(
            schematic.tables[0].uuid.as_deref(),
            Some("67676767-6767-4767-8767-676767676767")
        );
        assert_eq!(
            schematic.tables[0].cells[0].uuid.as_deref(),
            Some("68686868-6868-4868-8868-686868686868")
        );
        assert!(schematic.to_summary_json().contains("\"table_count\": 1"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"table_cell_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_table_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_table_cell_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"locked_table_cell_count\": 1")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.tables.len(), 1);
        assert_eq!(scene.tables[0].cells.len(), 2);
        assert_eq!(
            scene.tables[0].cells[0]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("color")
        );
        assert!(scene.to_summary_json().contains("\"table_count\": 1"));
        assert!(scene.to_summary_json().contains("\"table_cell_count\": 2"));
        assert_close(scene.bounds.unwrap().width(), 48.26);
        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(scene_json["table_count"], 1);
        assert_eq!(scene_json["table_cell_count"], 2);
        assert_eq!(
            scene_json["tables"][0]["uuid"],
            "67676767-6767-4767-8767-676767676767"
        );
        assert_close(
            scene_json["tables"][0]["bounds"]["width"].as_f64().unwrap(),
            48.26,
        );
        assert_eq!(scene_json["tables"][0]["column_count"], 2);
        assert_eq!(scene_json["tables"][0]["cell_count"], 2);
        assert_eq!(
            scene_json["tables"][0]["cells"][0]["uuid"],
            "68686868-6868-4868-8868-686868686868"
        );
        assert_close(
            scene_json["tables"][0]["cells"][0]["bounds"]["width"]
                .as_f64()
                .unwrap(),
            26.67,
        );
        assert_eq!(scene_json["tables"][0]["cells"][0]["text"], "LED pin");
        assert_eq!(
            scene_json["tables"][0]["cells"][0]["effects"]["justify"][1],
            "top"
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(table"));
        assert!(roundtrip.contains("(column_count 2)"));
        assert!(roundtrip.contains(
            "(border (external yes) (header yes) (stroke (width 0.127) (type dash) (color 10 20 30 1)))"
        ));
        assert!(roundtrip.contains(
            "(separators (rows yes) (cols no) (stroke (width 0.0508) (type dot) (color 40 50 60 0.5)))"
        ));
        assert!(roundtrip.contains("(column_widths 26.67 21.59)"));
        assert!(roundtrip.contains("(fill (type color) (color 255 228 206 0.5))"));
        assert!(
            roundtrip
                .contains("(effects (font (size 1.27 1.27) (color 10 9 37 1)) (justify left top))")
        );
        assert!(roundtrip.contains("(locked yes)"));
        assert!(roundtrip.contains("(table_cell \"LED pin\""));
        assert!(roundtrip.contains("(uuid \"67676767-6767-4767-8767-676767676767\")"));
        assert!(roundtrip.contains("(uuid \"68686868-6868-4868-8868-686868686868\")"));
        let reparsed = parse_kicad_schematic(&roundtrip, "table_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.tables.len(), 1);
        assert_eq!(reparsed.tables[0].cells.len(), 2);
        assert_eq!(reparsed.tables[0].cells[1].locked, Some(true));
        assert_eq!(
            reparsed.tables[0].cells[0]
                .effects
                .as_ref()
                .unwrap()
                .font_color,
            Some(KicadColor {
                red: 10.0,
                green: 9.0,
                blue: 37.0,
                alpha: 1.0,
            })
        );
        assert_eq!(reparsed.canvas_scene().tables.len(), 1);
    }

    #[test]
    fn hit_tests_rotated_table_cells_by_shape() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 1)
    (column_widths 10)
    (row_heights 4)
    (uuid "67676767-6767-4767-8767-676767676767")
    (cells
      (table_cell "Rotated cell"
        (at 40 10 45)
        (size 10 4)
        (uuid "68686868-6868-4868-8868-686868686868")
      )
    )
  )
)"#,
            "rotated_table_hit.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        let cell = &scene.tables[0].cells[0];
        assert!(cell.bounds.unwrap().width() > 9.0);
        assert!(cell.bounds.unwrap().height() > 9.0);

        let hit = scene.hit_test(KicadPoint { x: 42.12, y: 14.95 });
        assert!(hit.hits.iter().any(|hit| hit.kind == "table-cell"
            && hit.uuid.as_deref() == Some("68686868-6868-4868-8868-686868686868")));
        assert!(hit.hits.iter().any(|hit| hit.kind == "table"
            && hit.uuid.as_deref() == Some("67676767-6767-4767-8767-676767676767")));

        let aabb_corner_miss = scene.hit_test(KicadPoint { x: 46.8, y: 10.3 });
        assert!(
            !aabb_corner_miss
                .hits
                .iter()
                .any(|hit| (hit.kind == "table-cell"
                    && hit.uuid.as_deref() == Some("68686868-6868-4868-8868-686868686868"))
                    || (hit.kind == "table"
                        && hit.uuid.as_deref() == Some("67676767-6767-4767-8767-676767676767")))
        );
    }

    #[test]
    fn parses_schematic_groups_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 5 5) (xy 10 5)) (uuid "7e1da7e2-473f-48bf-b7bf-2eb79e1b1372"))
  (label "OUT" (at 10 5 0) (uuid "d26fc350-11e5-4917-ba78-4e25070d7aa8"))
  (group "GroupName"
    (uuid "7267eac2-0eb2-494a-bc81-61295bcdf08c")
    (locked yes)
    (members "7e1da7e2-473f-48bf-b7bf-2eb79e1b1372" "d26fc350-11e5-4917-ba78-4e25070d7aa8")
  )
)"#,
            "group.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.groups.len(), 1);
        assert_eq!(schematic.groups[0].name, "GroupName");
        assert_eq!(
            schematic.groups[0].uuid.as_deref(),
            Some("7267eac2-0eb2-494a-bc81-61295bcdf08c")
        );
        assert_eq!(schematic.groups[0].locked, Some(true));
        assert_eq!(schematic.groups[0].members.len(), 2);
        assert_eq!(
            schematic.groups[0].members[0],
            "7e1da7e2-473f-48bf-b7bf-2eb79e1b1372"
        );
        assert!(schematic.to_summary_json().contains("\"group_count\": 1"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"group_member_count\": 2")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.wires.len(), 1);
        assert_eq!(scene.groups.len(), 1);
        assert_eq!(
            scene.groups[0].uuid.as_deref(),
            Some("7267eac2-0eb2-494a-bc81-61295bcdf08c")
        );
        assert_eq!(scene.groups[0].members.len(), 2);
        assert!(scene.to_summary_json().contains("\"wire_count\": 1"));
        assert!(scene.to_summary_json().contains("\"group_count\": 1"));
        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(scene_json["group_count"], 1);
        assert_eq!(scene_json["group_member_count"], 2);
        assert_eq!(
            scene_json["groups"][0]["uuid"],
            "7267eac2-0eb2-494a-bc81-61295bcdf08c"
        );
        assert_eq!(scene_json["groups"][0]["member_count"], 2);
        assert!(scene_json["groups"][0]["bounds"]["width"].as_f64().unwrap() > 5.0);

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(group \"GroupName\""));
        assert!(roundtrip.contains("(uuid \"7267eac2-0eb2-494a-bc81-61295bcdf08c\")"));
        assert!(roundtrip.contains("(locked yes)"));
        assert!(roundtrip.contains(
            "(members \"7e1da7e2-473f-48bf-b7bf-2eb79e1b1372\" \"d26fc350-11e5-4917-ba78-4e25070d7aa8\")"
        ));
        let reparsed = parse_kicad_schematic(&roundtrip, "group_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.groups.len(), 1);
        assert_eq!(reparsed.groups[0].members.len(), 2);
        assert_eq!(reparsed.groups[0].locked, Some(true));
    }

    #[test]
    fn preserves_schematic_file_metadata_and_instances() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "eeschema")
  (generator_version "9.99")
  (uuid "10101010-1010-4010-8010-101010101010")
  (paper "A4")
  (title_block
    (title "Control Board")
    (date "2026-06-09")
    (rev "A")
    (company "NekoSpice")
    (comment 1 "simulation front-end")
    (comment 4 "${APPROVER}")
  )
  (lib_symbols)
  (symbol
    (lib_id "Device:R")
    (at 10 20 0)
    (unit 1)
    (uuid "20202020-2020-4020-8020-202020202020")
    (property "Reference" "R1" (at 10 17.46 0))
    (property "Value" "1k" (at 10 22.54 0))
    (pin "1" (uuid "30303030-3030-4030-8030-303030303030"))
    (pin "2" (uuid "40404040-4040-4040-8040-404040404040"))
  )
  (sheet_instances
    (path "/" (page "1"))
    (path "/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee" (page "2"))
  )
  (symbol_instances
    (path "/20202020-2020-4020-8020-202020202020"
      (reference "R1")
      (unit 1)
      (value "1k")
      (footprint "")
    )
  )
  (embedded_fonts no)
)"#,
            "metadata.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.generator_version.as_deref(), Some("9.99"));
        assert_eq!(schematic.embedded_fonts, Some(false));
        let title_block = schematic.title_block.as_ref().unwrap();
        assert_eq!(title_block.title.as_deref(), Some("Control Board"));
        assert_eq!(title_block.revision.as_deref(), Some("A"));
        assert_eq!(title_block.comments.len(), 2);
        assert_eq!(title_block.comments[1].index, 4);
        assert_eq!(title_block.comments[1].text, "${APPROVER}");
        assert_eq!(schematic.sheet_instances.len(), 2);
        assert_eq!(schematic.sheet_instances[1].page.as_deref(), Some("2"));
        assert_eq!(schematic.symbol_instances.len(), 1);
        assert_eq!(
            schematic.symbol_instances[0].path,
            "/20202020-2020-4020-8020-202020202020"
        );
        assert_eq!(
            schematic.symbol_instances[0].reference.as_deref(),
            Some("R1")
        );
        assert_eq!(schematic.symbol_instances[0].unit, Some(1));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"has_title_block\": true")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"title_comment_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"sheet_instance_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"symbol_instance_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"embedded_fonts\": false")
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(generator_version \"9.99\")"));
        assert!(roundtrip.contains("(title \"Control Board\")"));
        assert!(roundtrip.contains("(comment 4 \"${APPROVER}\")"));
        assert!(roundtrip.contains("(sheet_instances"));
        assert!(roundtrip.contains("(path \"/\" (page \"1\"))"));
        assert!(roundtrip.contains("(symbol_instances"));
        assert!(roundtrip.contains("(reference \"R1\")"));
        assert!(roundtrip.contains("(embedded_fonts no)"));

        let reparsed = parse_kicad_schematic(&roundtrip, "metadata_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.generator_version.as_deref(), Some("9.99"));
        assert_eq!(reparsed.title_block.unwrap().comments.len(), 2);
        assert_eq!(reparsed.sheet_instances.len(), 2);
        assert_eq!(reparsed.symbol_instances.len(), 1);
        assert_eq!(reparsed.embedded_fonts, Some(false));
    }

    #[test]
    fn preserves_symbol_instance_pin_alternates_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "NekoSpice:AltPin")
    (at 10 20 0)
    (unit 1)
    (uuid "20202020-2020-4020-8020-202020202020")
    (property "Reference" "U1" (at 10 17.46 0))
    (property "Value" "AltPin" (at 10 22.54 0))
    (pin "G39"
      (uuid "30303030-3030-4030-8030-303030303030")
      (alternate "CAN0_DIN")
    )
    (pin "G38"
      (uuid "40404040-4040-4040-8040-404040404040")
      (alternate "CAN0_DOUT")
    )
  )
)"#,
            "symbol_pin_alternates.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.symbols.len(), 1);
        assert_eq!(schematic.symbols[0].pins.len(), 2);
        assert_eq!(
            schematic.symbols[0].pins[0].alternate.as_deref(),
            Some("CAN0_DIN")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"symbol_pin_alternate_count\": 2")
        );

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(alternate \"CAN0_DIN\")"));
        assert!(exported.contains("(alternate \"CAN0_DOUT\")"));

        let reparsed =
            parse_kicad_schematic(&exported, "symbol_pin_alternates_roundtrip.kicad_sch").unwrap();
        assert_eq!(
            reparsed.symbols[0].pins[1].alternate.as_deref(),
            Some("CAN0_DOUT")
        );
    }

    #[test]
    fn preserves_embedded_project_instances_and_variants() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "Connector:J")
    (at 10 20 0)
    (unit 1)
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Reference" "J1" (at 10 17.46 0))
    (property "Value" "Conn" (at 10 22.54 0))
    (pin "1" (uuid "22222222-2222-4222-8222-222222222222"))
    (instances
      (project "variants"
        (path "/aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"
          (reference "J1")
          (unit 1)
          (variant
            (name "Variant 1")
            (dnp yes)
          )
        )
      )
    )
  )
  (sheet
    (at 40 20)
    (size 20 10)
    (uuid "33333333-3333-4333-8333-333333333333")
    (property "Sheetname" "Sub" (at 40 17.46 0))
    (property "Sheetfile" "sub.kicad_sch" (at 40 32.54 0))
    (instances
      (project "variants"
        (path "/33333333-3333-4333-8333-333333333333"
          (page "2")
        )
      )
    )
  )
)"#,
            "embedded_instances.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.symbols[0].instances.len(), 1);
        assert_eq!(schematic.symbols[0].instances[0].name, "variants");
        assert_eq!(schematic.symbols[0].instances[0].paths.len(), 1);
        let symbol_path = &schematic.symbols[0].instances[0].paths[0];
        assert_eq!(symbol_path.reference.as_deref(), Some("J1"));
        assert_eq!(symbol_path.unit, Some(1));
        assert_eq!(symbol_path.variants.len(), 1);
        assert_eq!(symbol_path.variants[0].name.as_deref(), Some("Variant 1"));
        assert_eq!(symbol_path.variants[0].dnp, Some(true));
        assert_eq!(schematic.sheets[0].instances.len(), 1);
        assert_eq!(
            schematic.sheets[0].instances[0].paths[0].page.as_deref(),
            Some("2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"embedded_project_instance_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"embedded_instance_path_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"variant_instance_count\": 1")
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(instances"));
        assert!(roundtrip.contains("(project \"variants\""));
        assert!(roundtrip.contains("(reference \"J1\")"));
        assert!(roundtrip.contains("(name \"Variant 1\")"));
        assert!(roundtrip.contains("(dnp yes)"));
        assert!(roundtrip.contains("(page \"2\")"));
        let reparsed =
            parse_kicad_schematic(&roundtrip, "embedded_instances_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.symbols[0].instances[0].paths[0].variants.len(), 1);
        assert_eq!(
            reparsed.sheets[0].instances[0].paths[0].page.as_deref(),
            Some("2")
        );
    }

    #[test]
    fn preserves_symbol_and_sheet_assembly_flags() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "Device:R")
    (at 10 20 0)
    (mirror x y)
    (unit 1)
    (exclude_from_sim no)
    (in_bom no)
    (on_board yes)
    (dnp yes)
    (fields_autoplaced yes)
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Reference" "Rskip" (at 10 17.46 0))
    (property "Value" "DNP" (at 10 22.54 0))
    (pin "1" (uuid "22222222-2222-4222-8222-222222222222"))
  )
  (sheet
    (at 40 20)
    (size 20 10)
    (exclude_from_sim no)
    (in_bom yes)
    (on_board no)
    (dnp no)
    (fields_autoplaced yes)
    (uuid "33333333-3333-4333-8333-333333333333")
    (property "Sheetname" "Sub" (at 40 17.46 0))
    (property "Sheetfile" "sub.kicad_sch" (at 40 32.54 0))
  )
)"#,
            "assembly_flags.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.symbols[0].mirror.as_deref(), Some("x y"));
        assert_eq!(schematic.symbols[0].in_bom, Some(false));
        assert_eq!(schematic.symbols[0].on_board, Some(true));
        assert_eq!(schematic.symbols[0].dnp, Some(true));
        assert_eq!(schematic.symbols[0].fields_autoplaced, Some(true));
        assert_eq!(schematic.sheets[0].in_bom, Some(true));
        assert_eq!(schematic.sheets[0].on_board, Some(false));
        assert_eq!(schematic.sheets[0].dnp, Some(false));
        assert_eq!(schematic.sheets[0].fields_autoplaced, Some(true));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"dnp_item_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"bom_excluded_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"board_excluded_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"mirrored_symbol_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"fields_autoplaced_count\": 2")
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(mirror x y)"));
        assert!(roundtrip.contains("(in_bom no)"));
        assert!(roundtrip.contains("(on_board yes)"));
        assert!(roundtrip.contains("(dnp yes)"));
        assert!(roundtrip.contains("(fields_autoplaced yes)"));
        assert!(roundtrip.contains("(on_board no)"));
        let reparsed =
            parse_kicad_schematic(&roundtrip, "assembly_flags_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.symbols[0].mirror.as_deref(), Some("x y"));
        assert_eq!(reparsed.symbols[0].dnp, Some(true));
        assert_eq!(reparsed.sheets[0].on_board, Some(false));
        assert_eq!(reparsed.sheets[0].fields_autoplaced, Some(true));
    }

    #[test]
    fn preserves_property_display_flags_and_effects() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (symbol
    (lib_id "Device:R")
    (at 10 20 0)
    (unit 1)
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Reference" "R1"
      (id 0)
      (at 10 17.46 0)
      (hide yes)
      (show_name no)
      (do_not_autoplace no)
      (effects
        (font
          (size 1.524 1.016)
          (thickness 0.254)
          (bold yes)
          (italic yes)
          (color 10 9 37 1)
        )
        (justify left bottom)
        (href "https://kicad.org")
      )
    )
    (property "Value" "1k"
      (at 10 22.54 0)
      (effects
        (font
          (size 1.27 1.27)
        )
      )
    )
    (pin "1" (uuid "22222222-2222-4222-8222-222222222222"))
  )
)"#,
            "property_effects.kicad_sch",
        )
        .unwrap();

        let property = &schematic.symbols[0].properties[0];
        assert_eq!(property.id, Some(0));
        assert_eq!(property.hide, Some(true));
        assert_eq!(property.show_name, Some(false));
        assert_eq!(property.do_not_autoplace, Some(false));
        let effects = property.effects.as_ref().unwrap();
        assert_close(effects.font_size.unwrap().width, 1.524);
        assert_close(effects.font_size.unwrap().height, 1.016);
        assert_close(effects.font_thickness.unwrap(), 0.254);
        assert_eq!(effects.font_bold, Some(true));
        assert_eq!(effects.font_italic, Some(true));
        assert_eq!(
            effects.font_color,
            Some(KicadColor {
                red: 10.0,
                green: 9.0,
                blue: 37.0,
                alpha: 1.0,
            })
        );
        assert_eq!(
            effects.justify,
            vec!["left".to_string(), "bottom".to_string()]
        );
        assert_eq!(effects.href.as_deref(), Some("https://kicad.org"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"hidden_property_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"property_effect_count\": 2")
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(hide yes)"));
        assert!(roundtrip.contains("(id 0)"));
        assert!(roundtrip.contains("(show_name no)"));
        assert!(roundtrip.contains("(do_not_autoplace no)"));
        assert!(roundtrip.contains("(font (size 1.524 1.016)"));
        assert!(roundtrip.contains("(thickness 0.254)"));
        assert!(roundtrip.contains("(bold yes)"));
        assert!(roundtrip.contains("(italic yes)"));
        assert!(roundtrip.contains("(color 10 9 37 1)"));
        assert!(roundtrip.contains("(justify left bottom)"));
        assert!(roundtrip.contains("(href \"https://kicad.org\")"));
        let reparsed =
            parse_kicad_schematic(&roundtrip, "property_effects_roundtrip.kicad_sch").unwrap();
        let property = &reparsed.symbols[0].properties[0];
        assert_eq!(property.id, Some(0));
        assert_eq!(property.hide, Some(true));
        assert_eq!(property.show_name, Some(false));
        assert_eq!(property.do_not_autoplace, Some(false));
        assert_eq!(property.effects.as_ref().unwrap().font_bold, Some(true));
        assert_eq!(
            property.effects.as_ref().unwrap().justify,
            vec!["left".to_string(), "bottom".to_string()]
        );
    }

    #[test]
    fn preserves_canvas_text_effects() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (label "OUT"
    (at 10 5 0)
    (effects (font (size 1.27 1.27) italic) (justify left bottom) hide)
    (uuid "11111111-1111-4111-8111-111111111111")
  )
  (text "note"
    (at 20 5 0)
    (effects
      (font
        (size 1.905 1.905)
        (thickness 0.254)
        (bold yes)
        (color 10 9 37 1)
      )
      (justify right)
      (href "https://kicad.org")
    )
    (uuid "22222222-2222-4222-8222-222222222222")
  )
  (text_box "box"
    (at 30 5 0)
    (size 10 5)
    (effects (font (size 1.27 1.27) (italic yes)) (justify center))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
  (sheet
    (at 40 5)
    (size 15 10)
    (uuid "44444444-4444-4444-8444-444444444444")
    (property "Sheetname" "Sub" (at 40 4 0))
    (property "Sheetfile" "sub.kicad_sch" (at 40 16 0))
    (pin "BUS{0}" bidirectional
      (at 55 10 0)
      (effects (font (size 1.27 1.27)) (justify right))
      (uuid "55555555-5555-4555-8555-555555555555")
    )
  )
)"#,
            "canvas_text_effects.kicad_sch",
        )
        .unwrap();

        let label_effects = schematic.labels[0].effects.as_ref().unwrap();
        assert_eq!(label_effects.font_italic, Some(true));
        assert_eq!(
            label_effects.justify,
            vec!["left".to_string(), "bottom".to_string()]
        );
        assert!(label_effects.hide);

        let text_effects = schematic.text_items[0].effects.as_ref().unwrap();
        assert_close(text_effects.font_size.unwrap().width, 1.905);
        assert_close(text_effects.font_thickness.unwrap(), 0.254);
        assert_eq!(text_effects.font_bold, Some(true));
        assert_eq!(
            text_effects.font_color,
            Some(KicadColor {
                red: 10.0,
                green: 9.0,
                blue: 37.0,
                alpha: 1.0,
            })
        );
        assert_eq!(text_effects.href.as_deref(), Some("https://kicad.org"));
        assert_eq!(
            schematic.text_boxes[0]
                .effects
                .as_ref()
                .unwrap()
                .font_italic,
            Some(true)
        );
        assert_eq!(
            schematic.sheets[0].pins[0]
                .effects
                .as_ref()
                .unwrap()
                .justify,
            vec!["right".to_string()]
        );

        let scene = schematic.canvas_scene();
        assert!(scene.labels[0].effects.as_ref().unwrap().hide);
        assert_eq!(
            scene.text_items[0]
                .effects
                .as_ref()
                .unwrap()
                .href
                .as_deref(),
            Some("https://kicad.org")
        );
        assert_eq!(
            scene.text_boxes[0].effects.as_ref().unwrap().font_italic,
            Some(true)
        );
        assert_eq!(
            scene.sheets[0].pins[0].effects.as_ref().unwrap().justify,
            vec!["right".to_string()]
        );
        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(scene_json["sheet_count"], 1);
        assert_eq!(scene_json["sheet_pin_count"], 1);
        assert_eq!(scene_json["label_count"], 1);
        assert_eq!(scene_json["text_box_count"], 1);
        assert_eq!(scene_json["sheets"][0]["name"], "Sub");
        assert_eq!(
            scene_json["sheets"][0]["pins"][0]["effects"]["justify"][0],
            "right"
        );
        assert_eq!(scene_json["labels"][0]["effects"]["hide"], true);
        assert_eq!(
            scene_json["text_items"][0]["effects"]["href"],
            "https://kicad.org"
        );
        assert_eq!(scene_json["text_boxes"][0]["effects"]["font_italic"], true);

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(justify left bottom) hide"));
        assert!(roundtrip.contains("(thickness 0.254)"));
        assert!(roundtrip.contains("(bold yes)"));
        assert!(roundtrip.contains("(color 10 9 37 1)"));
        assert!(roundtrip.contains("(href \"https://kicad.org\")"));
        assert!(roundtrip.contains("(justify right)"));
        let reparsed =
            parse_kicad_schematic(&roundtrip, "canvas_text_effects_roundtrip.kicad_sch").unwrap();
        assert_eq!(
            reparsed.labels[0].effects.as_ref().unwrap().font_italic,
            Some(true)
        );
        assert_eq!(
            reparsed.text_items[0].effects.as_ref().unwrap().font_bold,
            Some(true)
        );
        assert_eq!(
            reparsed.sheets[0].pins[0].effects.as_ref().unwrap().justify,
            vec!["right".to_string()]
        );
    }

    #[test]
    fn preserves_kicad_directive_labels_and_roundtrips() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (netclass_flag ""
    (length 3.81)
    (shape dot)
    (at 102.87 30.48 0)
    (fields_autoplaced yes)
    (effects
      (font
        (size 1.27 1.27)
        (color 236 104 255 1)
      )
      (justify left bottom)
    )
    (uuid "3c7ec402-4c06-4b52-9acd-ed760671ff85")
    (property "Net Class" "HV"
      (at 103.5685 27.94 0)
      (show_name no)
      (do_not_autoplace no)
      (effects (font (size 1.27 1.27)) (justify left))
    )
    (property "Component Class" "Classy"
      (at 99.822 24.892 0)
      (show_name no)
      (do_not_autoplace no)
      (effects (font (size 1.27 1.27) (italic yes)) (justify left))
    )
  )
  (netclass_flag ""
    (length 2.54)
    (shape dot)
    (at 110 30 0)
    (property "Net Class" "" (at 110 28 0))
    (property "Component Class" "OnlyComponent" (at 110 26 0))
  )
)"#,
            "directive_label.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.directive_labels.len(), 2);
        let label = &schematic.directive_labels[0];
        assert_eq!(label.display_text(), "HV");
        assert_eq!(
            schematic.directive_labels[1].display_text(),
            "OnlyComponent"
        );
        assert_close(label.length.unwrap(), 3.81);
        assert_eq!(label.shape.as_deref(), Some("dot"));
        assert_eq!(label.fields_autoplaced, Some(true));
        assert_eq!(
            label.effects.as_ref().unwrap().font_color,
            Some(KicadColor {
                red: 236.0,
                green: 104.0,
                blue: 255.0,
                alpha: 1.0,
            })
        );
        assert_eq!(label.properties.len(), 2);
        assert!(
            schematic
                .to_summary_json()
                .contains("\"directive_label_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"directive_label_property_count\": 4")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.directive_labels.len(), 2);
        assert_eq!(scene.directive_labels[0].text, "HV");
        assert_eq!(scene.directive_labels[1].text, "OnlyComponent");
        assert!(
            scene
                .to_summary_json()
                .contains("\"directive_label_count\": 2")
        );
        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(scene_json["directive_label_count"], 2);
        assert_eq!(scene_json["directive_labels"][0]["text"], "HV");
        assert_eq!(scene_json["directive_labels"][0]["shape"], "dot");
        assert_eq!(
            scene_json["directive_labels"][0]["properties"][1]["effects"]["font_italic"],
            true
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(netclass_flag \"\""));
        assert!(roundtrip.contains("(length 3.81)"));
        assert!(roundtrip.contains("(shape dot)"));
        assert!(roundtrip.contains("(fields_autoplaced yes)"));
        assert!(roundtrip.contains("(color 236 104 255 1)"));
        assert!(roundtrip.contains("(property \"Net Class\" \"HV\""));
        let reparsed =
            parse_kicad_schematic(&roundtrip, "directive_label_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.directive_labels.len(), 2);
        assert_eq!(
            reparsed.directive_labels[0].uuid.as_deref(),
            Some("3c7ec402-4c06-4b52-9acd-ed760671ff85")
        );
        assert_eq!(reparsed.directive_labels[0].display_text(), "HV");
        assert_eq!(reparsed.directive_labels[1].display_text(), "OnlyComponent");
    }

    #[test]
    fn preserves_label_shape_autoplace_and_properties() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20251028)
  (generator "eeschema")
  (paper "A4")
  (lib_symbols)
  (global_label "NET_OK" (shape input) (at 31.75 30.48 0) (fields_autoplaced)
    (effects (font (size 1.27 1.27)) (justify left))
    (uuid "11111111-1111-4111-8111-111111111111")
    (property "Intersheet References" "${INTERSHEET_REFS}" (id 0) (at 41.2993 30.4006 0)
      (effects (font (size 1.27 1.27)) (justify left) hide)
    )
  )
  (hierarchical_label "CHILD_IN"
    (shape output)
    (at 50.8 30.48 180)
    (fields_autoplaced no)
    (effects (font (size 1.27 1.27)) (justify right))
    (uuid "22222222-2222-4222-8222-222222222222")
  )
)"#,
            "label_metadata.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.labels.len(), 2);
        let global = &schematic.labels[0];
        assert_eq!(global.kind, KicadLabelKind::Global);
        assert_eq!(global.shape.as_deref(), Some("input"));
        assert_eq!(global.fields_autoplaced, Some(true));
        assert_eq!(global.properties.len(), 1);
        assert_eq!(global.properties[0].id, Some(0));
        assert!(global.properties[0].effects.as_ref().unwrap().hide);

        let hierarchical = &schematic.labels[1];
        assert_eq!(hierarchical.kind, KicadLabelKind::Hierarchical);
        assert_eq!(hierarchical.shape.as_deref(), Some("output"));
        assert_eq!(hierarchical.fields_autoplaced, Some(false));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"fields_autoplaced_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"shaped_label_count\": 2")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"label_property_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"hidden_property_count\": 1")
        );

        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(global_label \"NET_OK\" (shape input)"));
        assert!(roundtrip.contains("(fields_autoplaced yes)"));
        assert!(
            roundtrip.contains("(property \"Intersheet References\" \"${INTERSHEET_REFS}\" (id 0)")
        );
        assert!(roundtrip.contains("(justify left) hide"));
        assert!(roundtrip.contains("(hierarchical_label \"CHILD_IN\" (shape output)"));
        assert!(roundtrip.contains("(fields_autoplaced no)"));

        let reparsed =
            parse_kicad_schematic(&roundtrip, "label_metadata_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.labels[0].shape.as_deref(), Some("input"));
        assert_eq!(reparsed.labels[0].fields_autoplaced, Some(true));
        assert_eq!(reparsed.labels[0].properties[0].id, Some(0));
        assert_eq!(reparsed.labels[1].fields_autoplaced, Some(false));
    }

    #[test]
    fn parses_hierarchical_sheet_items_and_reports_unsupported_expansion() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (wire (pts (xy 5 5) (xy 10 5)))
  (label "0" (at 5 5 0))
  (text ".op" (at 5 2 0))
  (sheet
    (at 20 10)
    (size 15 10)
    (exclude_from_sim no)
    (stroke (width 0.3048) (type dash) (color 139 160 255 1))
    (fill (color 247 255 168 0.3607843137))
    (uuid "aaaaaaaa-0000-0000-0000-000000000001")
    (property "Sheetname" "gain_stage" (at 20 9 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (at 20 21 0))
    (pin "in" input (at 20 15 180) (uuid "aaaaaaaa-0000-0000-0000-000000000002"))
    (pin "out" output (at 35 15 0) (uuid "aaaaaaaa-0000-0000-0000-000000000003"))
  )
)"#,
            "hierarchical.kicad_sch",
        )
        .unwrap();

        assert_eq!(schematic.sheets.len(), 1);
        assert_eq!(schematic.sheets[0].sheet_name(), Some("gain_stage"));
        assert_eq!(
            schematic.sheets[0].sheet_file(),
            Some("gain_stage.kicad_sch")
        );
        assert_eq!(schematic.sheets[0].pins.len(), 2);
        assert_eq!(schematic.sheets[0].pins[0].pin_type, "input");
        assert_eq!(schematic.sheets[0].bounding_box().unwrap().width(), 15.0);
        assert_close(
            schematic.sheets[0].stroke.as_ref().unwrap().width.unwrap(),
            0.3048,
        );
        assert_eq!(
            schematic.sheets[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash")
        );
        assert_eq!(
            schematic.sheets[0].stroke.as_ref().unwrap().color,
            Some(KicadColor {
                red: 139.0,
                green: 160.0,
                blue: 255.0,
                alpha: 1.0,
            })
        );
        assert_eq!(schematic.sheets[0].fill.as_ref().unwrap().fill_type, None);
        assert_eq!(
            schematic.sheets[0].fill.as_ref().unwrap().color,
            Some(KicadColor {
                red: 247.0,
                green: 255.0,
                blue: 168.0,
                alpha: 0.3607843137,
            })
        );
        assert!(schematic.to_summary_json().contains("\"sheet_count\": 1"));
        assert!(
            schematic
                .to_summary_json()
                .contains("\"styled_sheet_count\": 1")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"sheet_pin_count\": 2")
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.sheets.len(), 1);
        assert_eq!(scene.sheets[0].pins.len(), 2);
        assert_eq!(
            scene.sheets[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash")
        );
        assert_eq!(
            scene.sheets[0].fill.as_ref().unwrap().color,
            Some(KicadColor {
                red: 247.0,
                green: 255.0,
                blue: 168.0,
                alpha: 0.3607843137,
            })
        );
        assert!(scene.to_summary_json().contains("\"sheet_count\": 1"));
        assert!(scene.to_summary_json().contains("\"sheet_pin_count\": 2"));

        let report = schematic.check_report();
        assert_eq!(report.sheet_count, 1);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == KicadDiagnosticSeverity::Error
                && diagnostic.code == "hierarchical-sheet-unsupported"
        }));

        let netlist = schematic.to_spice_netlist().unwrap();
        assert!(
            netlist
                .contains("* Unsupported KiCad hierarchical sheet gain_stage gain_stage.kicad_sch")
        );
        let roundtrip = schematic.to_kicad_schematic_sexpr();
        assert!(roundtrip.contains("(sheet"));
        assert!(roundtrip.contains("(stroke (width 0.3048) (type dash) (color 139 160 255 1))"));
        assert!(roundtrip.contains("(fill (color 247 255 168 0.3607843137))"));
        assert!(roundtrip.contains("(property \"Sheetname\" \"gain_stage\""));
        assert!(roundtrip.contains("(pin \"in\" input"));
        let reparsed =
            parse_kicad_schematic(&roundtrip, "hierarchical_roundtrip.kicad_sch").unwrap();
        assert_eq!(
            reparsed.sheets[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash")
        );
        assert_eq!(
            reparsed.sheets[0].fill.as_ref().unwrap().color,
            Some(KicadColor {
                red: 247.0,
                green: 255.0,
                blue: 168.0,
                alpha: 0.3607843137,
            })
        );
    }

    #[test]
    fn checks_hierarchical_schematic_fixture_with_expansion() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic_path =
            workspace_root.join("examples/kicad_hierarchical/kicad_hierarchical.kicad_sch");
        let schematic = read_kicad_schematic_with_libraries(&schematic_path).unwrap();
        let report = schematic
            .check_report_with_hierarchy(schematic_path.parent().unwrap())
            .unwrap();

        assert_eq!(report.sheet_count, 1);
        assert_eq!(report.spice_directive_count, 1);
        assert_eq!(report.error_count(), 0);
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "hierarchical-sheet-unsupported"
                || diagnostic.code == "missing-spice-directive"
        }));
    }

    #[test]
    fn exports_kicad_sim_fields_to_spice_netlist() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Dual"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "unused" (at 0 -2.54 0))
      (property "Sim.Device" "SUBCKT" (at 0 0 0))
      (property "Sim.Library" "models/opamp.lib" (at 0 0 0))
      (symbol "Dual_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "IN") (number "1"))
        (pin passive line (at 0 -2.54 90) (length 2.54) (name "OUT") (number "2"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "VCC") (number "3"))
      )
    )
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (wire (pts (xy 10 10) (xy 17.46 10)))
  (wire (pts (xy 20 0) (xy 20 7.46)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "out" (at 20 0 0))
  (label "vcc" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:Dual")
    (at 20 10 0)
    (property "Reference" "U1" (at 20 8 0))
    (property "Value" "opamp_model" (at 20 12 0))
    (property "Sim.Pins" "2=OUT 1=IN 3=VCC" (at 20 10 0))
    (property "Sim.Params" "model=\"opamp_model\" gain=100k" (at 20 10 0))
  )
  (symbol
    (lib_id "NekoSpice:R")
    (at 50 50 0)
    (exclude_from_sim yes)
    (property "Reference" "Rskip" (at 50 48 0))
    (property "Value" "1k" (at 50 52 0))
  )
)"#,
            "sim_fields.kicad_sch",
        )
        .unwrap();

        let netlist = schematic.to_spice_netlist().unwrap();

        assert!(netlist.contains(".include \"models/opamp.lib\""));
        assert!(netlist.contains("XU1 out in vcc opamp_model gain=100k"));
        assert!(!netlist.contains("Rskip"));
        assert!(netlist.contains(".op"));
        let reparsed = parse_kicad_schematic(
            &schematic.to_kicad_schematic_sexpr(),
            "sim_fields_roundtrip.kicad_sch",
        )
        .unwrap();
        assert_eq!(
            reparsed
                .symbols
                .iter()
                .find(|symbol| symbol.reference() == Some("Rskip"))
                .unwrap()
                .exclude_from_sim,
            Some(true)
        );
    }

    #[test]
    fn exports_legacy_kicad_spice_fields_to_spice_netlist() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:LegacyD"
      (property "Reference" "D" (at 0 0 0))
      (property "Value" "unused" (at 0 -2.54 0))
      (property "Spice_Primitive" "D" (at 0 0 0))
      (property "Spice_Model" "Dfast" (at 0 0 0))
      (symbol "LegacyD_0_1"
        (pin passive line (at 0 -2.54 90) (length 2.54) (name "A") (number "1"))
        (pin passive line (at 0 2.54 270) (length 2.54) (name "K") (number "2"))
      )
    )
  )
  (wire (pts (xy 40 37.46) (xy 35 37.46)))
  (wire (pts (xy 40 42.54) (xy 45 42.54)))
  (label "anode" (at 35 37.46 0))
  (label "0" (at 45 42.54 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:LegacyD")
    (at 40 40 0)
    (property "Reference" "XD1" (at 40 38 0))
    (property "Value" "ignored" (at 40 42 0))
    (property "Spice_Node_Sequence" "2 1" (at 40 40 0))
  )
)"#,
            "legacy_spice_fields.kicad_sch",
        )
        .unwrap();

        let netlist = schematic.to_spice_netlist().unwrap();

        assert!(netlist.contains("DXD1 0 anode Dfast"));
    }

    #[test]
    fn reports_invalid_sim_pin_mapping() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "R_0_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
  )
  (wire (pts (xy 10 10) (xy 20 10)))
  (label "0" (at 10 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
    (property "Sim.Pins" "1 99" (at 12.54 10 0))
  )
)"#,
            "bad_sim_pins.kicad_sch",
        )
        .unwrap();

        let report = schematic.check_report();

        assert!(report.error_count() >= 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "invalid-sim-pin")
        );
    }

    #[test]
    fn resolves_missing_symbols_from_project_library_table() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_library_resolution_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        fs::copy(
            workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
            project_dir.join("neko_spice.kicad_sym"),
        )
        .unwrap();
        fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "NekoSpice")(type "KiCad")(uri "${KIPRJMOD}/neko_spice.kicad_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 10 10) (xy 7 10)))
  (wire (pts (xy 15.08 10) (xy 18 10)))
  (label "in" (at 7 10 0))
  (label "0" (at 18 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:R")
    (at 12.54 10 0)
    (property "Reference" "R1" (at 12.54 8 0))
    (property "Value" "1k" (at 12.54 12 0))
  )
)"#,
            "library_resolution.kicad_sch",
        )
        .unwrap();

        assert!(
            schematic
                .check_report()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing-symbol-definition")
        );
        let diagnostics = schematic
            .resolve_project_symbol_libraries(&project_dir)
            .unwrap();
        let netlist = schematic.to_spice_netlist().unwrap();

        assert_eq!(diagnostics.len(), 0);
        assert_eq!(schematic.library_symbols.len(), 1);
        assert!(netlist.contains("R1 in 0 1k"));
        assert!(
            !schematic
                .check_report()
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing-symbol-definition")
        );

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn resolves_external_derived_symbol_parent_from_library_table() {
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_derived_library_resolution_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            project_dir.join("derived.kicad_sym"),
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "BaseR"
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "1k" (at 0 -2.54 0))
    (property "Sim.Device" "R" (at 0 0 0))
    (symbol "BaseR_0_1"
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
    )
  )
  (symbol "DerivedR"
    (extends "BaseR")
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "10k" (at 0 -2.54 0))
  )
)"#,
        )
        .unwrap();
        fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "Demo")(type "KiCad")(uri "${KIPRJMOD}/derived.kicad_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (wire (pts (xy 17.46 10) (xy 10 10)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "0" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "Demo:DerivedR")
    (at 20 10 0)
    (property "Reference" "R1" (at 20 8 0))
    (property "Value" "4.7k" (at 20 12 0))
  )
)"#,
            "derived_library_resolution.kicad_sch",
        )
        .unwrap();

        let diagnostics = schematic
            .resolve_project_symbol_libraries(&project_dir)
            .unwrap();
        let scene = schematic.canvas_scene();
        let netlist = schematic.to_spice_netlist().unwrap();

        assert_eq!(diagnostics.len(), 0);
        assert!(schematic.symbol_definition("Demo:DerivedR").is_some());
        assert!(schematic.symbol_definition("Demo:BaseR").is_some());
        assert_eq!(scene.symbols[0].pins.len(), 2);
        assert!(netlist.contains("R1 in 0 4.7k"));

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn builds_canvas_scene_from_kicad_schematic_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let scene = schematic.canvas_scene();
        assert_eq!(scene.symbols.len(), 3);
        assert_eq!(
            scene
                .symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        assert_eq!(
            scene
                .symbols
                .iter()
                .map(|symbol| symbol.pins.len())
                .sum::<usize>(),
            6
        );
        assert_eq!(scene.wires.len(), 3);
        assert_eq!(scene.labels.len(), 3);
        assert_eq!(scene.text_items.len(), 1);
        assert!(scene.text_items[0].is_spice_directive);
        assert!(scene.bounds.unwrap().width() > 20.0);

        let resistor = scene
            .symbols
            .iter()
            .find(|symbol| symbol.reference == "R1")
            .unwrap();
        assert_eq!(resistor.lib_id, "NekoSpice:R");
        assert_eq!(resistor.graphics.len(), 1);
        assert_close(resistor.pins[0].start.x, 67.31);
        assert_close(resistor.pins[0].end.x, 69.85);
        assert!(scene.to_summary_json().contains("\"graphic_count\": 6"));
        assert!(scene.to_summary_json().contains("\"pin_count\": 6"));
        assert!(scene.to_summary_json().contains("\"text_count\": 1"));
        assert!(
            scene
                .to_summary_json()
                .contains("\"spice_directive_count\": 1")
        );
    }

    #[test]
    fn selects_kicad_symbol_unit_scope_for_canvas_and_netlist() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Multi"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Multi" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "Multi_0_1"
        (rectangle
          (start -1 -1)
          (end 1 1)
          (stroke (width 0) (type default))
          (fill (type none))
        )
      )
      (symbol "Multi_1_1"
        (polyline
          (pts (xy -1 0) (xy 1 0))
          (stroke (width 0.127) (type default))
          (fill (type none))
        )
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A1") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "B1") (number "2"))
      )
      (symbol "Multi_2_1"
        (circle
          (center 0 0)
          (radius 1)
          (stroke (width 0.127) (type default))
          (fill (type none))
        )
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A2") (number "3"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "B2") (number "4"))
      )
    )
  )
  (wire (pts (xy 17.46 10) (xy 10 10)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "0" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:Multi")
    (at 20 10 0)
    (unit 2)
    (body_style 1)
    (property "Reference" "R1" (at 20 8 0))
    (property "Value" "10k" (at 20 12 0))
  )
)"#,
            "multi_unit.kicad_sch",
        )
        .unwrap();

        let definition = schematic.symbol_definition("NekoSpice:Multi").unwrap();
        assert_eq!(definition.graphics[0].unit, 0);
        assert_eq!(definition.graphics[1].unit, 1);
        assert_eq!(definition.graphics[2].unit, 2);
        assert_eq!(definition.pins[0].unit, 1);
        assert_eq!(definition.pins[2].unit, 2);
        assert_eq!(
            definition
                .graphics
                .iter()
                .filter(|graphic| graphic.unit != 0)
                .count(),
            2
        );
        assert_eq!(
            definition.pins.iter().filter(|pin| pin.unit != 0).count(),
            4
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"symbol_body_style_count\": 1")
        );

        let scene = schematic.canvas_scene();
        let symbol = scene
            .symbols
            .iter()
            .find(|symbol| symbol.reference == "R1")
            .unwrap();
        assert_eq!(symbol.graphics.len(), 2);
        assert_eq!(symbol.pins.len(), 2);
        assert_eq!(symbol.pins[0].number, "3");
        assert_eq!(symbol.pins[1].number, "4");
        assert!(!symbol.pins.iter().any(|pin| pin.number == "1"));

        let netlist = schematic.to_spice_netlist().unwrap();
        assert!(netlist.contains("R1 in 0 10k"));

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(body_style 1)"));
        assert!(exported.contains("(symbol \"Multi_0_1\""));
        assert!(exported.contains("(symbol \"Multi_1_1\""));
        assert!(exported.contains("(symbol \"Multi_2_1\""));
        let reparsed = parse_kicad_schematic(&exported, "multi_unit_roundtrip.kicad_sch").unwrap();
        assert_eq!(
            reparsed
                .symbols
                .iter()
                .find(|symbol| symbol.reference() == Some("R1"))
                .unwrap()
                .body_style,
            Some(1)
        );
        assert_eq!(
            reparsed
                .canvas_scene()
                .symbols
                .iter()
                .find(|symbol| symbol.reference == "R1")
                .unwrap()
                .pins
                .len(),
            2
        );
    }

    #[test]
    fn preserves_kicad_symbol_unit_display_names() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:NamedUnits"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "NamedUnits" (at 0 -2.54 0))
      (symbol "NamedUnits_1_1"
        (unit_name "Power")
        (pin passive line (at -2.54 0 0) (length 2.54) (name "VIN") (number "1"))
      )
      (symbol "NamedUnits_2_1"
        (unit_name "Logic")
        (pin passive line (at 2.54 0 180) (length 2.54) (name "IO") (number "2"))
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:NamedUnits")
    (at 20 10 0)
    (unit 2)
    (property "Reference" "U1" (at 20 8 0))
    (property "Value" "NamedUnits" (at 20 12 0))
  )
)"#,
            "named_units.kicad_sch",
        )
        .unwrap();

        let definition = schematic.symbol_definition("NekoSpice:NamedUnits").unwrap();
        assert_eq!(
            definition.unit_names.get(&1).map(String::as_str),
            Some("Power")
        );
        assert_eq!(
            definition.unit_names.get(&2).map(String::as_str),
            Some("Logic")
        );
        assert_eq!(
            schematic.canvas_scene().symbols[0].unit_name.as_deref(),
            Some("Logic")
        );
        assert!(
            schematic
                .to_summary_json()
                .contains("\"library_unit_name_count\": 2")
        );

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(unit_name \"Power\")"));
        assert!(exported.contains("(unit_name \"Logic\")"));
        let reparsed = parse_kicad_schematic(&exported, "named_units_roundtrip.kicad_sch").unwrap();
        assert_eq!(
            reparsed
                .symbol_definition("NekoSpice:NamedUnits")
                .unwrap()
                .unit_names
                .get(&2)
                .map(String::as_str),
            Some("Logic")
        );

        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:NamedUnits"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "NamedUnits" (at 0 -2.54 0))
    (symbol "NamedUnits_1_1"
      (unit_name "Power")
      (pin passive line (at -2.54 0 0) (length 2.54) (name "VIN") (number "1"))
    )
    (symbol "NamedUnits_2_1"
      (unit_name "Logic")
      (pin passive line (at 2.54 0 180) (length 2.54) (name "IO") (number "2"))
    )
  )
)"#,
            "named_units.kicad_sym",
        )
        .unwrap();

        assert!(library.to_summary_json().contains("\"unit_name_count\": 2"));
        let exported_library = library.to_kicad_symbol_library_sexpr();
        assert!(exported_library.contains("(unit_name \"Power\")"));
        assert!(exported_library.contains("(unit_name \"Logic\")"));
    }

    #[test]
    fn roundtrips_kicad_schematic_fixture_through_writer() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(kicad_sch"));
        assert!(exported.contains("(lib_symbols"));
        assert!(exported.contains("(lib_id \"NekoSpice:R\")"));
        let reparsed = parse_kicad_schematic(&exported, "roundtrip.kicad_sch").unwrap();

        assert_eq!(reparsed.symbols.len(), 3);
        assert_eq!(reparsed.paper.as_deref(), Some("A4"));
        assert_eq!(reparsed.library_symbols.len(), 3);
        assert_eq!(reparsed.wires.len(), 3);
        assert_eq!(
            reparsed.wires[0].uuid.as_deref(),
            Some("22222222-2222-2222-2222-222222222222")
        );
        assert_eq!(reparsed.labels.len(), 3);
        assert_eq!(
            reparsed.labels[1].uuid.as_deref(),
            Some("66666666-6666-6666-6666-666666666666")
        );
        assert_eq!(reparsed.spice_directives()[0].text, ".tran 1u 1m");
        assert_eq!(
            reparsed.spice_directives()[0].uuid.as_deref(),
            Some("77777777-7777-7777-7777-777777777777")
        );
        assert_eq!(reparsed.symbols[0].pins[0].number.as_deref(), Some("1"));
        assert_eq!(
            reparsed.symbols[0].pins[0].uuid.as_deref(),
            Some("99999999-9999-9999-9999-999999999991")
        );
        assert_eq!(
            reparsed
                .library_symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        assert!(reparsed.canvas_scene().bounds.is_some());
        let netlist = reparsed.to_spice_netlist().unwrap();
        assert!(netlist.contains("R1 in out 1k"));
        assert!(netlist.contains("C1 out 0 100n"));
    }

    #[test]
    fn edits_kicad_schematic_in_rust_ir_and_roundtrips() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::MoveSymbol {
                reference: "R1".to_string(),
                to: KicadPoint { x: 73.66, y: 50.8 },
                rotation: Some(0.0),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::SetSymbolProperty {
                reference: "R1".to_string(),
                name: "Value".to_string(),
                value: "2k".to_string(),
                at: None,
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddWire {
                points: vec![
                    KicadPoint { x: 73.66, y: 45.72 },
                    KicadPoint { x: 88.9, y: 45.72 },
                ],
                uuid: Some("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddBus {
                points: vec![
                    KicadPoint { x: 88.9, y: 38.1 },
                    KicadPoint { x: 101.6, y: 38.1 },
                ],
                uuid: Some("33333333-aaaa-bbbb-cccc-333333333333".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddBusEntry {
                at: KicadPoint { x: 101.6, y: 38.1 },
                size: KicadSize {
                    width: 2.54,
                    height: -2.54,
                },
                uuid: Some("44444444-aaaa-bbbb-cccc-444444444444".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddJunction {
                at: KicadPoint { x: 88.9, y: 45.72 },
                uuid: Some("11111111-aaaa-bbbb-cccc-111111111111".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddNoConnect {
                at: KicadPoint { x: 101.6, y: 45.72 },
                uuid: Some("22222222-aaaa-bbbb-cccc-222222222222".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddLabel {
                text: "sense".to_string(),
                kind: KicadLabelKind::Global,
                at: KicadAt {
                    x: 88.9,
                    y: 45.72,
                    rotation: 0.0,
                },
                uuid: Some("ffffffff-ffff-ffff-ffff-ffffffffffff".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddText {
                text: ".save v(sense)".to_string(),
                at: KicadAt {
                    x: 45.72,
                    y: 35.56,
                    rotation: 0.0,
                },
                uuid: Some("abababab-abab-abab-abab-abababababab".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::AddSheet {
                name: "gain_stage".to_string(),
                file: "gain_stage.kicad_sch".to_string(),
                at: KicadAt {
                    x: 101.6,
                    y: 43.18,
                    rotation: 0.0,
                },
                size: KicadSize {
                    width: 25.4,
                    height: 12.7,
                },
                pins: vec![
                    KicadSheetPin {
                        name: "in".to_string(),
                        pin_type: "input".to_string(),
                        at: Some(KicadAt {
                            x: 101.6,
                            y: 48.26,
                            rotation: 180.0,
                        }),
                        uuid: None,
                        effects: None,
                    },
                    KicadSheetPin {
                        name: "out".to_string(),
                        pin_type: "output".to_string(),
                        at: Some(KicadAt {
                            x: 127.0,
                            y: 48.26,
                            rotation: 0.0,
                        }),
                        uuid: None,
                        effects: None,
                    },
                ],
                uuid: Some("cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd".to_string()),
            })
            .unwrap();

        let resistor = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("R1"))
            .unwrap();
        assert_close(resistor.at.unwrap().x, 73.66);
        assert_close(
            resistor
                .properties
                .iter()
                .find(|property| property.name == "Reference")
                .unwrap()
                .at
                .unwrap()
                .x,
            73.66,
        );
        assert_eq!(resistor.value(), Some("2k"));
        assert_eq!(schematic.wires.len(), 4);
        assert_eq!(schematic.buses.len(), 1);
        assert_eq!(schematic.bus_entries.len(), 1);
        assert_eq!(schematic.junctions.len(), 1);
        assert_eq!(schematic.no_connects.len(), 1);
        assert_eq!(schematic.sheets.len(), 1);
        assert_eq!(schematic.sheets[0].sheet_name(), Some("gain_stage"));
        assert_eq!(schematic.sheets[0].pins.len(), 2);
        assert!(schematic.labels.iter().any(|label| {
            label.text == "sense"
                && label.kind == KicadLabelKind::Global
                && label.uuid.as_deref() == Some("ffffffff-ffff-ffff-ffff-ffffffffffff")
        }));
        assert!(
            schematic
                .spice_directives()
                .iter()
                .any(|directive| directive.text == ".save v(sense)")
        );

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(bus"));
        assert!(exported.contains("(uuid \"33333333-aaaa-bbbb-cccc-333333333333\")"));
        assert!(exported.contains("(bus_entry"));
        assert!(exported.contains("(uuid \"44444444-aaaa-bbbb-cccc-444444444444\")"));
        assert!(exported.contains("(junction"));
        assert!(exported.contains("(uuid \"11111111-aaaa-bbbb-cccc-111111111111\")"));
        assert!(exported.contains("(no_connect"));
        assert!(exported.contains("(uuid \"22222222-aaaa-bbbb-cccc-222222222222\")"));
        assert!(exported.contains("(global_label \"sense\""));
        assert!(exported.contains("(sheet"));
        assert!(exported.contains("(property \"Sheetname\" \"gain_stage\""));
        assert!(exported.contains("(pin \"in\" input"));
        assert!(exported.contains("(text \".save v(sense)\""));
        let reparsed = parse_kicad_schematic(&exported, "edited.kicad_sch").unwrap();
        assert_eq!(reparsed.wires.len(), 4);
        assert_eq!(reparsed.buses.len(), 1);
        assert_eq!(
            reparsed.buses[0].uuid.as_deref(),
            Some("33333333-aaaa-bbbb-cccc-333333333333")
        );
        assert_eq!(reparsed.bus_entries.len(), 1);
        assert_eq!(
            reparsed.bus_entries[0].uuid.as_deref(),
            Some("44444444-aaaa-bbbb-cccc-444444444444")
        );
        assert_eq!(reparsed.junctions.len(), 1);
        assert_eq!(
            reparsed.junctions[0].uuid.as_deref(),
            Some("11111111-aaaa-bbbb-cccc-111111111111")
        );
        assert_eq!(reparsed.no_connects.len(), 1);
        assert_eq!(
            reparsed.no_connects[0].uuid.as_deref(),
            Some("22222222-aaaa-bbbb-cccc-222222222222")
        );
        assert_eq!(reparsed.sheets.len(), 1);
        assert_eq!(reparsed.sheets[0].pins.len(), 2);
        assert_eq!(reparsed.canvas_scene().buses.len(), 1);
        assert_eq!(reparsed.canvas_scene().bus_entries.len(), 1);
        assert_eq!(reparsed.canvas_scene().junctions.len(), 1);
        assert_eq!(reparsed.canvas_scene().no_connects.len(), 1);
        assert_eq!(
            reparsed
                .symbols
                .iter()
                .find(|symbol| symbol.reference() == Some("R1"))
                .unwrap()
                .value(),
            Some("2k")
        );
        assert!(
            reparsed
                .spice_directives()
                .iter()
                .any(|directive| directive.text == ".save v(sense)")
        );
    }

    #[test]
    fn deletes_kicad_schematic_items_by_uuid_and_roundtrips() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: "22222222-2222-2222-2222-222222222222".to_string(),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: "66666666-6666-6666-6666-666666666666".to_string(),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: "77777777-7777-7777-7777-777777777777".to_string(),
            })
            .unwrap();

        assert_eq!(schematic.wires.len(), 2);
        assert_eq!(schematic.labels.len(), 2);
        assert!(schematic.spice_directives().is_empty());

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(!exported.contains("22222222-2222-2222-2222-222222222222"));
        assert!(!exported.contains("66666666-6666-6666-6666-666666666666"));
        assert!(!exported.contains("77777777-7777-7777-7777-777777777777"));
        let reparsed = parse_kicad_schematic(&exported, "deleted_items.kicad_sch").unwrap();
        assert_eq!(reparsed.wires.len(), 2);
        assert_eq!(reparsed.labels.len(), 2);
        assert!(reparsed.canvas_scene().text_items.is_empty());

        let error = schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: "00000000-0000-4000-8000-000000000000".to_string(),
            })
            .unwrap_err();
        assert!(error.to_string().contains("was not found"));
    }

    #[test]
    fn edits_kicad_table_cells_by_uuid_and_roundtrips() {
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (table
    (column_count 2)
    (uuid "67676767-6767-4767-8767-676767676767")
    (column_widths 20 20)
    (row_heights 5)
    (cells
      (table_cell "Move me"
        (at 10 10 45)
        (size 20 5)
        (uuid "68686868-6868-4868-8868-686868686868")
      )
      (table_cell "Delete me"
        (at 30 10 0)
        (size 20 5)
        (uuid "69696969-6969-4969-8969-696969696969")
      )
    )
  )
)"#,
            "table_cell_edits.kicad_sch",
        )
        .unwrap();

        let move_summary = schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: "68686868-6868-4868-8868-686868686868".to_string(),
                delta: KicadPoint { x: 2.54, y: -1.27 },
            })
            .unwrap();
        assert_eq!(move_summary.operation, "move-table-cell");
        assert_close(schematic.tables[0].cells[0].at.unwrap().x, 12.54);
        assert_close(schematic.tables[0].cells[0].at.unwrap().y, 8.73);
        assert_close(schematic.tables[0].cells[1].at.unwrap().x, 30.0);

        let delete_summary = schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: "69696969-6969-4969-8969-696969696969".to_string(),
            })
            .unwrap();
        assert_eq!(delete_summary.operation, "delete-table-cell");
        assert_eq!(schematic.tables.len(), 1);
        assert_eq!(schematic.tables[0].cells.len(), 1);
        assert_eq!(schematic.tables[0].cells[0].text, "Move me");

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(table"));
        assert!(exported.contains("(table_cell \"Move me\""));
        assert!(exported.contains("(at 12.54 8.73 45)"));
        assert!(!exported.contains("Delete me"));
        assert!(!exported.contains("69696969-6969-4969-8969-696969696969"));
        let reparsed =
            parse_kicad_schematic(&exported, "table_cell_edits_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.tables.len(), 1);
        assert_eq!(reparsed.tables[0].cells.len(), 1);
        assert_eq!(
            reparsed.tables[0].cells[0].uuid.as_deref(),
            Some("68686868-6868-4868-8868-686868686868")
        );
        assert_close(reparsed.tables[0].cells[0].at.unwrap().x, 12.54);
        assert_eq!(reparsed.canvas_scene().tables[0].cells.len(), 1);
    }

    #[test]
    fn edits_kicad_sheet_pins_by_uuid_and_roundtrips() {
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
  (sheet
    (at 50 40)
    (size 30 20)
    (property "Sheetname" "gain_stage" (id 0) (at 50 38 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (id 1) (at 50 62 0))
    (pin "in" input (at 50 45 180) (uuid "11111111-1111-4111-8111-111111111111"))
    (pin "out" output (at 80 45 0) (uuid "22222222-2222-4222-8222-222222222222"))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
            "sheet_pin_edits.kicad_sch",
        )
        .unwrap();

        let move_summary = schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: "11111111-1111-4111-8111-111111111111".to_string(),
                delta: KicadPoint { x: 2.54, y: -1.27 },
            })
            .unwrap();
        assert_eq!(move_summary.operation, "move-sheet-pin");
        assert_close(schematic.sheets[0].pins[0].at.unwrap().x, 52.54);
        assert_close(schematic.sheets[0].pins[0].at.unwrap().y, 43.73);
        assert_close(schematic.sheets[0].at.unwrap().x, 50.0);

        let delete_summary = schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: "22222222-2222-4222-8222-222222222222".to_string(),
            })
            .unwrap();
        assert_eq!(delete_summary.operation, "delete-sheet-pin");
        assert_eq!(schematic.sheets.len(), 1);
        assert_eq!(schematic.sheets[0].pins.len(), 1);
        assert_eq!(schematic.sheets[0].pins[0].name, "in");

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(sheet"));
        assert!(exported.contains("(pin \"in\" input (at 52.54 43.73 180)"));
        assert!(!exported.contains("pin \"out\""));
        assert!(!exported.contains("22222222-2222-4222-8222-222222222222"));
        let reparsed =
            parse_kicad_schematic(&exported, "sheet_pin_edits_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.sheets.len(), 1);
        assert_eq!(reparsed.sheets[0].pins.len(), 1);
        assert_eq!(
            reparsed.sheets[0].pins[0].uuid.as_deref(),
            Some("11111111-1111-4111-8111-111111111111")
        );
        assert_close(reparsed.sheets[0].pins[0].at.unwrap().x, 52.54);
        assert_eq!(reparsed.canvas_scene().sheets[0].pins.len(), 1);
    }

    #[test]
    fn moves_kicad_schematic_items_by_uuid_and_roundtrips() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::AddSheet {
                name: "gain_stage".to_string(),
                file: "gain_stage.kicad_sch".to_string(),
                at: KicadAt {
                    x: 101.6,
                    y: 43.18,
                    rotation: 0.0,
                },
                size: KicadSize {
                    width: 25.4,
                    height: 12.7,
                },
                pins: vec![KicadSheetPin {
                    name: "in".to_string(),
                    pin_type: "input".to_string(),
                    at: Some(KicadAt {
                        x: 101.6,
                        y: 48.26,
                        rotation: 180.0,
                    }),
                    uuid: None,
                    effects: None,
                }],
                uuid: Some("cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd".to_string()),
            })
            .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa".to_string(),
                delta: KicadPoint { x: 2.54, y: -1.27 },
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: "22222222-2222-2222-2222-222222222222".to_string(),
                delta: KicadPoint { x: 1.27, y: 2.54 },
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: "66666666-6666-6666-6666-666666666666".to_string(),
                delta: KicadPoint { x: -2.54, y: 1.27 },
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: "cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd".to_string(),
                delta: KicadPoint { x: 5.08, y: 2.54 },
            })
            .unwrap();

        let resistor = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.uuid.as_deref() == Some("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"))
            .unwrap();
        assert_close(resistor.at.unwrap().x, 72.39);
        assert_close(resistor.at.unwrap().y, 49.53);
        assert_close(
            resistor
                .properties
                .iter()
                .find(|property| property.name == "Reference")
                .unwrap()
                .at
                .unwrap()
                .x,
            72.39,
        );
        assert_close(schematic.wires[0].points[0].x, 52.07);
        assert_close(schematic.wires[0].points[0].y, 53.34);
        assert_close(
            schematic
                .labels
                .iter()
                .find(|label| label.uuid.as_deref() == Some("66666666-6666-6666-6666-666666666666"))
                .unwrap()
                .at
                .unwrap()
                .x,
            86.36,
        );
        assert_close(schematic.sheets[0].at.unwrap().x, 106.68);
        assert_close(schematic.sheets[0].pins[0].at.unwrap().x, 106.68);

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(at 72.39 49.53 0)"));
        assert!(exported.contains("(xy 52.07 53.34)"));
        assert!(exported.contains("(at 86.36 52.07 0)"));
        assert!(exported.contains("(at 106.68 45.72)"));
        let reparsed = parse_kicad_schematic(&exported, "moved_items.kicad_sch").unwrap();
        let scene = reparsed.canvas_scene();
        assert_close(scene.symbols[1].at.x, 72.39);
        assert_close(scene.wires[0].points[0].x, 52.07);
        assert_close(scene.labels[1].at.unwrap().x, 86.36);
        assert_close(scene.sheets[0].at.unwrap().x, 106.68);
        assert_close(scene.sheets[0].pins[0].at.unwrap().x, 106.68);

        let error = schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: "00000000-0000-4000-8000-000000000000".to_string(),
                delta: KicadPoint { x: 1.0, y: 1.0 },
            })
            .unwrap_err();
        assert!(error.to_string().contains("was not found"));
    }

    #[test]
    fn places_symbol_from_kicad_library_into_schematic_ir() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();
        let library = read_kicad_symbol_library(
            &workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
        )
        .unwrap();
        let capacitor = library.symbol("NekoSpice:C").unwrap().clone();

        schematic
            .apply_edit(KicadSchematicEdit::PlaceSymbol {
                definition: Box::new(capacitor),
                library_symbols: library.symbols.clone(),
                reference: "C2".to_string(),
                value: "47n".to_string(),
                at: KicadAt {
                    x: 101.6,
                    y: 53.34,
                    rotation: 0.0,
                },
                unit: Some(1),
                body_style: None,
                pin_alternates: BTreeMap::new(),
                uuid: Some("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee".to_string()),
            })
            .unwrap();

        let placed = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("C2"))
            .unwrap();
        assert_eq!(placed.lib_id, "NekoSpice:C");
        assert_eq!(placed.value(), Some("47n"));
        assert_eq!(placed.pins.len(), 2);
        assert!(placed.pins.iter().all(|pin| pin.uuid.is_some()));
        assert!(
            schematic
                .library_symbols
                .iter()
                .any(|symbol| symbol.name == "NekoSpice:C")
        );

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(property \"Reference\" \"C2\""));
        assert!(exported.contains("(property \"Value\" \"47n\""));
        let reparsed = parse_kicad_schematic(&exported, "placed.kicad_sch").unwrap();
        assert!(
            reparsed
                .canvas_scene()
                .symbols
                .iter()
                .any(|symbol| symbol.reference == "C2" && symbol.pins.len() == 2)
        );
    }

    #[test]
    fn places_derived_symbol_with_parent_library_context() {
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
)"#,
            "empty_derived_placement.kicad_sch",
        )
        .unwrap();
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (symbol "NekoSpice:ParentR"
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "1k" (at 0 -2.54 0))
    (symbol "ParentR_0_1"
      (rectangle (start -1 -1) (end 1 1) (stroke (width 0.127) (type default)) (fill (type none)))
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
    )
  )
  (symbol "NekoSpice:DerivedR"
    (extends "NekoSpice:ParentR")
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "2.2k" (at 0 -2.54 0))
  )
)"#,
            "derived_placement.kicad_sym",
        )
        .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::PlaceSymbol {
                definition: Box::new(library.symbol("NekoSpice:DerivedR").unwrap().clone()),
                library_symbols: library.symbols.clone(),
                reference: "R1".to_string(),
                value: "2.2k".to_string(),
                at: KicadAt {
                    x: 10.0,
                    y: 10.0,
                    rotation: 0.0,
                },
                unit: Some(1),
                body_style: None,
                pin_alternates: BTreeMap::new(),
                uuid: Some("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".to_string()),
            })
            .unwrap();

        assert!(
            schematic
                .library_symbols
                .iter()
                .any(|symbol| symbol.name == "NekoSpice:ParentR")
        );
        assert!(
            schematic
                .library_symbols
                .iter()
                .any(|symbol| symbol.name == "NekoSpice:DerivedR")
        );
        let placed = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("R1"))
            .unwrap();
        assert_eq!(placed.pins.len(), 2);
        assert_eq!(schematic.canvas_scene().symbols[0].graphics.len(), 1);

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(symbol \"NekoSpice:ParentR\""));
        assert!(exported.contains("(symbol \"NekoSpice:DerivedR\""));
        assert!(exported.contains("(extends \"NekoSpice:ParentR\")"));
        assert!(!exported.contains("(symbol \"DerivedR_0_1\""));
    }

    #[test]
    fn places_symbol_when_embedded_library_has_explicit_default_property_effects() {
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:R"
      (property "Reference" "R" (at 0 0 0)
        (effects (font (size 1.27 1.27)))
      )
      (property "Value" "1k" (at 0 -2.54 0)
        (effects (font (size 1.27 1.27)))
      )
      (symbol "R_0_1"
        (rectangle (start -1.27 -1.27) (end 1.27 1.27) (stroke (width 0.254) (type default)) (fill (type none)))
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))
      )
    )
  )
)"#,
            "explicit_default_effects.kicad_sch",
        )
        .unwrap();
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (symbol "NekoSpice:R"
    (property "Reference" "R" (at 0 0 0))
    (property "Value" "1k" (at 0 -2.54 0))
    (symbol "R_0_1"
      (rectangle (start -1.27 -1.27) (end 1.27 1.27) (stroke (width 0.254) (type default)) (fill (type none)))
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))
    )
  )
)"#,
            "implicit_default_effects.kicad_sym",
        )
        .unwrap();

        let summary = schematic
            .apply_edit(KicadSchematicEdit::PlaceSymbol {
                definition: Box::new(library.symbol("NekoSpice:R").unwrap().clone()),
                library_symbols: library.symbols.clone(),
                reference: "R1".to_string(),
                value: "1k".to_string(),
                at: KicadAt {
                    x: 10.0,
                    y: 10.0,
                    rotation: 0.0,
                },
                unit: Some(1),
                body_style: None,
                pin_alternates: BTreeMap::new(),
                uuid: Some("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string()),
            })
            .unwrap();

        assert_eq!(summary.operation, "place-symbol");
        assert_eq!(schematic.library_symbols.len(), 1);
        assert_eq!(schematic.symbols[0].reference(), Some("R1"));
    }

    #[test]
    fn places_selected_kicad_symbol_unit_and_body_style() {
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols)
)"#,
            "empty.kicad_sch",
        )
        .unwrap();
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (symbol "NekoSpice:Scoped"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Scoped" (at 0 -2.54 0))
    (symbol "Scoped_1_1"
      (pin passive line (at -2.54 0 0) (length 2.54) (name "A1") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "B1") (number "2"))
    )
    (symbol "Scoped_2_2"
      (unit_name "Analog")
      (pin passive line (at -2.54 0 0) (length 2.54) (name "A2") (number "3"))
      (pin passive line
        (at 2.54 0 180)
        (length 2.54)
        (name "B2")
        (number "4")
        (alternate "ALT4" output line)
      )
    )
  )
)"#,
            "scoped.kicad_sym",
        )
        .unwrap();
        let definition = library.symbol("NekoSpice:Scoped").unwrap().clone();

        schematic
            .apply_edit(KicadSchematicEdit::PlaceSymbol {
                definition: Box::new(definition),
                library_symbols: library.symbols.clone(),
                reference: "U2".to_string(),
                value: "Scoped".to_string(),
                at: KicadAt {
                    x: 20.0,
                    y: 10.0,
                    rotation: 0.0,
                },
                unit: Some(2),
                body_style: Some(2),
                pin_alternates: BTreeMap::from([("4".to_string(), "ALT4".to_string())]),
                uuid: Some("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_string()),
            })
            .unwrap();

        let placed = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("U2"))
            .unwrap();
        assert_eq!(placed.unit, Some(2));
        assert_eq!(placed.body_style, Some(2));
        assert_eq!(placed.pins[1].alternate.as_deref(), Some("ALT4"));
        assert_eq!(
            placed
                .pins
                .iter()
                .filter_map(|pin| pin.number.as_deref())
                .collect::<Vec<_>>(),
            vec!["3", "4"]
        );

        let scene = schematic.canvas_scene();
        assert_eq!(scene.symbols[0].unit_name.as_deref(), Some("Analog"));
        assert_eq!(
            scene.symbols[0]
                .pins
                .iter()
                .map(|pin| pin.number.as_str())
                .collect::<Vec<_>>(),
            vec!["3", "4"]
        );

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(unit 2)"));
        assert!(exported.contains("(body_style 2)"));
        assert!(exported.contains("(pin \"3\""));
        assert!(exported.contains("(alternate \"ALT4\")"));
        assert!(!exported.contains("(pin \"1\""));
        let reparsed = parse_kicad_schematic(&exported, "placed_scoped.kicad_sch").unwrap();
        assert_eq!(reparsed.symbols[0].unit, Some(2));
        assert_eq!(reparsed.symbols[0].body_style, Some(2));
        assert_eq!(
            reparsed.symbols[0].pins[1].alternate.as_deref(),
            Some("ALT4")
        );
        assert_eq!(reparsed.canvas_scene().symbols[0].pins.len(), 2);

        let definition = schematic
            .symbol_definition("NekoSpice:Scoped")
            .unwrap()
            .clone();
        let error = schematic
            .apply_edit(KicadSchematicEdit::PlaceSymbol {
                definition: Box::new(definition),
                library_symbols: Vec::new(),
                reference: "U3".to_string(),
                value: "Scoped".to_string(),
                at: KicadAt {
                    x: 30.0,
                    y: 10.0,
                    rotation: 0.0,
                },
                unit: Some(2),
                body_style: Some(2),
                pin_alternates: BTreeMap::from([("4".to_string(), "MISSING".to_string())]),
                uuid: None,
            })
            .unwrap_err();
        assert!(error.to_string().contains("has no alternate 'MISSING'"));
    }

    #[test]
    fn exposes_kicad_canvas_item_uuids_for_editor_selection() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let scene = schematic.canvas_scene();
        assert_eq!(
            scene.symbols[0].uuid.as_deref(),
            Some("88888888-8888-8888-8888-888888888888")
        );
        assert_eq!(
            scene.wires[0].uuid.as_deref(),
            Some("22222222-2222-2222-2222-222222222222")
        );
        assert_eq!(
            scene.labels[1].uuid.as_deref(),
            Some("66666666-6666-6666-6666-666666666666")
        );
        assert_eq!(
            scene.text_items[0].uuid.as_deref(),
            Some("77777777-7777-7777-7777-777777777777")
        );

        let scene_json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(
            scene_json["symbols"][0]["uuid"],
            "88888888-8888-8888-8888-888888888888"
        );
        assert!(
            scene_json["symbols"][0]["bounds"]["width"]
                .as_f64()
                .unwrap()
                > 0.0
        );
        assert_eq!(
            scene_json["wires"][0]["uuid"],
            "22222222-2222-2222-2222-222222222222"
        );
        assert!(scene_json["wires"][0]["bounds"]["width"].as_f64().unwrap() > 16.0);
        assert_eq!(
            scene_json["labels"][1]["uuid"],
            "66666666-6666-6666-6666-666666666666"
        );
        assert!(
            scene_json["labels"][1]["bounds"]["width"].as_f64().unwrap()
                >= super::KICAD_CANVAS_POINT_BOUNDS_RADIUS * 2.0
        );
        assert_eq!(
            scene_json["text_items"][0]["uuid"],
            "77777777-7777-7777-7777-777777777777"
        );
        assert!(
            scene_json["text_items"][0]["bounds"]["height"]
                .as_f64()
                .unwrap()
                >= super::KICAD_CANVAS_POINT_BOUNDS_RADIUS * 2.0
        );
    }

    #[test]
    fn finds_kicad_canvas_items_by_uuid_for_editor_state() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();
        let scene = schematic.canvas_scene();

        let wire_hit = scene
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert_eq!(wire_hit.kind, "wire");
        assert_eq!(wire_hit.label, "wire");
        assert!(wire_hit.bounds.width() > 16.0);

        let source_hit = scene
            .item_hit_by_uuid("88888888-8888-8888-8888-888888888888")
            .unwrap();
        assert_eq!(source_hit.kind, "symbol");
        assert_eq!(source_hit.label, "V1");

        let resistor_hit = scene
            .item_hit_by_uuid("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
            .unwrap();
        assert_eq!(resistor_hit.kind, "symbol");
        assert_eq!(resistor_hit.label, "R1");

        assert!(
            scene
                .item_hit_by_uuid("00000000-0000-4000-8000-000000000000")
                .is_none()
        );
    }

    #[test]
    fn hit_tests_kicad_canvas_items_by_bounds() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let hit_report = schematic
            .canvas_scene()
            .hit_test(KicadPoint { x: 88.9, y: 50.8 });

        assert!(hit_report.hit_count >= 2);
        assert_eq!(hit_report.hits[0].kind, "label");
        assert_eq!(
            hit_report.hits[0].uuid.as_deref(),
            Some("66666666-6666-6666-6666-666666666666")
        );
        assert!(hit_report.hits.iter().any(|hit| hit.kind == "wire"
            && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));
        let json: serde_json::Value = serde_json::from_str(&hit_report.to_json()).unwrap();
        assert_eq!(
            json["hit_count"].as_u64().unwrap(),
            hit_report.hit_count as u64
        );
        assert_eq!(json["hits"][0]["kind"], "label");
        assert_eq!(
            json["hits"][0]["uuid"],
            "66666666-6666-6666-6666-666666666666"
        );
        assert!(json["hits"][0]["bounds"]["width"].as_f64().unwrap() > 0.0);

        let empty_report = schematic
            .canvas_scene()
            .hit_test(KicadPoint { x: 10.0, y: 10.0 });
        assert_eq!(empty_report.hit_count, 0);
        assert!(empty_report.hits.is_empty());
    }

    #[test]
    fn hit_tests_symbols_by_body_and_pin_geometry() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-4111-8111-111111111111")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Sparse"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Sparse" (at 0 -2.54 0))
      (symbol "Sparse_0_1"
        (polyline (pts (xy -2.54 0) (xy 2.54 0)))
        (pin passive line (at -5.08 0 0) (length 2.54) (name "A") (number "1"))
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:Sparse")
    (at 20 20 0)
    (property "Reference" "U1" (at 20 17 0))
    (property "Value" "Sparse" (at 20 23 0))
    (uuid "22222222-2222-4222-8222-222222222222")
    (pin "1" (uuid "33333333-3333-4333-8333-333333333333"))
  )
)"#,
            "symbol_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        let symbol = &scene.symbols[0];
        assert!(symbol.bounds.unwrap().height() >= super::KICAD_CANVAS_LINE_BOUNDS_PADDING * 2.0);

        let body_hit = scene.hit_test(KicadPoint { x: 20.0, y: 20.4 });
        assert!(body_hit.hits.iter().any(|hit| hit.kind == "symbol"
            && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

        let pin_hit = scene.hit_test(KicadPoint { x: 16.2, y: 20.4 });
        assert!(pin_hit.hits.iter().any(|hit| hit.kind == "symbol"
            && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

        let bounds_only_miss = scene.hit_test(KicadPoint { x: 17.0, y: 20.7 });
        assert!(!bounds_only_miss.hits.iter().any(|hit| hit.kind == "symbol"
            && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));
    }

    #[test]
    fn hit_tests_line_items_by_segment_distance() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (wire (pts (xy 10 10) (xy 30 10)) (stroke (width 0) (type default)) (uuid "22222222-2222-2222-2222-222222222222"))
  (bus (pts (xy 10 20) (xy 30 20)) (stroke (width 0) (type default)) (uuid "33333333-3333-3333-3333-333333333333"))
  (bus_entry (at 30 20) (size 2.54 -2.54) (stroke (width 0) (type default)) (uuid "44444444-4444-4444-4444-444444444444"))
)"#,
            "line_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();

        let wire_hit = scene.hit_test(KicadPoint { x: 20.0, y: 10.4 });
        assert!(wire_hit.hits.iter().any(|hit| hit.kind == "wire"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

        let wire_miss_inside_bounds = scene.hit_test(KicadPoint { x: 20.0, y: 10.7 });
        assert!(
            !wire_miss_inside_bounds
                .hits
                .iter()
                .any(|hit| hit.kind == "wire"
                    && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
        );

        let bus_hit = scene.hit_test(KicadPoint { x: 20.0, y: 20.4 });
        assert!(bus_hit.hits.iter().any(|hit| hit.kind == "bus"
            && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));

        let entry_hit = scene.hit_test(KicadPoint { x: 31.27, y: 18.73 });
        assert!(entry_hit.hits.iter().any(|hit| hit.kind == "bus-entry"
            && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444")));
    }

    #[test]
    fn hit_tests_junctions_and_no_connects_by_shape() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-4111-8111-111111111111")
  (paper "A4")
  (lib_symbols)
  (junction (at 10 10) (diameter 2.54) (uuid "22222222-2222-4222-8222-222222222222"))
  (no_connect (at 20 10) (uuid "33333333-3333-4333-8333-333333333333"))
)"#,
            "point_shape_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        assert!(scene.junctions[0].bounds.width() > 2.5);
        assert!(scene.no_connects[0].bounds.width() > super::KICAD_CANVAS_POINT_BOUNDS_RADIUS);

        let junction_hit = scene.hit_test(KicadPoint { x: 11.0, y: 10.0 });
        assert!(junction_hit.hits.iter().any(|hit| hit.kind == "junction"
            && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

        let junction_corner_miss = scene.hit_test(KicadPoint { x: 10.95, y: 10.95 });
        assert!(
            !junction_corner_miss
                .hits
                .iter()
                .any(|hit| hit.kind == "junction"
                    && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222"))
        );

        let no_connect_hit = scene.hit_test(KicadPoint { x: 20.9, y: 10.9 });
        assert!(
            no_connect_hit
                .hits
                .iter()
                .any(|hit| hit.kind == "no-connect"
                    && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333"))
        );

        let no_connect_corner_miss = scene.hit_test(KicadPoint { x: 20.95, y: 10.0 });
        assert!(
            !no_connect_corner_miss
                .hits
                .iter()
                .any(|hit| hit.kind == "no-connect"
                    && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333"))
        );
    }

    #[test]
    fn hit_tests_sheet_pins_by_segment_distance() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
  (paper "A4")
  (lib_symbols)
  (sheet
    (at 50 40)
    (size 30 20)
    (property "Sheetname" "gain_stage" (id 0) (at 50 38 0))
    (property "Sheetfile" "gain_stage.kicad_sch" (id 1) (at 50 62 0))
    (pin "in" input (at 50 45 180) (uuid "11111111-1111-4111-8111-111111111111"))
    (uuid "33333333-3333-4333-8333-333333333333")
  )
)"#,
            "sheet_pin_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        let pin = &scene.sheets[0].pins[0];
        let pin_bounds = pin.bounds.unwrap();
        assert!(pin_bounds.width() > 2.54);
        assert!(pin_bounds.min.x < 48.0);

        let pin_hit = scene.hit_test(KicadPoint { x: 48.73, y: 45.0 });
        assert!(pin_hit.hits.iter().any(|hit| hit.kind == "sheet-pin"
            && hit.uuid.as_deref() == Some("11111111-1111-4111-8111-111111111111")));
        assert!(!pin_hit.hits.iter().any(|hit| hit.kind == "sheet"
            && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333")));

        let anchor_box_miss = scene.hit_test(KicadPoint { x: 50.0, y: 46.2 });
        assert!(
            !anchor_box_miss
                .hits
                .iter()
                .any(|hit| hit.kind == "sheet-pin"
                    && hit.uuid.as_deref() == Some("11111111-1111-4111-8111-111111111111"))
        );
        assert!(anchor_box_miss.hits.iter().any(|hit| hit.kind == "sheet"
            && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333")));

        let corner_miss = scene.hit_test(KicadPoint { x: 46.86, y: 45.62 });
        assert!(!corner_miss.hits.iter().any(|hit| hit.kind == "sheet-pin"
            && hit.uuid.as_deref() == Some("11111111-1111-4111-8111-111111111111")));
        assert!(!corner_miss.hits.iter().any(|hit| hit.kind == "sheet"
            && hit.uuid.as_deref() == Some("33333333-3333-4333-8333-333333333333")));
    }

    #[test]
    fn hit_tests_directive_labels_by_segment_and_text_bounds() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-4111-8111-111111111111")
  (paper "A4")
  (lib_symbols)
  (netclass_flag ""
    (length 3.81)
    (shape dot)
    (at 50 40 0)
    (effects (font (size 1.27 1.27)))
    (uuid "22222222-2222-4222-8222-222222222222")
    (property "Net Class" "HV" (at 50 38 0))
  )
)"#,
            "directive_label_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        let label = &scene.directive_labels[0];
        let bounds = label.bounds.unwrap();
        assert!(bounds.width() > 4.0);
        assert!(bounds.height() > 2.0);

        let segment_hit = scene.hit_test(KicadPoint { x: 52.0, y: 40.4 });
        assert!(
            segment_hit
                .hits
                .iter()
                .any(|hit| hit.kind == "directive-label"
                    && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222"))
        );

        let text_hit = scene.hit_test(KicadPoint { x: 51.0, y: 41.0 });
        assert!(text_hit.hits.iter().any(|hit| hit.kind == "directive-label"
            && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222")));

        let bounds_only_miss = scene.hit_test(KicadPoint { x: 54.0, y: 41.5 });
        assert!(
            !bounds_only_miss
                .hits
                .iter()
                .any(|hit| hit.kind == "directive-label"
                    && hit.uuid.as_deref() == Some("22222222-2222-4222-8222-222222222222"))
        );
    }

    #[test]
    fn hit_tests_text_items_by_estimated_text_bounds() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (label "LONG_LABEL" (at 10 10 0) (effects (font (size 1.27 1.27))) (uuid "22222222-2222-2222-2222-222222222222"))
  (text "First line\nSecond line" (at 10 20 0) (effects (font (size 1.27 1.27))) (uuid "33333333-3333-3333-3333-333333333333"))
  (text "RIGHT" (at 40 10 0) (effects (font (size 1.27 1.27)) (justify right)) (uuid "44444444-4444-4444-4444-444444444444"))
)"#,
            "text_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();

        let label_hit = scene.hit_test(KicadPoint { x: 16.0, y: 10.7 });
        assert!(label_hit.hits.iter().any(|hit| hit.kind == "label"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

        let label_miss = scene.hit_test(KicadPoint { x: 21.0, y: 10.7 });
        assert!(!label_miss.hits.iter().any(|hit| hit.kind == "label"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

        let multiline_hit = scene.hit_test(KicadPoint { x: 13.0, y: 22.5 });
        assert!(multiline_hit.hits.iter().any(|hit| hit.kind == "text"
            && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));

        let right_justified_hit = scene.hit_test(KicadPoint { x: 37.0, y: 10.7 });
        assert!(right_justified_hit.hits.iter().any(|hit| hit.kind == "text"
            && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444")));

        let right_justified_miss = scene.hit_test(KicadPoint { x: 42.0, y: 10.7 });
        assert!(
            !right_justified_miss
                .hits
                .iter()
                .any(|hit| hit.kind == "text"
                    && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444"))
        );
    }

    #[test]
    fn hit_tests_schematic_graphics_by_shape() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (rectangle (start 10 10) (end 20 20) (stroke (width 0) (type default)) (fill (type none)) (uuid "22222222-2222-2222-2222-222222222222"))
  (rectangle (start 30 10) (end 40 20) (stroke (width 0) (type default)) (fill (type color) (color 255 228 206 0.5)) (uuid "33333333-3333-3333-3333-333333333333"))
  (circle (center 55 15) (radius 5) (stroke (width 0) (type default)) (fill (type none)) (uuid "44444444-4444-4444-4444-444444444444"))
  (polyline (pts (xy 10 30) (xy 20 30) (xy 20 40)) (stroke (width 0) (type default)) (fill (type none)) (uuid "55555555-5555-5555-5555-555555555555"))
)"#,
            "graphic_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();

        let hollow_rectangle_center = scene.hit_test(KicadPoint { x: 15.0, y: 15.0 });
        assert!(
            !hollow_rectangle_center
                .hits
                .iter()
                .any(|hit| hit.kind == "graphic"
                    && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
        );

        let hollow_rectangle_edge = scene.hit_test(KicadPoint { x: 15.0, y: 10.4 });
        assert!(
            hollow_rectangle_edge
                .hits
                .iter()
                .any(|hit| hit.kind == "graphic"
                    && hit.label == "rectangle"
                    && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
        );

        let filled_rectangle_center = scene.hit_test(KicadPoint { x: 35.0, y: 15.0 });
        assert!(
            filled_rectangle_center
                .hits
                .iter()
                .any(|hit| hit.kind == "graphic"
                    && hit.label == "rectangle"
                    && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333"))
        );

        let hollow_circle_center = scene.hit_test(KicadPoint { x: 55.0, y: 15.0 });
        assert!(
            !hollow_circle_center
                .hits
                .iter()
                .any(|hit| hit.kind == "graphic"
                    && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444"))
        );

        let hollow_circle_edge = scene.hit_test(KicadPoint { x: 60.0, y: 15.0 });
        assert!(
            hollow_circle_edge
                .hits
                .iter()
                .any(|hit| hit.kind == "graphic"
                    && hit.label == "circle"
                    && hit.uuid.as_deref() == Some("44444444-4444-4444-4444-444444444444"))
        );

        let polyline_hit = scene.hit_test(KicadPoint { x: 20.0, y: 35.0 });
        assert!(polyline_hit.hits.iter().any(|hit| hit.kind == "graphic"
            && hit.label == "polyline"
            && hit.uuid.as_deref() == Some("55555555-5555-5555-5555-555555555555")));
    }

    #[test]
    fn hit_tests_bezier_graphics_by_sampled_curve() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (bezier
    (pts (xy 10 20) (xy 10 10) (xy 30 10) (xy 30 20))
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "22222222-2222-2222-2222-222222222222")
  )
)"#,
            "bezier_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();

        let curve_hit = scene.hit_test(KicadPoint { x: 20.0, y: 12.5 });
        assert!(curve_hit.hits.iter().any(|hit| hit.kind == "graphic"
            && hit.label == "bezier"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

        let control_polygon_miss = scene.hit_test(KicadPoint { x: 20.0, y: 10.0 });
        assert!(
            !control_polygon_miss
                .hits
                .iter()
                .any(|hit| hit.kind == "graphic"
                    && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222"))
        );
    }

    #[test]
    fn hit_tests_arc_graphics_by_sampled_curve() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (arc
    (start 10 20)
    (mid 20 10)
    (end 30 20)
    (stroke (width 0) (type default))
    (fill (type none))
    (uuid "22222222-2222-2222-2222-222222222222")
  )
)"#,
            "arc_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();

        let curve_hit = scene.hit_test(KicadPoint { x: 20.0, y: 10.0 });
        assert!(curve_hit.hits.iter().any(|hit| hit.kind == "graphic"
            && hit.label == "arc"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

        let chord_miss = scene.hit_test(KicadPoint { x: 15.0, y: 15.0 });
        assert!(!chord_miss.hits.iter().any(|hit| hit.kind == "graphic"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));
    }

    #[test]
    fn hit_tests_rule_areas_by_polygon_shape() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (uuid "11111111-1111-1111-1111-111111111111")
  (paper "A4")
  (rule_area
    (polyline
      (pts (xy 10 10) (xy 20 10) (xy 20 20) (xy 10 20))
      (stroke (width 0) (type default))
      (fill (type none))
      (uuid "22222222-2222-2222-2222-222222222222")
    )
  )
  (rule_area
    (polyline
      (pts (xy 30 10) (xy 40 10) (xy 40 20) (xy 30 20))
      (stroke (width 0) (type default))
      (fill (type color) (color 20 200 170 0.25))
      (uuid "33333333-3333-3333-3333-333333333333")
    )
  )
)"#,
            "rule_area_hit_test.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();

        let hollow_center = scene.hit_test(KicadPoint { x: 15.0, y: 15.0 });
        assert!(!hollow_center.hits.iter().any(|hit| hit.kind == "rule-area"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

        let hollow_edge = scene.hit_test(KicadPoint { x: 15.0, y: 10.4 });
        assert!(hollow_edge.hits.iter().any(|hit| hit.kind == "rule-area"
            && hit.uuid.as_deref() == Some("22222222-2222-2222-2222-222222222222")));

        let filled_center = scene.hit_test(KicadPoint { x: 35.0, y: 15.0 });
        assert!(filled_center.hits.iter().any(|hit| hit.kind == "rule-area"
            && hit.uuid.as_deref() == Some("33333333-3333-3333-3333-333333333333")));
    }

    #[test]
    fn checks_no_connect_markers_against_selected_symbol_scope() {
        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Scoped"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Scoped" (at 0 -2.54 0))
      (symbol "Scoped_1_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A1") (number "1"))
      )
      (symbol "Scoped_2_1"
        (pin passive line (at 2.54 0 180) (length 2.54) (name "A2") (number "2"))
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:Scoped")
    (at 20 10 0)
    (unit 2)
    (uuid "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
    (property "Reference" "U2" (at 20 7.46 0))
    (property "Value" "Scoped" (at 20 12.54 0))
    (pin "2" (uuid "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb2"))
  )
  (no_connect (at 22.54 10) (uuid "cccccccc-cccc-4ccc-8ccc-ccccccccccc1"))
  (no_connect (at 17.46 10) (uuid "cccccccc-cccc-4ccc-8ccc-ccccccccccc2"))
)"#,
            "scoped_no_connect.kicad_sch",
        )
        .unwrap();

        let report = schematic.check_report();
        let floating = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "floating-no-connect")
            .collect::<Vec<_>>();
        assert_eq!(floating.len(), 1);
        assert!(floating[0].message.contains("17.46,10"));
    }

    #[test]
    fn configures_existing_symbol_scope_mirror_and_pin_alternates() {
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Scoped"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Scoped" (at 0 -2.54 0))
      (symbol "Scoped_1_1"
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A1") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "B1") (number "2"))
      )
      (symbol "Scoped_2_2"
        (unit_name "Analog")
        (pin passive line (at -2.54 0 0) (length 2.54) (name "A2") (number "3"))
        (pin passive line
          (at 2.54 0 180)
          (length 2.54)
          (name "B2")
          (number "4")
          (alternate "ALT4" output line)
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:Scoped")
    (at 20 10 0)
    (unit 1)
    (uuid "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa")
    (property "Reference" "U2" (at 20 7.46 0))
    (property "Value" "Scoped" (at 20 12.54 0))
    (pin "1" (uuid "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1"))
    (pin "2" (uuid "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb2"))
  )
)"#,
            "configure_symbol.kicad_sch",
        )
        .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::ConfigureSymbol {
                reference: "U2".to_string(),
                unit: Some(2),
                body_style: Some(Some(2)),
                mirror: Some(Some("x y".to_string())),
                pin_alternates: Some(BTreeMap::from([("4".to_string(), "ALT4".to_string())])),
            })
            .unwrap();

        let symbol = schematic.symbols[0].clone();
        assert_eq!(symbol.unit, Some(2));
        assert_eq!(symbol.body_style, Some(2));
        assert_eq!(symbol.mirror.as_deref(), Some("x y"));
        assert_eq!(
            symbol
                .pins
                .iter()
                .filter_map(|pin| pin.number.as_deref())
                .collect::<Vec<_>>(),
            vec!["3", "4"]
        );
        assert_eq!(symbol.pins[1].alternate.as_deref(), Some("ALT4"));

        let scene = schematic.canvas_scene();
        assert_eq!(scene.symbols[0].unit_name.as_deref(), Some("Analog"));
        assert_eq!(scene.symbols[0].mirror.as_deref(), Some("x y"));
        let pin3 = scene.symbols[0]
            .pins
            .iter()
            .find(|pin| pin.number == "3")
            .unwrap();
        assert_close(pin3.start.x, 22.54);
        assert_close(pin3.end.x, 20.0);

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(mirror x y)"));
        assert!(exported.contains("(unit 2)"));
        assert!(exported.contains("(body_style 2)"));
        assert!(exported.contains("(alternate \"ALT4\")"));
        let reparsed =
            parse_kicad_schematic(&exported, "configure_symbol_roundtrip.kicad_sch").unwrap();
        assert_eq!(reparsed.symbols[0].mirror.as_deref(), Some("x y"));
        assert_eq!(
            reparsed.symbols[0].pins[1].alternate.as_deref(),
            Some("ALT4")
        );

        schematic
            .apply_edit(KicadSchematicEdit::ConfigureSymbol {
                reference: "U2".to_string(),
                unit: None,
                body_style: Some(None),
                mirror: Some(None),
                pin_alternates: Some(BTreeMap::new()),
            })
            .unwrap();
        assert_eq!(schematic.symbols[0].body_style, None);
        assert_eq!(schematic.symbols[0].mirror, None);
        assert!(
            schematic.symbols[0]
                .pins
                .iter()
                .all(|pin| pin.alternate.is_none())
        );

        let error = schematic
            .apply_edit(KicadSchematicEdit::ConfigureSymbol {
                reference: "U2".to_string(),
                unit: Some(2),
                body_style: Some(Some(2)),
                mirror: None,
                pin_alternates: Some(BTreeMap::from([("4".to_string(), "MISSING".to_string())])),
            })
            .unwrap_err();
        assert!(error.to_string().contains("has no alternate 'MISSING'"));
    }

    #[test]
    fn rejects_edit_that_reuses_existing_kicad_uuid() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        let error = schematic
            .apply_edit(KicadSchematicEdit::AddWire {
                points: vec![
                    KicadPoint { x: 10.0, y: 10.0 },
                    KicadPoint { x: 20.0, y: 10.0 },
                ],
                uuid: Some("22222222-2222-2222-2222-222222222222".to_string()),
            })
            .unwrap_err();

        assert!(error.to_string().contains("already used"));
    }

    #[test]
    fn parses_kicad_symbol_library_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let library = read_kicad_symbol_library(
            &workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
        )
        .unwrap();

        let resistor = library.symbol("NekoSpice:R").unwrap();
        assert_eq!(resistor.property("Reference"), Some("R"));
        assert_eq!(resistor.graphics.len(), 1);
        assert_eq!(resistor.pins.len(), 2);
        assert_eq!(resistor.pins[0].number(), "1");
        assert_eq!(resistor.pins[0].electrical_type, "passive");
        let bounds = resistor.bounding_box().unwrap();
        assert_eq!(bounds.min.x, -2.54);
        assert_eq!(bounds.max.x, 2.54);
        assert!(bounds.width() > 5.0);
        assert!(library.to_summary_json().contains("\"symbol_count\": 3"));
        assert!(library.to_summary_json().contains("\"graphic_count\": 6"));
    }

    #[test]
    fn roundtrips_kicad_symbol_library_fixture_through_writer() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let library = read_kicad_symbol_library(
            &workspace_root.join("examples/kicad_schematic/neko_spice.kicad_sym"),
        )
        .unwrap();

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(kicad_symbol_lib"));
        assert!(exported.contains("(symbol \"NekoSpice:R\""));
        let reparsed = parse_kicad_symbol_library(&exported, "roundtrip.kicad_sym").unwrap();

        assert_eq!(reparsed.symbols.len(), library.symbols.len());
        assert_eq!(
            reparsed
                .symbols
                .iter()
                .map(|symbol| symbol.graphics.len())
                .sum::<usize>(),
            6
        );
        let resistor = reparsed.symbol("NekoSpice:R").unwrap();
        assert_eq!(resistor.pins.len(), 2);
        assert_eq!(resistor.property("Reference"), Some("R"));
        assert_eq!(resistor.graphics.len(), 1);
        let bounds = resistor.bounding_box().unwrap();
        assert_close(bounds.min.x, -2.54);
        assert_close(bounds.max.x, 2.54);
    }

    #[test]
    fn preserves_kicad_symbol_library_file_metadata() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "kicad_symbol_editor")
  (generator_version "9.0")
  (symbol "NekoSpice:Fonted"
    (embedded_fonts no)
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Fonted" (at 0 -2.54 0))
  )
  (symbol "NekoSpice:Embedded"
    (embedded_fonts yes)
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Embedded" (at 0 -2.54 0))
  )
)"#,
            "metadata.kicad_sym",
        )
        .unwrap();

        assert_eq!(library.generator_version.as_deref(), Some("9.0"));
        assert_eq!(
            library.symbol("NekoSpice:Fonted").unwrap().embedded_fonts,
            Some(false)
        );
        assert_eq!(
            library.symbol("NekoSpice:Embedded").unwrap().embedded_fonts,
            Some(true)
        );
        let summary = library.to_summary_json();
        assert!(summary.contains("\"generator_version\": \"9.0\""));
        assert!(summary.contains("\"embedded_font_symbol_count\": 2"));

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(generator_version \"9.0\")"));
        assert!(exported.contains("(embedded_fonts no)"));
        assert!(exported.contains("(embedded_fonts yes)"));

        let reparsed =
            parse_kicad_symbol_library(&exported, "metadata_roundtrip.kicad_sym").unwrap();
        assert_eq!(reparsed.generator_version.as_deref(), Some("9.0"));
        assert_eq!(
            reparsed.symbol("NekoSpice:Fonted").unwrap().embedded_fonts,
            Some(false)
        );
    }

    #[test]
    fn parses_kicad_symbol_library_bezier_graphics_and_roundtrips() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:Curve"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Curve" (at 0 -2.54 0))
    (symbol "Curve_0_1"
      (bezier
        (pts (xy -2.54 0) (xy -1.27 -2.54) (xy 1.27 2.54) (xy 2.54 0))
        (stroke (width 0.254) (type default))
        (fill (type none))
      )
    )
  )
)"#,
            "curve.kicad_sym",
        )
        .unwrap();

        let symbol = library.symbol("NekoSpice:Curve").unwrap();
        assert_eq!(symbol.graphics.len(), 1);
        if let KicadGraphic::Bezier { points } = &symbol.graphics[0].graphic {
            assert_eq!(points.len(), 4);
            assert_close(points[0].x, -2.54);
            assert_close(points[3].x, 2.54);
        } else {
            panic!("expected bezier symbol graphic");
        }
        let bounds = symbol.bounding_box().unwrap();
        assert_close(bounds.min.x, -2.54);
        assert_close(bounds.max.y, 2.54);

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(bezier"));
        assert!(
            exported.contains("(pts (xy -2.54 0) (xy -1.27 -2.54) (xy 1.27 2.54) (xy 2.54 0))")
        );
        let reparsed = parse_kicad_symbol_library(&exported, "curve_roundtrip.kicad_sym").unwrap();
        let reparsed_symbol = reparsed.symbol("NekoSpice:Curve").unwrap();
        assert!(matches!(
            &reparsed_symbol.graphics[0].graphic,
            KicadGraphic::Bezier { points } if points.len() == 4
        ));
    }

    #[test]
    fn preserves_kicad_symbol_pin_display_and_text_effects() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:StyledPin"
    (pin_numbers
      (hide yes)
    )
    (pin_names
      (offset 2.54)
      (hide no)
    )
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "StyledPin" (at 0 -2.54 0))
    (symbol "StyledPin_0_1"
      (pin input clock
        (at -5.08 0 0)
        (length 5.08)
        (name "CLK"
          (effects
            (font (size 1.524 1.016) (thickness 0.1524) bold italic (color 58 104 255 0.5))
            (justify left bottom)
            (hide yes)
          )
        )
        (number "1"
          (effects
            (font (size 1.27 1.27) (color 255 89 101 1))
            (justify right)
          )
        )
      )
    )
  )
)"#,
            "styled_pin.kicad_sym",
        )
        .unwrap();

        let symbol = library.symbol("NekoSpice:StyledPin").unwrap();
        assert_eq!(symbol.pin_numbers.as_ref().unwrap().hide, Some(true));
        assert_close(symbol.pin_names.as_ref().unwrap().offset.unwrap(), 2.54);
        assert_eq!(symbol.pin_names.as_ref().unwrap().hide, Some(false));
        assert_eq!(symbol.pins.len(), 1);
        let pin = &symbol.pins[0];
        assert_eq!(pin.number(), "1");
        assert_eq!(pin.name(), "CLK");
        assert_eq!(pin.electrical_type, "input");
        assert_eq!(pin.shape, "clock");
        assert_close(pin.name_effects().unwrap().font_size.unwrap().width, 1.524);
        assert_close(pin.name_effects().unwrap().font_size.unwrap().height, 1.016);
        assert_eq!(pin.name_effects().unwrap().font_bold, Some(true));
        assert_eq!(pin.name_effects().unwrap().font_italic, Some(true));
        assert!(pin.name_effects().unwrap().hide);
        assert_eq!(
            pin.number_effects().unwrap().font_color,
            Some(KicadColor {
                red: 255.0,
                green: 89.0,
                blue: 101.0,
                alpha: 1.0,
            })
        );
        let summary = library.to_summary_json();
        assert!(summary.contains("\"pin_count\": 1"));
        assert!(summary.contains("\"pin_display_setting_count\": 2"));
        assert!(summary.contains("\"pin_text_effect_count\": 2"));

        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:StyledPin"
      (pin_numbers (hide yes))
      (pin_names (offset 2.54) (hide no))
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "StyledPin" (at 0 -2.54 0))
      (symbol "StyledPin_0_1"
        (pin input clock
          (at -5.08 0 0)
          (length 5.08)
          (name "CLK" (effects (font (size 1.524 1.016) bold italic) (hide yes)))
          (number "1" (effects (font (size 1.27 1.27) (color 255 89 101 1))))
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:StyledPin")
    (at 10 10 0)
    (property "Reference" "U1" (at 10 7 0))
    (property "Value" "StyledPin" (at 10 13 0))
  )
)"#,
            "styled_pin_canvas.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        assert_eq!(scene.symbols.len(), 1);
        assert_eq!(
            scene.symbols[0].pin_numbers.as_ref().unwrap().hide,
            Some(true)
        );
        assert_close(
            scene.symbols[0].pin_names.as_ref().unwrap().offset.unwrap(),
            2.54,
        );
        assert!(scene.symbols[0].pins[0].name_effects.as_ref().unwrap().hide);
        assert_eq!(
            scene.symbols[0].pins[0]
                .number_effects
                .as_ref()
                .unwrap()
                .font_color,
            Some(KicadColor {
                red: 255.0,
                green: 89.0,
                blue: 101.0,
                alpha: 1.0,
            })
        );

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(pin_numbers"));
        assert!(exported.contains("(hide yes)"));
        assert!(exported.contains("(pin_names"));
        assert!(exported.contains("(offset 2.54)"));
        assert!(exported.contains("(hide no)"));
        assert!(exported.contains("(font (size 1.524 1.016) (thickness 0.1524) (bold yes) (italic yes) (color 58 104 255 0.5))"));
        assert!(exported.contains("(justify left bottom)"));
        assert!(exported.contains("(font (size 1.27 1.27) (color 255 89 101 1))"));

        let reparsed =
            parse_kicad_symbol_library(&exported, "styled_pin_roundtrip.kicad_sym").unwrap();
        let reparsed_symbol = reparsed.symbol("NekoSpice:StyledPin").unwrap();
        assert_eq!(
            reparsed_symbol.pin_numbers.as_ref().unwrap().hide,
            Some(true)
        );
        assert_close(
            reparsed_symbol.pin_names.as_ref().unwrap().offset.unwrap(),
            2.54,
        );
        assert_eq!(
            reparsed_symbol.pins[0].name_effects().unwrap().font_bold,
            Some(true)
        );
        assert_eq!(
            reparsed_symbol.pins[0].number_effects().unwrap().justify,
            vec!["right"]
        );
    }

    #[test]
    fn preserves_kicad_symbol_pin_alternates_and_canvas_metadata() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:AltPin"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "AltPin" (at 0 -2.54 0))
    (symbol "AltPin_0_1"
      (pin input line
        (at -5.08 0 0)
        (length 5.08)
        (name "SDI" (effects (font (size 1.27 1.27))))
        (number "6" (effects (font (size 1.27 1.27))))
        (alternate "SDA" bidirectional line)
        (alternate "SDO" output clock)
      )
    )
  )
)"#,
            "alt_pin.kicad_sym",
        )
        .unwrap();

        let symbol = library.symbol("NekoSpice:AltPin").unwrap();
        assert_eq!(symbol.pins.len(), 1);
        assert_eq!(symbol.pins[0].alternates.len(), 2);
        assert_eq!(symbol.pins[0].alternates[0].name, "SDA");
        assert_eq!(
            symbol.pins[0].alternates[0].electrical_type,
            "bidirectional"
        );
        assert_eq!(symbol.pins[0].alternates[1].name, "SDO");
        assert_eq!(symbol.pins[0].alternates[1].shape, "clock");
        assert!(
            library
                .to_summary_json()
                .contains("\"pin_alternate_count\": 2")
        );

        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:AltPin"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "AltPin" (at 0 -2.54 0))
      (symbol "AltPin_0_1"
        (pin input line
          (at -5.08 0 0)
          (length 5.08)
          (name "SDI" (effects (font (size 1.27 1.27))))
          (number "6" (effects (font (size 1.27 1.27))))
          (alternate "SDA" bidirectional line)
          (alternate "SDO" output clock)
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:AltPin")
    (at 10 10 0)
    (property "Reference" "U1" (at 10 7 0))
    (property "Value" "AltPin" (at 10 13 0))
  )
)"#,
            "alt_pin_canvas.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        assert_eq!(scene.symbols[0].pins[0].alternates.len(), 2);
        assert_eq!(scene.symbols[0].pins[0].alternates[0].name, "SDA");
        assert_eq!(
            scene.symbols[0].pins[0].alternates[1].electrical_type,
            "output"
        );

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(alternate \"SDA\" bidirectional line)"));
        assert!(exported.contains("(alternate \"SDO\" output clock)"));

        let reparsed =
            parse_kicad_symbol_library(&exported, "alt_pin_roundtrip.kicad_sym").unwrap();
        assert_eq!(
            reparsed.symbol("NekoSpice:AltPin").unwrap().pins[0].alternates,
            symbol.pins[0].alternates
        );
    }

    #[test]
    fn preserves_kicad_symbol_definition_flags_and_roundtrips() {
        let library = parse_kicad_symbol_library(
            r##"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:PowerBare"
    (power)
    (exclude_from_sim no)
    (in_bom no)
    (on_board yes)
    (in_pos_files no)
    (duplicate_pin_numbers_are_jumpers yes)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PowerBare" (at 0 -2.54 0))
    (symbol "PowerBare_0_1"
      (pin power_in line (at 0 0 0) (length 0) (name "VCC") (number "1"))
    )
  )
  (symbol "NekoSpice:PowerGlobal"
    (power global)
    (in_bom yes)
    (on_board no)
    (in_pos_files yes)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PowerGlobal" (at 0 -2.54 0))
  )
  (symbol "NekoSpice:PowerLocal"
    (power local)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PowerLocal" (at 0 -2.54 0))
  )
)"##,
            "symbol_flags.kicad_sym",
        )
        .unwrap();

        let bare = library.symbol("NekoSpice:PowerBare").unwrap();
        assert_eq!(bare.power, Some(KicadSymbolPower::Bare));
        assert_eq!(bare.exclude_from_sim, Some(false));
        assert_eq!(bare.in_bom, Some(false));
        assert_eq!(bare.on_board, Some(true));
        assert_eq!(bare.in_pos_files, Some(false));
        assert_eq!(bare.duplicate_pin_numbers_are_jumpers, Some(true));
        assert_eq!(
            library.symbol("NekoSpice:PowerGlobal").unwrap().power,
            Some(KicadSymbolPower::Global)
        );
        assert_eq!(
            library.symbol("NekoSpice:PowerGlobal").unwrap().in_bom,
            Some(true)
        );
        assert_eq!(
            library.symbol("NekoSpice:PowerGlobal").unwrap().on_board,
            Some(false)
        );
        assert_eq!(
            library
                .symbol("NekoSpice:PowerGlobal")
                .unwrap()
                .in_pos_files,
            Some(true)
        );
        assert_eq!(
            library.symbol("NekoSpice:PowerLocal").unwrap().power,
            Some(KicadSymbolPower::Local)
        );

        let summary = library.to_summary_json();
        assert!(summary.contains("\"power_symbol_count\": 3"));
        assert!(summary.contains("\"symbol_in_bom_setting_count\": 2"));
        assert!(summary.contains("\"symbol_on_board_setting_count\": 2"));
        assert!(summary.contains("\"symbol_in_pos_files_setting_count\": 2"));
        assert!(summary.contains("\"duplicate_pin_numbers_are_jumpers_count\": 1"));

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(power)"));
        assert!(exported.contains("(power global)"));
        assert!(exported.contains("(power local)"));
        assert!(exported.contains("(exclude_from_sim no)"));
        assert!(exported.contains("(in_bom no)"));
        assert!(exported.contains("(in_bom yes)"));
        assert!(exported.contains("(on_board no)"));
        assert!(exported.contains("(on_board yes)"));
        assert!(exported.contains("(in_pos_files no)"));
        assert!(exported.contains("(in_pos_files yes)"));
        assert!(exported.contains("(duplicate_pin_numbers_are_jumpers yes)"));

        let reparsed =
            parse_kicad_symbol_library(&exported, "symbol_flags_roundtrip.kicad_sym").unwrap();
        assert_eq!(
            reparsed.symbol("NekoSpice:PowerBare").unwrap().power,
            Some(KicadSymbolPower::Bare)
        );
        assert_eq!(
            reparsed
                .symbol("NekoSpice:PowerBare")
                .unwrap()
                .duplicate_pin_numbers_are_jumpers,
            Some(true)
        );
        assert_eq!(
            reparsed.symbol("NekoSpice:PowerGlobal").unwrap().power,
            Some(KicadSymbolPower::Global)
        );
        assert_eq!(
            reparsed.symbol("NekoSpice:PowerLocal").unwrap().power,
            Some(KicadSymbolPower::Local)
        );
    }

    #[test]
    fn preserves_kicad_symbol_inheritance_body_styles_and_jumpers() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:Parent"
    (body_styles demorgan)
    (duplicate_pin_numbers_are_jumpers yes)
    (jumper_pin_groups
      ("A1" "A2")
      ("B1" "B2" "B3")
    )
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Parent" (at 0 -2.54 0))
  )
  (symbol "NekoSpice:Derived"
    (extends "NekoSpice:Parent")
    (body_styles "logic" "analog-front-end")
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Derived" (at 0 -2.54 0))
  )
)"#,
            "symbol_inheritance.kicad_sym",
        )
        .unwrap();

        let parent = library.symbol("NekoSpice:Parent").unwrap();
        assert_eq!(parent.body_styles, Some(KicadSymbolBodyStyles::Demorgan));
        assert_eq!(parent.duplicate_pin_numbers_are_jumpers, Some(true));
        assert_eq!(
            parent.jumper_pin_groups,
            vec![
                vec!["A1".to_string(), "A2".to_string()],
                vec!["B1".to_string(), "B2".to_string(), "B3".to_string()]
            ]
        );

        let derived = library.symbol("NekoSpice:Derived").unwrap();
        assert_eq!(derived.extends.as_deref(), Some("NekoSpice:Parent"));
        assert_eq!(
            derived.body_styles,
            Some(KicadSymbolBodyStyles::Names(vec![
                "logic".to_string(),
                "analog-front-end".to_string()
            ]))
        );

        let summary = library.to_summary_json();
        assert!(summary.contains("\"extended_symbol_count\": 1"));
        assert!(summary.contains("\"body_style_symbol_count\": 2"));
        assert!(summary.contains("\"jumper_pin_group_count\": 2"));

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(body_styles demorgan)"));
        assert!(exported.contains("(duplicate_pin_numbers_are_jumpers yes)"));
        assert!(exported.contains("(jumper_pin_groups"));
        assert!(exported.contains("(\"A1\" \"A2\")"));
        assert!(exported.contains("(\"B1\" \"B2\" \"B3\")"));
        assert!(exported.contains("(extends \"NekoSpice:Parent\")"));
        assert!(exported.contains("(body_styles logic analog-front-end)"));

        let reparsed =
            parse_kicad_symbol_library(&exported, "symbol_inheritance_roundtrip.kicad_sym")
                .unwrap();
        assert_eq!(
            reparsed
                .symbol("NekoSpice:Derived")
                .unwrap()
                .extends
                .as_deref(),
            Some("NekoSpice:Parent")
        );
        assert_eq!(
            reparsed
                .symbol("NekoSpice:Parent")
                .unwrap()
                .jumper_pin_groups
                .len(),
            2
        );
    }

    #[test]
    fn resolves_kicad_symbol_inheritance_for_canvas_netlist_and_placement() {
        let mut schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:ParentR"
      (pin_names (offset 0.508))
      (pin_numbers (hide yes))
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "1k" (at 0 -2.54 0))
      (property "Sim.Device" "R" (at 0 0 0))
      (symbol "ParentR_0_1"
        (rectangle
          (start -1 -1)
          (end 1 1)
          (stroke (width 0.127) (type default))
          (fill (type none))
        )
        (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
        (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2"))
      )
    )
    (symbol "NekoSpice:DerivedR"
      (extends "NekoSpice:ParentR")
      (pin_names (offset 1.016))
      (property "Reference" "R" (at 0 0 0))
      (property "Value" "4.7k" (at 0 -2.54 0))
    )
  )
  (wire (pts (xy 17.46 10) (xy 10 10)))
  (wire (pts (xy 22.54 10) (xy 30 10)))
  (label "in" (at 10 10 0))
  (label "0" (at 30 10 0))
  (text ".op" (at 5 5 0))
  (symbol
    (lib_id "NekoSpice:DerivedR")
    (at 20 10 0)
    (property "Reference" "R1" (at 20 8 0))
    (property "Value" "2.2k" (at 20 12 0))
  )
)"#,
            "derived_symbol.kicad_sch",
        )
        .unwrap();

        let scene = schematic.canvas_scene();
        let symbol = scene
            .symbols
            .iter()
            .find(|symbol| symbol.reference == "R1")
            .unwrap();
        assert_eq!(symbol.graphics.len(), 1);
        assert_eq!(symbol.pins.len(), 2);
        assert_close(symbol.pin_names.as_ref().unwrap().offset.unwrap(), 1.016);
        assert_eq!(symbol.pin_numbers.as_ref().unwrap().hide, Some(true));

        let netlist = schematic.to_spice_netlist().unwrap();
        assert!(netlist.contains("R1 in 0 2.2k"));

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(extends \"NekoSpice:ParentR\")"));
        assert!(!exported.contains("(symbol \"DerivedR_0_1\""));

        let derived = schematic
            .symbol_definition("NekoSpice:DerivedR")
            .unwrap()
            .clone();
        schematic
            .place_symbol(KicadSymbolPlacement {
                definition: derived,
                library_symbols: Vec::new(),
                reference: "R2".to_string(),
                value: "3.3k".to_string(),
                at: KicadAt {
                    x: 40.0,
                    y: 10.0,
                    rotation: 0.0,
                },
                unit: Some(1),
                body_style: None,
                pin_alternates: BTreeMap::new(),
                uuid: None,
            })
            .unwrap();
        let placed = schematic
            .symbols
            .iter()
            .find(|symbol| symbol.reference() == Some("R2"))
            .unwrap();
        assert_eq!(placed.pins.len(), 2);
    }

    #[test]
    fn preserves_kicad_symbol_graphic_styles_and_roundtrips() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "NekoSpice:Styled"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Styled" (at 0 -2.54 0))
    (symbol "Styled_0_1"
      (polyline private
        (pts (xy -2.54 -1.27) (xy 0 1.27) (xy 2.54 -1.27))
        (stroke (width 0.0254) (type dash_dot) (color 58 104 255 0.5))
        (fill (type outline))
        (uuid "a5cd8da1-8f7f-4f80-bb23-0317de562222")
        (locked yes)
      )
      (rectangle
        (start -1 -1)
        (end 1 1)
        (stroke (width 0) (type default) (color 0 0 0 0))
        (fill (type background))
      )
      (text "ALT"
        (at 1.27 2.54 90)
        (effects
          (font (size 1.524 1.016) (thickness 0.1524) bold italic (color 255 89 101 0.75))
          (justify right bottom)
          (href "https://nekospice.test/symbol-text")
        )
      )
    )
  )
)"#,
            "styled_symbol.kicad_sym",
        )
        .unwrap();

        let symbol = library.symbol("NekoSpice:Styled").unwrap();
        assert_eq!(symbol.graphics.len(), 3);
        let styled = &symbol.graphics[0];
        assert!(styled.private);
        assert!(matches!(
            styled.graphic,
            KicadGraphic::Polyline { ref points } if points.len() == 3
        ));
        assert_close(styled.stroke.as_ref().unwrap().width.unwrap(), 0.0254);
        assert_eq!(
            styled.stroke.as_ref().unwrap().stroke_type.as_deref(),
            Some("dash_dot")
        );
        assert_eq!(
            styled.stroke.as_ref().unwrap().color,
            Some(KicadColor {
                red: 58.0,
                green: 104.0,
                blue: 255.0,
                alpha: 0.5,
            })
        );
        assert_eq!(
            styled.fill.as_ref().unwrap().fill_type.as_deref(),
            Some("outline")
        );
        assert_eq!(
            styled.uuid.as_deref(),
            Some("a5cd8da1-8f7f-4f80-bb23-0317de562222")
        );
        assert_eq!(styled.locked, Some(true));
        assert_eq!(
            symbol.graphics[1]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("background")
        );
        if let KicadGraphic::Text { text, at, effects } = &symbol.graphics[2].graphic {
            assert_eq!(text, "ALT");
            assert_close(at.unwrap().x, 1.27);
            assert_close(at.unwrap().rotation, 90.0);
            let effects = effects.as_ref().unwrap();
            assert_close(effects.font_size.unwrap().width, 1.524);
            assert_close(effects.font_size.unwrap().height, 1.016);
            assert_close(effects.font_thickness.unwrap(), 0.1524);
            assert_eq!(effects.font_bold, Some(true));
            assert_eq!(effects.font_italic, Some(true));
            assert_eq!(
                effects.font_color,
                Some(KicadColor {
                    red: 255.0,
                    green: 89.0,
                    blue: 101.0,
                    alpha: 0.75,
                })
            );
            assert_eq!(effects.justify, vec!["right", "bottom"]);
            assert_eq!(
                effects.href.as_deref(),
                Some("https://nekospice.test/symbol-text")
            );
        } else {
            panic!("expected styled text symbol graphic");
        }

        let schematic = parse_kicad_schematic(
            r#"(kicad_sch
  (version 20230121)
  (generator "NekoSpice")
  (paper "A4")
  (lib_symbols
    (symbol "NekoSpice:Styled"
      (property "Reference" "U" (at 0 0 0))
      (property "Value" "Styled" (at 0 -2.54 0))
      (symbol "Styled_0_1"
        (polyline private
          (pts (xy -2.54 -1.27) (xy 0 1.27) (xy 2.54 -1.27))
          (stroke (width 0.0254) (type dash_dot) (color 58 104 255 0.5))
          (fill (type outline))
        )
      )
    )
  )
  (symbol
    (lib_id "NekoSpice:Styled")
    (at 10 10 0)
    (property "Reference" "U1" (at 10 7 0))
    (property "Value" "Styled" (at 10 13 0))
  )
)"#,
            "styled_symbol_canvas.kicad_sch",
        )
        .unwrap();
        let scene = schematic.canvas_scene();
        assert_eq!(scene.symbols.len(), 1);
        assert!(matches!(
            &scene.symbols[0].graphics[0],
            super::KicadCanvasGraphic::Polyline {
                stroke: Some(stroke),
                fill: Some(fill),
                ..
            } if stroke.stroke_type.as_deref() == Some("dash_dot")
                && fill.fill_type.as_deref() == Some("outline")
        ));

        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(
            library
                .to_summary_json()
                .contains("\"symbol_graphic_text_effect_count\": 1")
        );
        assert!(exported.contains("(polyline private"));
        assert!(
            exported.contains("(stroke (width 0.0254) (type dash_dot) (color 58 104 255 0.5))")
        );
        assert!(exported.contains("(fill (type outline))"));
        assert!(exported.contains("(uuid \"a5cd8da1-8f7f-4f80-bb23-0317de562222\")"));
        assert!(exported.contains("(locked yes)"));
        assert!(exported.contains("(fill (type background))"));
        assert!(exported.contains("(text \"ALT\" (at 1.27 2.54 90)"));
        assert!(
            exported.contains(
                "(effects (font (size 1.524 1.016) (thickness 0.1524) (bold yes) (italic yes) (color 255 89 101 0.75)) (justify right bottom) (href \"https://nekospice.test/symbol-text\"))"
            )
        );

        let reparsed =
            parse_kicad_symbol_library(&exported, "styled_symbol_roundtrip.kicad_sym").unwrap();
        let reparsed_symbol = reparsed.symbol("NekoSpice:Styled").unwrap();
        assert_eq!(reparsed_symbol.graphics.len(), 3);
        assert!(reparsed_symbol.graphics[0].private);
        assert_eq!(
            reparsed_symbol.graphics[0]
                .stroke
                .as_ref()
                .unwrap()
                .stroke_type
                .as_deref(),
            Some("dash_dot")
        );
        assert_eq!(
            reparsed_symbol.graphics[0]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("outline")
        );
        assert_eq!(reparsed_symbol.graphics[0].locked, Some(true));
        assert_eq!(
            reparsed_symbol.graphics[1]
                .fill
                .as_ref()
                .unwrap()
                .fill_type
                .as_deref(),
            Some("background")
        );
        assert!(matches!(
            &reparsed_symbol.graphics[2].graphic,
            KicadGraphic::Text { effects: Some(effects), .. }
                if effects.font_italic == Some(true)
                    && effects.justify == vec!["right".to_string(), "bottom".to_string()]
                    && effects.href.as_deref() == Some("https://nekospice.test/symbol-text")
        ));
    }

    #[test]
    fn parses_kicad_symbol_library_table_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let table = read_kicad_symbol_library_table(
            &workspace_root.join("examples/kicad_schematic/sym-lib-table"),
        )
        .unwrap();

        assert_eq!(table.version.as_deref(), Some("7"));
        assert_eq!(table.libraries.len(), 1);
        assert_eq!(table.libraries[0].name, "NekoSpice");
        assert_eq!(table.libraries[0].library_type, "KiCad");
        assert_eq!(
            table.libraries[0].description.as_deref(),
            Some("NekoSpice analog simulation symbols")
        );
        assert_eq!(table.enabled_kicad_libraries().count(), 1);
        assert!(table.to_summary_json().contains("\"library_count\": 1"));
    }

    #[test]
    fn parses_kicad_project_fixture_and_sheet_summary() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let project = read_kicad_project(
            &workspace_root
                .join("examples/kicad_project_schematic/kicad_project_schematic.kicad_pro"),
        )
        .unwrap();

        assert_eq!(
            project.meta_filename.as_deref(),
            Some("kicad_project_schematic.kicad_pro")
        );
        assert_eq!(project.meta_version, Some(1));
        assert_eq!(
            project.project_name.as_deref(),
            Some("kicad_project_schematic")
        );
        assert!(
            project
                .schematic_stem_candidates()
                .contains(&"kicad_project_schematic".to_string())
        );
        assert!(project.to_summary_json().contains("\"project_name\""));

        let project = parse_kicad_project(
            r#"{
  "meta": { "filename": "root_project.kicad_pro", "version": 2 },
  "schematic": { "page_layout_descr_file": "layout.kicad_wks" },
  "sheets": [
    [ "root-sheet", "Root" ],
    [ "child-sheet", "child" ]
  ],
  "text_variables": { "REV": "A" }
}"#,
            "root_project.kicad_pro",
        )
        .unwrap();

        assert_eq!(project.meta_version, Some(2));
        assert_eq!(
            project.schematic_page_layout_descr_file.as_deref(),
            Some("layout.kicad_wks")
        );
        assert_eq!(project.sheets.len(), 2);
        assert_eq!(project.sheets[0].name, "Root");
        assert_eq!(project.sheets[1].uuid, "child-sheet");
        assert_eq!(project.text_variable_count, 1);
        assert_eq!(
            project.schematic_stem_candidates(),
            vec!["root_project".to_string()]
        );
    }

    #[test]
    fn builds_kicad_symbol_library_index_fixture() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let index = read_kicad_symbol_library_index(
            &workspace_root.join("examples/kicad_schematic/sym-lib-table"),
        )
        .unwrap();

        assert_eq!(index.libraries.len(), 1);
        assert_eq!(index.symbols.len(), 3);
        assert_eq!(index.diagnostics.len(), 0);
        let resistor = index.symbol("NekoSpice:R").unwrap();
        assert_eq!(resistor.library, "NekoSpice");
        assert_eq!(resistor.name, "R");
        assert_eq!(resistor.pin_count, 2);
        assert_eq!(resistor.graphic_count, 1);
        assert!(resistor.bounding_box.is_some());
        assert!(index.to_summary_json().contains("\"symbol_count\": 3"));
    }

    #[test]
    fn indexes_kicad_symbol_library_browser_metadata() {
        let project_dir = std::env::temp_dir().join(format!(
            "nekospice_kicad_symbol_index_metadata_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            project_dir.join("browser.kicad_sym"),
            r##"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "Parent"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Parent" (at 0 -2.54 0))
    (property "Description" "Parent analog switch" (at 0 -5.08 0))
    (property "ki_keywords" "switch analog mux" (at 0 -7.62 0) (hide yes))
    (property "ki_fp_filters" "Package_SO:SOIC-* Connector{space}Foo:*" (at 0 -10.16 0) (hide yes))
    (symbol "Parent_0_1"
      (rectangle (start -1 -1) (end 1 1) (stroke (width 0.127) (type default)) (fill (type none)))
    )
  )
  (symbol "Derived"
    (extends "Parent")
    (body_styles "normal" "alternate-body" "unused-body")
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Derived" (at 0 -2.54 0))
    (property "ki_keywords" "" (at 0 -5.08 0) (hide yes))
    (symbol "Derived_1_1"
      (unit_name "Logic")
      (pin passive line
        (at -2.54 0 0)
        (length 2.54)
        (name "A")
        (number "1")
        (alternate "A_ALT" bidirectional line)
      )
    )
    (symbol "Derived_2_1"
      (unit_name "Power")
      (pin power_in line (at 2.54 0 180) (length 2.54) (name "VCC") (number "2"))
    )
    (symbol "Derived_1_2"
      (pin passive inverted (at -2.54 2.54 0) (length 2.54) (name "A2") (number "3"))
    )
  )
  (symbol "PWR" (power global)
    (property "Reference" "#PWR" (at 0 0 0))
    (property "Value" "PWR" (at 0 -2.54 0))
  )
)"##,
        )
        .unwrap();
        fs::write(
            project_dir.join("sym-lib-table"),
            r#"(sym_lib_table
  (version 7)
  (lib (name "Browser")(type "KiCad")(uri "${KIPRJMOD}/browser.kicad_sym")(options "")(descr ""))
)"#,
        )
        .unwrap();

        let index = read_kicad_symbol_library_index(&project_dir.join("sym-lib-table")).unwrap();
        let derived = index.symbol("Browser:Derived").unwrap();
        let power = index.symbol("Browser:PWR").unwrap();

        assert_eq!(derived.description.as_deref(), Some("Parent analog switch"));
        assert_eq!(derived.keywords.as_deref(), Some("switch analog mux"));
        assert_eq!(
            derived.footprint_filters,
            vec![
                "Package_SO:SOIC-*".to_string(),
                "Connector Foo:*".to_string()
            ]
        );
        assert_eq!(derived.pin_count, 3);
        assert_eq!(derived.graphic_count, 1);
        assert_eq!(derived.unit_count, 2);
        assert_eq!(
            derived.units,
            vec![
                KicadIndexedSymbolUnit {
                    unit: 1,
                    name: Some("Logic".to_string())
                },
                KicadIndexedSymbolUnit {
                    unit: 2,
                    name: Some("Power".to_string())
                }
            ]
        );
        assert_eq!(
            derived.body_styles,
            vec![
                KicadIndexedSymbolBodyStyle {
                    body_style: 1,
                    name: Some("normal".to_string())
                },
                KicadIndexedSymbolBodyStyle {
                    body_style: 2,
                    name: Some("alternate-body".to_string())
                },
                KicadIndexedSymbolBodyStyle {
                    body_style: 3,
                    name: Some("unused-body".to_string())
                }
            ]
        );
        assert_eq!(derived.pins.len(), 3);
        assert_eq!(derived.pins[0].number, "1");
        assert_eq!(derived.pins[0].alternates[0].name, "A_ALT");
        assert_eq!(derived.pins[2].body_style, 2);
        assert_eq!(derived.extends.as_deref(), Some("Parent"));
        assert_eq!(power.power.as_deref(), Some("global"));
        assert!(index.to_summary_json().contains("\"unit_count\": 4"));
        assert!(
            index
                .to_summary_json()
                .contains("\"described_symbol_count\": 2")
        );
        assert!(
            index
                .to_summary_json()
                .contains("\"keyword_symbol_count\": 2")
        );
        assert!(
            index
                .to_summary_json()
                .contains("\"footprint_filter_count\": 4")
        );
        assert!(
            index
                .to_summary_json()
                .contains("\"extended_symbol_count\": 1")
        );
        assert!(
            index
                .to_summary_json()
                .contains("\"power_symbol_count\": 1")
        );
        let index_json: serde_json::Value = serde_json::from_str(&index.to_json()).unwrap();
        assert_eq!(index_json["library_count"], 1);
        assert_eq!(index_json["symbol_count"], 3);
        assert_eq!(index_json["libraries"][0]["name"], "Browser");
        assert_eq!(index_json["symbols"][1]["id"], "Browser:Derived");
        assert_eq!(
            index_json["symbols"][1]["description"],
            "Parent analog switch"
        );
        assert_eq!(
            index_json["symbols"][1]["footprint_filters"][1],
            "Connector Foo:*"
        );
        assert_eq!(index_json["symbols"][1]["units"][0]["name"], "Logic");
        assert_eq!(
            index_json["symbols"][1]["body_styles"][1]["name"],
            "alternate-body"
        );
        assert_eq!(
            index_json["symbols"][1]["body_styles"][2]["name"],
            "unused-body"
        );
        assert_eq!(
            index_json["symbols"][1]["pins"][0]["alternates"][0]["name"],
            "A_ALT"
        );
        assert_eq!(index_json["symbols"][1]["bounding_box"]["min"]["x"], -2.54);
        assert_eq!(index_json["diagnostic_count"], 0);
        assert!(index_json["diagnostics"].as_array().unwrap().is_empty());

        let by_text = index.query(&KicadSymbolLibraryIndexQuery {
            text: Some("analog".to_string()),
            ..Default::default()
        });
        assert_eq!(
            by_text
                .symbols
                .iter()
                .map(|symbol| symbol.id.as_str())
                .collect::<Vec<_>>(),
            vec!["Browser:Parent", "Browser:Derived"]
        );
        let by_footprint = index.query(&KicadSymbolLibraryIndexQuery {
            footprint: Some("Connector Foo:Bar".to_string()),
            ..Default::default()
        });
        assert_eq!(by_footprint.symbols.len(), 2);
        assert_eq!(by_footprint.libraries[0].symbol_count, 2);
        let by_library = index.query(&KicadSymbolLibraryIndexQuery {
            library: Some("missing".to_string()),
            ..Default::default()
        });
        assert!(by_library.symbols.is_empty());
        assert!(by_library.libraries.is_empty());

        let library = read_kicad_symbol_library(&project_dir.join("browser.kicad_sym")).unwrap();
        let parent = library.symbol("Parent").unwrap();
        assert_eq!(parent.description(), Some("Parent analog switch"));
        assert_eq!(parent.keywords(), Some("switch analog mux"));
        assert_eq!(
            parent.footprint_filters(),
            vec![
                "Package_SO:SOIC-*".to_string(),
                "Connector Foo:*".to_string()
            ]
        );
        let exported = library.to_kicad_symbol_library_sexpr();
        assert!(exported.contains("(property \"Description\" \"Parent analog switch\""));
        assert!(exported.contains("(property \"ki_keywords\" \"switch analog mux\""));
        assert!(
            exported.contains(
                "(property \"ki_fp_filters\" \"Package_SO:SOIC-* Connector{space}Foo:*\""
            )
        );
        let reparsed =
            parse_kicad_symbol_library(&exported, "browser_roundtrip.kicad_sym").unwrap();
        assert_eq!(
            reparsed.symbol("Parent").unwrap().footprint_filters(),
            vec![
                "Package_SO:SOIC-*".to_string(),
                "Connector Foo:*".to_string()
            ]
        );

        let _ = fs::remove_dir_all(project_dir);
    }

    #[test]
    fn builds_symbol_library_preview_canvas_scene() {
        let library = parse_kicad_symbol_library(
            r#"(kicad_symbol_lib
  (version 20230121)
  (generator "NekoSpice")
  (symbol "Parent"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Parent" (at 0 -2.54 0))
    (symbol "Parent_0_1"
      (rectangle (start -1 -1) (end 1 1) (stroke (width 0.127) (type default)) (fill (type none)))
    )
  )
  (symbol "Derived"
    (extends "Parent")
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Derived" (at 0 -2.54 0))
    (symbol "Derived_1_1"
      (unit_name "Logic")
      (pin passive line (at -2.54 0 0) (length 2.54) (name "A") (number "1"))
    )
  )
)"#,
            "preview.kicad_sym",
        )
        .unwrap();
        let symbol = library.symbol_by_name_or_local_name("Derived").unwrap();
        let scene = KicadCanvasScene::from_symbol_definition(
            "preview.kicad_sym:Derived",
            symbol,
            &library.symbols,
            Some(1),
            None,
        );

        assert_eq!(scene.source, "preview.kicad_sym:Derived");
        assert_eq!(scene.symbols.len(), 1);
        assert_eq!(scene.symbols[0].lib_id, "Derived");
        assert_eq!(scene.symbols[0].value, "Derived");
        assert_eq!(scene.symbols[0].unit_name.as_deref(), Some("Logic"));
        assert_eq!(scene.symbols[0].graphics.len(), 1);
        assert_eq!(scene.symbols[0].pins.len(), 1);
        assert!(scene.bounds.is_some());
        assert!(scene.to_summary_json().contains("\"symbol_count\": 1"));
        let json: serde_json::Value = serde_json::from_str(&scene.to_json()).unwrap();
        assert_eq!(json["symbol_count"], 1);
        assert_eq!(json["symbols"][0]["lib_id"], "Derived");
        assert_eq!(json["symbols"][0]["unit_name"], "Logic");
        assert_eq!(json["symbols"][0]["pins"][0]["number"], "1");
        assert_eq!(json["symbols"][0]["graphics"][0]["kind"], "rectangle");
    }

    #[test]
    fn indexes_kicad_library_table_diagnostics() {
        let table = parse_kicad_symbol_library_table(
            r#"(sym_lib_table
  (version 7)
  (lib (name "Disabled")(type "KiCad")(uri "disabled.kicad_sym")(options "")(descr "")(disabled))
  (lib (name "Future")(type "FutureCAD")(uri "future.kicad_sym")(options "")(descr ""))
)"#,
            "inline",
        )
        .unwrap();

        let index = super::KicadSymbolLibraryIndex::from_table(table, Path::new("."));
        assert_eq!(index.libraries.len(), 0);
        assert_eq!(index.symbols.len(), 0);
        assert_eq!(index.diagnostics.len(), 2);
        assert!(
            index
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == "library row is disabled")
        );
        assert!(index.diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("unsupported symbol library type")
        }));
    }

    #[test]
    fn parses_quoted_strings_and_comments() {
        let parsed =
            parse_sexpr("(root ; comment\n  \"quoted value\" (child \"a\\\\b\"))").unwrap();
        let items = match parsed {
            super::Sexp::List(items) => items,
            super::Sexp::Atom(_) => panic!("root should be a list"),
        };

        assert_eq!(items.len(), 3);
    }

    #[test]
    fn rejects_wrong_kicad_root() {
        let error = parse_kicad_schematic("(kicad_symbol_lib)", "bad.kicad_sch").unwrap_err();
        assert!(error.to_string().contains("expected KiCad root"));

        let error = parse_kicad_symbol_library("(kicad_sch)", "bad.kicad_sym").unwrap_err();
        assert!(error.to_string().contains("expected KiCad root"));

        let error = parse_kicad_symbol_library_table("(kicad_sch)", "sym-lib-table").unwrap_err();
        assert!(error.to_string().contains("expected KiCad root"));
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {actual} to be close to {expected}"
        );
    }
}
