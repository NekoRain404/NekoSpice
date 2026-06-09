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
use geometry::KICAD_CANVAS_POINT_BOUNDS_RADIUS;
use geometry::{KICAD_CANVAS_LINE_BOUNDS_PADDING, KicadBoundingBoxBuilder, kicad_points_bounds};
use osl_core::{OslError, OslResult, read_text, write_text};
use pins::{compare_pin_numbers, kicad_pin_alternate_value, kicad_pin_display_value};
use sexpr::{child, format_number, list_value};
use sheet::sheet_properties;
use simulation::is_spice_analysis_directive_text;
use spice_export::spice_primitive_for_device;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};
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

pub fn read_kicad_schematic_hierarchy_netlist(path: &Path) -> OslResult<KicadHierarchyNetlist> {
    let schematic = read_kicad_schematic_with_libraries(path)?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    schematic.to_spice_netlist_with_hierarchy(base_dir)
}

pub fn read_kicad_project(path: &Path) -> OslResult<KicadProject> {
    let content = read_text(path)?;
    parse_kicad_project(&content, &path.display().to_string())
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
            KicadSchematicEdit::SetSimulationDirective {
                kind,
                body,
                at,
                uuid,
            } => self.set_simulation_directive(KicadSimulationDirectiveUpdate {
                kind,
                body,
                at,
                uuid,
            }),
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
            .any(|directive| is_spice_analysis_directive_text(&directive.text))
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

    pub(crate) fn edit_uuid(
        &self,
        uuid: Option<String>,
        namespace: &str,
        payload: &str,
    ) -> OslResult<String> {
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

    pub(crate) fn used_uuids(&self) -> BTreeSet<String> {
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

    pub(crate) fn symbol_pin_points(&self) -> Vec<KicadPoint> {
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

    pub(crate) fn sheet_pin_points(&self) -> Vec<KicadPoint> {
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
pub(crate) fn parse_kicad_bool_value(value: String) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Some(true),
        "no" | "false" | "0" => Some(false),
        _ => None,
    }
}

pub(crate) fn parse_optional_bool_child(items: &[Sexp], name: &str) -> Option<bool> {
    child(items, name).map(|node| {
        list_value(node, 1)
            .and_then(parse_kicad_bool_value)
            .unwrap_or(true)
    })
}

#[cfg(test)]
mod tests {
    use super::{
        KicadAt, KicadBoundingBox, KicadCanvasScene, KicadColor, KicadDiagnosticSeverity,
        KicadGraphic, KicadIndexedSymbolBodyStyle, KicadIndexedSymbolUnit, KicadLabelKind,
        KicadPoint, KicadSchematicEdit, KicadSheetPin, KicadSimulationDirectiveKind, KicadSize,
        KicadSymbolBodyStyles, KicadSymbolLibraryIndexQuery, KicadSymbolPlacement,
        KicadSymbolPower, parse_kicad_project, parse_kicad_schematic, parse_kicad_symbol_library,
        parse_kicad_symbol_library_table, parse_sexpr, read_kicad_project, read_kicad_schematic,
        read_kicad_schematic_with_libraries, read_kicad_symbol_library,
        read_kicad_symbol_library_index, read_kicad_symbol_library_table,
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
    fn sets_structured_simulation_directives_and_roundtrips() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut schematic =
            read_kicad_schematic(&workspace_root.join("examples/kicad_schematic/rc.kicad_sch"))
                .unwrap();

        schematic
            .apply_edit(KicadSchematicEdit::SetSimulationDirective {
                kind: KicadSimulationDirectiveKind::Tran,
                body: "2u 2m".to_string(),
                at: Some(KicadAt {
                    x: 30.48,
                    y: 20.32,
                    rotation: 0.0,
                }),
                uuid: Some("aaaaaaaa-0000-4000-8000-000000000001".to_string()),
            })
            .unwrap();
        schematic
            .apply_edit(KicadSchematicEdit::SetSimulationDirective {
                kind: KicadSimulationDirectiveKind::Save,
                body: "v(out)".to_string(),
                at: Some(KicadAt {
                    x: 30.48,
                    y: 25.4,
                    rotation: 0.0,
                }),
                uuid: Some("aaaaaaaa-0000-4000-8000-000000000002".to_string()),
            })
            .unwrap();

        let directives = schematic.simulation_directives();
        assert!(directives.iter().any(|directive| {
            directive.kind == KicadSimulationDirectiveKind::Tran
                && directive.text == ".tran 2u 2m"
                && directive.uuid.as_deref() == Some("77777777-7777-7777-7777-777777777777")
        }));
        assert!(directives.iter().any(|directive| {
            directive.kind == KicadSimulationDirectiveKind::Save
                && directive.text == ".save v(out)"
                && directive.uuid.as_deref() == Some("aaaaaaaa-0000-4000-8000-000000000002")
        }));

        let exported = schematic.to_kicad_schematic_sexpr();
        assert!(exported.contains("(text \".tran 2u 2m\""));
        assert!(exported.contains("(text \".save v(out)\""));
        let reparsed = parse_kicad_schematic(&exported, "simulation_directives.kicad_sch").unwrap();
        assert!(reparsed.simulation_directives().iter().any(|directive| {
            directive.kind == KicadSimulationDirectiveKind::Save && directive.text == ".save v(out)"
        }));
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
