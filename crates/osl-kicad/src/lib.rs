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

#[cfg(test)]
mod tests;
