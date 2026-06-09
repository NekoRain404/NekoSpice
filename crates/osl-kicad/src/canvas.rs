use crate::geometry::{
    KICAD_CANVAS_LINE_BOUNDS_PADDING, KICAD_CANVAS_POINT_BOUNDS_RADIUS, KicadBoundingBoxBuilder,
    kicad_arc_hits_point, kicad_at_bounds, kicad_bezier_hits_point, kicad_circle_hits_point,
    kicad_closed_polyline_hits_point, kicad_fill_is_solid, kicad_junction_bounds,
    kicad_no_connect_bounds, kicad_point_bounds, kicad_points_bounds, kicad_polygon_contains_point,
    kicad_polyline_hits_point, kicad_rectangle_hits_point, kicad_sheet_pin_bounds,
    kicad_text_bounds, pin_body_end, sample_kicad_arc_points,
};
use crate::json::{kicad_bounding_box_json, kicad_bounding_box_value, kicad_property_value};
use crate::transform::transform_local_point;
use crate::{
    KicadAt, KicadBoundingBox, KicadColor, KicadFill, KicadLabelKind, KicadMargins,
    KicadPinAlternate, KicadPinDef, KicadPinDisplay, KicadPoint, KicadProperty,
    KicadResolvedSymbolDef, KicadSchematic, KicadSize, KicadStroke, KicadSymbolDef,
    KicadTextEffects, kicad_at_value, kicad_color_value, kicad_fill_value, kicad_margins_value,
    kicad_pin_alternate_value, kicad_pin_display_value, kicad_point_value, kicad_points_value,
    kicad_size_value, kicad_stroke_value, kicad_text_effects_value, resolve_symbol_definition,
};
use osl_core::json_escape;

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasScene {
    pub source: String,
    pub symbols: Vec<KicadCanvasSymbol>,
    pub sheets: Vec<KicadCanvasSheet>,
    pub graphics: Vec<KicadCanvasGraphic>,
    pub images: Vec<KicadCanvasImage>,
    pub tables: Vec<KicadCanvasTable>,
    pub rule_areas: Vec<KicadCanvasRuleArea>,
    pub wires: Vec<KicadCanvasWire>,
    pub buses: Vec<KicadCanvasBus>,
    pub bus_entries: Vec<KicadCanvasBusEntry>,
    pub directive_labels: Vec<KicadCanvasDirectiveLabel>,
    pub labels: Vec<KicadCanvasLabel>,
    pub text_items: Vec<KicadCanvasText>,
    pub text_boxes: Vec<KicadCanvasTextBox>,
    pub junctions: Vec<KicadCanvasJunction>,
    pub no_connects: Vec<KicadCanvasNoConnect>,
    pub groups: Vec<KicadCanvasGroup>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasScene {
    pub fn from_symbol_definition(
        source: impl Into<String>,
        symbol: &KicadSymbolDef,
        library_symbols: &[KicadSymbolDef],
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> Self {
        Self::from_symbol_definition_at(
            source,
            symbol,
            library_symbols,
            KicadAt {
                x: 0.0,
                y: 0.0,
                rotation: 0.0,
            },
            unit,
            body_style,
        )
    }

    pub fn from_symbol_definition_at(
        source: impl Into<String>,
        symbol: &KicadSymbolDef,
        library_symbols: &[KicadSymbolDef],
        at: KicadAt,
        unit: Option<u32>,
        body_style: Option<u32>,
    ) -> Self {
        let definition = resolve_symbol_definition(symbol, library_symbols)
            .unwrap_or_else(|| KicadResolvedSymbolDef::from_symbol(symbol));
        let graphics = definition
            .scoped_graphics(unit, body_style)
            .map(|graphic| graphic.transformed(at, None))
            .collect::<Vec<_>>();
        let pins = definition
            .scoped_pins(unit, body_style)
            .filter_map(|pin| KicadCanvasPin::from_pin_def(pin, at, None))
            .collect::<Vec<_>>();
        let symbol_bounds = canvas_symbol_bounds(&graphics, &pins);
        let mut bounds = KicadBoundingBoxBuilder::default();
        if let Some(symbol_bounds) = symbol_bounds {
            bounds.include_box(symbol_bounds);
        }

        let selected_unit = unit.unwrap_or(1);
        Self {
            source: source.into(),
            symbols: vec![KicadCanvasSymbol {
                uuid: None,
                lib_id: symbol.name.clone(),
                reference: symbol
                    .property("Reference")
                    .filter(|reference| !reference.is_empty())
                    .unwrap_or("U")
                    .to_string(),
                value: symbol
                    .property("Value")
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| symbol.local_name())
                    .to_string(),
                at,
                mirror: None,
                graphics,
                pins,
                pin_names: definition.pin_names.clone(),
                pin_numbers: definition.pin_numbers.clone(),
                unit_name: definition.unit_names.get(&selected_unit).cloned(),
                bounds: symbol_bounds,
            }],
            sheets: Vec::new(),
            graphics: Vec::new(),
            images: Vec::new(),
            tables: Vec::new(),
            rule_areas: Vec::new(),
            wires: Vec::new(),
            buses: Vec::new(),
            bus_entries: Vec::new(),
            directive_labels: Vec::new(),
            labels: Vec::new(),
            text_items: Vec::new(),
            text_boxes: Vec::new(),
            junctions: Vec::new(),
            no_connects: Vec::new(),
            groups: Vec::new(),
            bounds: bounds.finish(),
        }
    }

    pub fn from_schematic(schematic: &KicadSchematic) -> Self {
        let mut bounds = KicadBoundingBoxBuilder::default();

        let symbols = schematic
            .symbols
            .iter()
            .filter_map(|symbol| {
                let definition = schematic.resolved_symbol_definition(&symbol.lib_id)?;
                let at = symbol.at.unwrap_or(KicadAt {
                    x: 0.0,
                    y: 0.0,
                    rotation: 0.0,
                });
                let graphics = definition
                    .scoped_graphics(symbol.unit, symbol.body_style)
                    .map(|graphic| graphic.transformed(at, symbol.mirror.as_deref()))
                    .collect::<Vec<_>>();
                let pins = definition
                    .scoped_pins(symbol.unit, symbol.body_style)
                    .filter_map(|pin| {
                        KicadCanvasPin::from_pin_def(pin, at, symbol.mirror.as_deref())
                    })
                    .collect::<Vec<_>>();
                let symbol_bounds = canvas_symbol_bounds(&graphics, &pins);
                if let Some(symbol_bounds) = symbol_bounds {
                    bounds.include_box(symbol_bounds);
                }

                Some(KicadCanvasSymbol {
                    uuid: symbol.uuid.clone(),
                    lib_id: symbol.lib_id.clone(),
                    reference: symbol.reference().unwrap_or_default().to_string(),
                    value: symbol.value().unwrap_or_default().to_string(),
                    at,
                    mirror: symbol.mirror.clone(),
                    graphics,
                    pins,
                    pin_names: definition.pin_names.clone(),
                    pin_numbers: definition.pin_numbers.clone(),
                    unit_name: symbol
                        .unit
                        .and_then(|unit| definition.unit_names.get(&unit))
                        .cloned(),
                    bounds: symbol_bounds,
                })
            })
            .collect::<Vec<_>>();

        let sheets = schematic
            .sheets
            .iter()
            .map(|sheet| {
                let mut sheet_bounds = KicadBoundingBoxBuilder::default();
                if let Some(sheet_box) = sheet.bounding_box() {
                    sheet_bounds.include_box(sheet_box);
                    bounds.include_box(sheet_box);
                }
                let pins = sheet
                    .pins
                    .iter()
                    .map(|pin| {
                        let pin_bounds = pin.at.and_then(kicad_sheet_pin_bounds);
                        if let Some(pin_bounds) = pin_bounds {
                            sheet_bounds.include_box(pin_bounds);
                            bounds.include_box(pin_bounds);
                        }
                        KicadCanvasSheetPin {
                            uuid: pin.uuid.clone(),
                            name: pin.name.clone(),
                            pin_type: pin.pin_type.clone(),
                            at: pin.at,
                            bounds: pin_bounds,
                            effects: pin.effects.clone(),
                        }
                    })
                    .collect();
                KicadCanvasSheet {
                    uuid: sheet.uuid.clone(),
                    name: sheet.sheet_name().unwrap_or_default().to_string(),
                    file: sheet.sheet_file().unwrap_or_default().to_string(),
                    at: sheet.at,
                    size: sheet.size,
                    stroke: sheet.stroke.clone(),
                    fill: sheet.fill.clone(),
                    pins,
                    bounds: sheet_bounds.finish(),
                }
            })
            .collect::<Vec<_>>();

        let wires = schematic
            .wires
            .iter()
            .map(|wire| {
                for point in &wire.points {
                    bounds.include(*point);
                }
                KicadCanvasWire {
                    uuid: wire.uuid.clone(),
                    points: wire.points.clone(),
                    stroke: wire.stroke.clone(),
                    bounds: kicad_points_bounds(&wire.points, KICAD_CANVAS_LINE_BOUNDS_PADDING),
                }
            })
            .collect::<Vec<_>>();

        let graphics = schematic
            .graphics
            .iter()
            .map(|graphic| {
                let canvas_graphic = graphic.to_canvas_graphic();
                canvas_graphic.include_in_bounds(&mut bounds);
                canvas_graphic
            })
            .collect::<Vec<_>>();

        let images = schematic
            .images
            .iter()
            .map(|image| {
                let image_size = image.image_size_mm();
                let image_bounds = image.bounding_box().or_else(|| {
                    image
                        .at
                        .map(|at| kicad_point_bounds(at, KICAD_CANVAS_POINT_BOUNDS_RADIUS))
                });
                if let Some(image_bounds) = image_bounds {
                    bounds.include_box(image_bounds);
                } else if let Some(at) = image.at {
                    bounds.include(at);
                }
                KicadCanvasImage {
                    uuid: image.uuid.clone(),
                    at: image.at,
                    scale: image.scale,
                    data_base64: image.data_base64.clone(),
                    mime_type: image.mime_type().to_string(),
                    image_size,
                    bounds: image_bounds,
                }
            })
            .collect::<Vec<_>>();

        let tables = schematic
            .tables
            .iter()
            .map(|table| {
                let mut table_bounds = KicadBoundingBoxBuilder::default();
                let cells = table
                    .cells
                    .iter()
                    .map(|cell| {
                        let cell_bounds = cell
                            .bounding_box()
                            .or_else(|| kicad_at_bounds(cell.at, KICAD_CANVAS_POINT_BOUNDS_RADIUS));
                        if let Some(cell_bounds) = cell_bounds {
                            table_bounds.include_box(cell_bounds);
                            bounds.include_box(cell_bounds);
                        }
                        KicadCanvasTableCell {
                            uuid: cell.uuid.clone(),
                            text: cell.text.clone(),
                            at: cell.at,
                            size: cell.size,
                            margins: cell.margins,
                            column_span: cell.column_span,
                            row_span: cell.row_span,
                            fill: cell.fill.clone(),
                            effects: cell.effects.clone(),
                            bounds: cell_bounds,
                        }
                    })
                    .collect::<Vec<_>>();
                KicadCanvasTable {
                    uuid: table.uuid.clone(),
                    column_count: table.column_count,
                    column_widths: table.column_widths.clone(),
                    row_heights: table.row_heights.clone(),
                    cells,
                    bounds: table_bounds.finish(),
                }
            })
            .collect::<Vec<_>>();

        let rule_areas = schematic
            .rule_areas
            .iter()
            .map(|rule_area| {
                for point in &rule_area.points {
                    bounds.include(*point);
                }
                KicadCanvasRuleArea {
                    uuid: rule_area.uuid.clone(),
                    points: rule_area.points.clone(),
                    stroke: rule_area.stroke.clone(),
                    fill: rule_area.fill.clone(),
                    bounds: kicad_points_bounds(
                        &rule_area.points,
                        KICAD_CANVAS_LINE_BOUNDS_PADDING,
                    ),
                }
            })
            .collect::<Vec<_>>();

        let buses = schematic
            .buses
            .iter()
            .map(|bus| {
                for point in &bus.points {
                    bounds.include(*point);
                }
                KicadCanvasBus {
                    uuid: bus.uuid.clone(),
                    points: bus.points.clone(),
                    stroke: bus.stroke.clone(),
                    bounds: kicad_points_bounds(&bus.points, KICAD_CANVAS_LINE_BOUNDS_PADDING),
                }
            })
            .collect::<Vec<_>>();

        let bus_entries = schematic
            .bus_entries
            .iter()
            .map(|entry| {
                bounds.include(entry.at);
                bounds.include(entry.end());
                let entry_points = [entry.at, entry.end()];
                KicadCanvasBusEntry {
                    uuid: entry.uuid.clone(),
                    at: entry.at,
                    size: entry.size,
                    stroke: entry.stroke.clone(),
                    bounds: kicad_points_bounds(&entry_points, KICAD_CANVAS_LINE_BOUNDS_PADDING),
                }
            })
            .collect::<Vec<_>>();

        let labels = schematic
            .labels
            .iter()
            .map(|label| {
                let label_bounds = kicad_text_bounds(&label.text, label.at, label.effects.as_ref());
                if let Some(label_bounds) = label_bounds {
                    bounds.include_box(label_bounds);
                }
                KicadCanvasLabel {
                    uuid: label.uuid.clone(),
                    text: label.text.clone(),
                    kind: label.kind,
                    at: label.at,
                    effects: label.effects.clone(),
                    bounds: label_bounds,
                }
            })
            .collect::<Vec<_>>();

        let directive_labels = schematic
            .directive_labels
            .iter()
            .map(|label| {
                let mut label_bounds = label
                    .at
                    .map(|at| {
                        let pin_end = pin_body_end(at, label.length.unwrap_or(2.54));
                        kicad_points_bounds(
                            &[at.point(), pin_end],
                            KICAD_CANVAS_LINE_BOUNDS_PADDING,
                        )
                        .expect("directive label bounds use two points")
                    })
                    .or_else(|| {
                        kicad_text_bounds(label.display_text(), label.at, label.effects.as_ref())
                    });
                if let Some(text_bounds) =
                    kicad_text_bounds(label.display_text(), label.at, label.effects.as_ref())
                {
                    label_bounds = Some(match label_bounds {
                        Some(bounds) => bounds.union(text_bounds),
                        None => text_bounds,
                    });
                }
                if let Some(label_bounds) = label_bounds {
                    bounds.include_box(label_bounds);
                } else if let Some(at) = label.at {
                    let pin_end = pin_body_end(at, label.length.unwrap_or(2.54));
                    bounds.include(at.point());
                    bounds.include(pin_end);
                }
                KicadCanvasDirectiveLabel {
                    uuid: label.uuid.clone(),
                    text: label.display_text().to_string(),
                    at: label.at,
                    length: label.length,
                    shape: label.shape.clone(),
                    effects: label.effects.clone(),
                    properties: label.properties.clone(),
                    bounds: label_bounds,
                }
            })
            .collect::<Vec<_>>();

        let text_items = schematic
            .text_items
            .iter()
            .map(|text| {
                let text_bounds = kicad_text_bounds(&text.text, text.at, text.effects.as_ref());
                if let Some(text_bounds) = text_bounds {
                    bounds.include_box(text_bounds);
                }
                KicadCanvasText {
                    uuid: text.uuid.clone(),
                    text: text.text.clone(),
                    at: text.at,
                    is_spice_directive: text.text.trim_start().starts_with('.'),
                    effects: text.effects.clone(),
                    bounds: text_bounds,
                }
            })
            .collect::<Vec<_>>();

        let text_boxes = schematic
            .text_boxes
            .iter()
            .map(|text_box| {
                let text_box_bounds = text_box
                    .bounding_box()
                    .or_else(|| kicad_at_bounds(text_box.at, KICAD_CANVAS_POINT_BOUNDS_RADIUS));
                if let Some(text_box_bounds) = text_box_bounds {
                    bounds.include_box(text_box_bounds);
                }
                KicadCanvasTextBox {
                    uuid: text_box.uuid.clone(),
                    text: text_box.text.clone(),
                    at: text_box.at,
                    size: text_box.size,
                    margins: text_box.margins,
                    stroke: text_box.stroke.clone(),
                    fill: text_box.fill.clone(),
                    effects: text_box.effects.clone(),
                    bounds: text_box_bounds,
                }
            })
            .collect::<Vec<_>>();

        let junctions = schematic
            .junctions
            .iter()
            .map(|junction| {
                bounds.include(junction.at);
                KicadCanvasJunction {
                    uuid: junction.uuid.clone(),
                    at: junction.at,
                    diameter: junction.diameter,
                    color: junction.color,
                    bounds: kicad_junction_bounds(junction.at, junction.diameter),
                }
            })
            .collect::<Vec<_>>();

        let no_connects = schematic
            .no_connects
            .iter()
            .map(|marker| {
                bounds.include(marker.at);
                KicadCanvasNoConnect {
                    uuid: marker.uuid.clone(),
                    at: marker.at,
                    bounds: kicad_no_connect_bounds(marker.at),
                }
            })
            .collect::<Vec<_>>();

        let item_bounds = kicad_canvas_item_bounds(
            &symbols,
            &sheets,
            &graphics,
            &images,
            &tables,
            &rule_areas,
            &wires,
            &buses,
            &bus_entries,
            &directive_labels,
            &labels,
            &text_items,
            &text_boxes,
            &junctions,
            &no_connects,
        );
        let groups = schematic
            .groups
            .iter()
            .map(|group| {
                let mut group_bounds = KicadBoundingBoxBuilder::default();
                for member in &group.members {
                    if let Some(bounds) = item_bounds.get(member) {
                        group_bounds.include_box(*bounds);
                    }
                }
                KicadCanvasGroup {
                    uuid: group.uuid.clone(),
                    name: group.name.clone(),
                    locked: group.locked,
                    members: group.members.clone(),
                    bounds: group_bounds.finish(),
                }
            })
            .collect::<Vec<_>>();

        Self {
            source: schematic.source.clone(),
            symbols,
            sheets,
            graphics,
            images,
            tables,
            rule_areas,
            wires,
            buses,
            bus_entries,
            directive_labels,
            labels,
            text_items,
            text_boxes,
            junctions,
            no_connects,
            groups,
            bounds: bounds.finish(),
        }
    }

    pub fn to_summary_json(&self) -> String {
        let bounds = self
            .bounds
            .map(kicad_bounding_box_json)
            .unwrap_or_else(|| "null".to_string());
        let symbol_graphic_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.graphics.len())
            .sum::<usize>();
        let graphic_count = symbol_graphic_count + self.graphics.len();
        let pin_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.pins.len())
            .sum::<usize>();

        format!(
            concat!(
                "{{\n",
                "  \"source\": \"{}\",\n",
                "  \"symbol_count\": {},\n",
                "  \"sheet_count\": {},\n",
                "  \"graphic_count\": {},\n",
                "  \"schematic_graphic_count\": {},\n",
                "  \"image_count\": {},\n",
                "  \"table_count\": {},\n",
                "  \"table_cell_count\": {},\n",
                "  \"rule_area_count\": {},\n",
                "  \"pin_count\": {},\n",
                "  \"sheet_pin_count\": {},\n",
                "  \"wire_count\": {},\n",
                "  \"bus_count\": {},\n",
                "  \"bus_entry_count\": {},\n",
                "  \"directive_label_count\": {},\n",
                "  \"label_count\": {},\n",
                "  \"text_count\": {},\n",
                "  \"text_box_count\": {},\n",
                "  \"spice_directive_count\": {},\n",
                "  \"junction_count\": {},\n",
                "  \"no_connect_count\": {},\n",
                "  \"group_count\": {},\n",
                "  \"group_member_count\": {},\n",
                "  \"bounds\": {}\n",
                "}}"
            ),
            json_escape(&self.source),
            self.symbols.len(),
            self.sheets.len(),
            graphic_count,
            self.graphics.len(),
            self.images.len(),
            self.tables.len(),
            self.tables
                .iter()
                .map(|table| table.cells.len())
                .sum::<usize>(),
            self.rule_areas.len(),
            pin_count,
            self.sheets
                .iter()
                .map(|sheet| sheet.pins.len())
                .sum::<usize>(),
            self.wires.len(),
            self.buses.len(),
            self.bus_entries.len(),
            self.directive_labels.len(),
            self.labels.len(),
            self.text_items.len(),
            self.text_boxes.len(),
            self.text_items
                .iter()
                .filter(|item| item.is_spice_directive)
                .count(),
            self.junctions.len(),
            self.no_connects.len(),
            self.groups.len(),
            self.groups
                .iter()
                .map(|group| group.members.len())
                .sum::<usize>(),
            bounds
        )
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_json_value())
            .expect("KiCad canvas scene JSON should serialize")
    }

    fn to_json_value(&self) -> serde_json::Value {
        let symbol_graphic_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.graphics.len())
            .sum::<usize>();
        let pin_count = self
            .symbols
            .iter()
            .map(|symbol| symbol.pins.len())
            .sum::<usize>();

        serde_json::json!({
            "source": self.source,
            "symbol_count": self.symbols.len(),
            "sheet_count": self.sheets.len(),
            "graphic_count": symbol_graphic_count + self.graphics.len(),
            "schematic_graphic_count": self.graphics.len(),
            "image_count": self.images.len(),
            "table_count": self.tables.len(),
            "table_cell_count": self.tables.iter().map(|table| table.cells.len()).sum::<usize>(),
            "rule_area_count": self.rule_areas.len(),
            "pin_count": pin_count,
            "sheet_pin_count": self.sheets.iter().map(|sheet| sheet.pins.len()).sum::<usize>(),
            "wire_count": self.wires.len(),
            "bus_count": self.buses.len(),
            "bus_entry_count": self.bus_entries.len(),
            "directive_label_count": self.directive_labels.len(),
            "label_count": self.labels.len(),
            "text_count": self.text_items.len(),
            "text_box_count": self.text_boxes.len(),
            "spice_directive_count": self.text_items.iter().filter(|item| item.is_spice_directive).count(),
            "junction_count": self.junctions.len(),
            "no_connect_count": self.no_connects.len(),
            "group_count": self.groups.len(),
            "group_member_count": self.groups.iter().map(|group| group.members.len()).sum::<usize>(),
            "bounds": self.bounds.map(kicad_bounding_box_value),
            "symbols": self.symbols.iter().map(KicadCanvasSymbol::to_json_value).collect::<Vec<_>>(),
            "sheets": self.sheets.iter().map(KicadCanvasSheet::to_json_value).collect::<Vec<_>>(),
            "graphics": self.graphics.iter().map(KicadCanvasGraphic::to_json_value).collect::<Vec<_>>(),
            "images": self.images.iter().map(KicadCanvasImage::to_json_value).collect::<Vec<_>>(),
            "tables": self.tables.iter().map(KicadCanvasTable::to_json_value).collect::<Vec<_>>(),
            "rule_areas": self.rule_areas.iter().map(KicadCanvasRuleArea::to_json_value).collect::<Vec<_>>(),
            "wires": self.wires.iter().map(KicadCanvasWire::to_json_value).collect::<Vec<_>>(),
            "buses": self.buses.iter().map(KicadCanvasBus::to_json_value).collect::<Vec<_>>(),
            "bus_entries": self.bus_entries.iter().map(KicadCanvasBusEntry::to_json_value).collect::<Vec<_>>(),
            "directive_labels": self.directive_labels.iter().map(KicadCanvasDirectiveLabel::to_json_value).collect::<Vec<_>>(),
            "labels": self.labels.iter().map(KicadCanvasLabel::to_json_value).collect::<Vec<_>>(),
            "text_items": self.text_items.iter().map(KicadCanvasText::to_json_value).collect::<Vec<_>>(),
            "text_boxes": self.text_boxes.iter().map(KicadCanvasTextBox::to_json_value).collect::<Vec<_>>(),
            "junctions": self.junctions.iter().map(KicadCanvasJunction::to_json_value).collect::<Vec<_>>(),
            "no_connects": self.no_connects.iter().map(KicadCanvasNoConnect::to_json_value).collect::<Vec<_>>(),
            "groups": self.groups.iter().map(KicadCanvasGroup::to_json_value).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSymbol {
    pub uuid: Option<String>,
    pub lib_id: String,
    pub reference: String,
    pub value: String,
    pub at: KicadAt,
    pub mirror: Option<String>,
    pub graphics: Vec<KicadCanvasGraphic>,
    pub pins: Vec<KicadCanvasPin>,
    pub pin_names: Option<KicadPinDisplay>,
    pub pin_numbers: Option<KicadPinDisplay>,
    pub unit_name: Option<String>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasSymbol {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "lib_id": self.lib_id,
            "reference": self.reference,
            "value": self.value,
            "at": kicad_at_value(self.at),
            "mirror": self.mirror,
            "unit_name": self.unit_name,
            "pin_names": self.pin_names.as_ref().map(kicad_pin_display_value),
            "pin_numbers": self.pin_numbers.as_ref().map(kicad_pin_display_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
            "graphics": self.graphics.iter().map(KicadCanvasGraphic::to_json_value).collect::<Vec<_>>(),
            "pins": self.pins.iter().map(KicadCanvasPin::to_json_value).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSheet {
    pub uuid: Option<String>,
    pub name: String,
    pub file: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub pins: Vec<KicadCanvasSheetPin>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasSheet {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "file": self.file,
            "at": self.at.map(kicad_at_value),
            "size": self.size.map(kicad_size_value),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "pin_count": self.pins.len(),
            "pins": self.pins.iter().map(KicadCanvasSheetPin::to_json_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasSheetPin {
    pub uuid: Option<String>,
    pub name: String,
    pub pin_type: String,
    pub at: Option<KicadAt>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasSheetPin {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "pin_type": self.pin_type,
            "at": self.at.map(kicad_at_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasRuleArea {
    pub uuid: Option<String>,
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasRuleArea {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": kicad_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }

    pub(crate) fn hits_point(&self, point: KicadPoint) -> bool {
        if kicad_fill_is_solid(self.fill.as_ref())
            && kicad_polygon_contains_point(&self.points, point)
        {
            return true;
        }
        kicad_closed_polyline_hits_point(&self.points, self.stroke.as_ref(), point)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasDirectiveLabel {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub length: Option<f64>,
    pub shape: Option<String>,
    pub effects: Option<KicadTextEffects>,
    pub properties: Vec<KicadProperty>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasDirectiveLabel {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "length": self.length,
            "shape": self.shape,
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "properties": self.properties.iter().map(kicad_property_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KicadCanvasGraphic {
    Polyline {
        uuid: Option<String>,
        points: Vec<KicadPoint>,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Bezier {
        uuid: Option<String>,
        points: Vec<KicadPoint>,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Rectangle {
        uuid: Option<String>,
        start: KicadPoint,
        end: KicadPoint,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Circle {
        uuid: Option<String>,
        center: KicadPoint,
        radius: f64,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Arc {
        uuid: Option<String>,
        start: KicadPoint,
        mid: Option<KicadPoint>,
        end: KicadPoint,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
    Text {
        uuid: Option<String>,
        text: String,
        at: Option<KicadAt>,
        effects: Option<KicadTextEffects>,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    },
}

impl KicadCanvasGraphic {
    fn to_json_value(&self) -> serde_json::Value {
        match self {
            Self::Polyline {
                uuid,
                points,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "polyline",
                "uuid": uuid,
                "points": kicad_points_value(points),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Bezier {
                uuid,
                points,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "bezier",
                "uuid": uuid,
                "points": kicad_points_value(points),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Rectangle {
                uuid,
                start,
                end,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "rectangle",
                "uuid": uuid,
                "start": kicad_point_value(*start),
                "end": kicad_point_value(*end),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Circle {
                uuid,
                center,
                radius,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "circle",
                "uuid": uuid,
                "center": kicad_point_value(*center),
                "radius": radius,
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Arc {
                uuid,
                start,
                mid,
                end,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "arc",
                "uuid": uuid,
                "start": kicad_point_value(*start),
                "mid": mid.map(kicad_point_value),
                "end": kicad_point_value(*end),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
            Self::Text {
                uuid,
                text,
                at,
                effects,
                stroke,
                fill,
            } => serde_json::json!({
                "kind": "text",
                "uuid": uuid,
                "text": text,
                "at": at.map(kicad_at_value),
                "effects": effects.as_ref().map(kicad_text_effects_value),
                "stroke": stroke.as_ref().map(kicad_stroke_value),
                "fill": fill.as_ref().map(kicad_fill_value),
                "bounds": self.bounds().map(kicad_bounding_box_value),
            }),
        }
    }

    pub(crate) fn with_style(
        mut self,
        stroke: Option<KicadStroke>,
        fill: Option<KicadFill>,
    ) -> Self {
        match &mut self {
            Self::Polyline {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Bezier {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Rectangle {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Circle {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Arc {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            }
            | Self::Text {
                stroke: graphic_stroke,
                fill: graphic_fill,
                ..
            } => {
                *graphic_stroke = stroke;
                *graphic_fill = fill;
            }
        }
        self
    }

    pub(crate) fn with_uuid(mut self, uuid: Option<String>) -> Self {
        match &mut self {
            Self::Polyline {
                uuid: graphic_uuid, ..
            }
            | Self::Bezier {
                uuid: graphic_uuid, ..
            }
            | Self::Rectangle {
                uuid: graphic_uuid, ..
            }
            | Self::Circle {
                uuid: graphic_uuid, ..
            }
            | Self::Arc {
                uuid: graphic_uuid, ..
            }
            | Self::Text {
                uuid: graphic_uuid, ..
            } => {
                *graphic_uuid = uuid;
            }
        }
        self
    }

    pub(crate) fn uuid(&self) -> Option<String> {
        match self {
            Self::Polyline { uuid, .. }
            | Self::Bezier { uuid, .. }
            | Self::Rectangle { uuid, .. }
            | Self::Circle { uuid, .. }
            | Self::Arc { uuid, .. }
            | Self::Text { uuid, .. } => uuid.clone(),
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Polyline { .. } => "polyline",
            Self::Bezier { .. } => "bezier",
            Self::Rectangle { .. } => "rectangle",
            Self::Circle { .. } => "circle",
            Self::Arc { .. } => "arc",
            Self::Text { .. } => "text",
        }
    }

    pub(crate) fn hits_point(&self, point: KicadPoint) -> bool {
        match self {
            Self::Polyline { points, stroke, .. } => {
                kicad_polyline_hits_point(points, stroke.as_ref(), point)
            }
            Self::Bezier { points, stroke, .. } => {
                kicad_bezier_hits_point(points, stroke.as_ref(), point)
            }
            Self::Rectangle {
                start,
                end,
                stroke,
                fill,
                ..
            } => kicad_rectangle_hits_point(*start, *end, stroke.as_ref(), fill.as_ref(), point),
            Self::Circle {
                center,
                radius,
                stroke,
                fill,
                ..
            } => kicad_circle_hits_point(*center, *radius, stroke.as_ref(), fill.as_ref(), point),
            Self::Arc {
                start,
                mid,
                end,
                stroke,
                ..
            } => kicad_arc_hits_point(*start, *mid, *end, stroke.as_ref(), point),
            Self::Text {
                text, at, effects, ..
            } => kicad_text_bounds(text, *at, effects.as_ref())
                .is_some_and(|bounds| bounds.contains(point)),
        }
    }

    pub(crate) fn include_in_bounds(&self, bounds: &mut KicadBoundingBoxBuilder) {
        match self {
            Self::Polyline { points, .. } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Bezier { points, .. } => {
                for point in points {
                    bounds.include(*point);
                }
            }
            Self::Rectangle { start, end, .. } => {
                bounds.include(*start);
                bounds.include(*end);
            }
            Self::Circle { center, radius, .. } => {
                bounds.include(KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                });
                bounds.include(KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                });
            }
            Self::Arc {
                start, mid, end, ..
            } => {
                for point in sample_kicad_arc_points(*start, *mid, *end) {
                    bounds.include(point);
                }
            }
            Self::Text {
                text, at, effects, ..
            } => {
                if let Some(text_bounds) = kicad_text_bounds(text, *at, effects.as_ref()) {
                    bounds.include_box(text_bounds);
                }
            }
        }
    }

    pub fn bounds(&self) -> Option<KicadBoundingBox> {
        match self {
            Self::Polyline { points, .. } | Self::Bezier { points, .. } => {
                kicad_points_bounds(points, KICAD_CANVAS_LINE_BOUNDS_PADDING)
            }
            Self::Rectangle { start, end, .. } => {
                kicad_points_bounds(&[*start, *end], KICAD_CANVAS_LINE_BOUNDS_PADDING)
            }
            Self::Circle { center, radius, .. } => Some(KicadBoundingBox {
                min: KicadPoint {
                    x: center.x - radius,
                    y: center.y - radius,
                },
                max: KicadPoint {
                    x: center.x + radius,
                    y: center.y + radius,
                },
            }),
            Self::Arc {
                start, mid, end, ..
            } => kicad_points_bounds(
                &sample_kicad_arc_points(*start, *mid, *end),
                KICAD_CANVAS_LINE_BOUNDS_PADDING,
            ),
            Self::Text {
                text, at, effects, ..
            } => kicad_text_bounds(text, *at, effects.as_ref()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasImage {
    pub uuid: Option<String>,
    pub at: Option<KicadPoint>,
    pub scale: f64,
    pub data_base64: String,
    pub mime_type: String,
    pub image_size: Option<KicadSize>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasImage {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": self.at.map(kicad_point_value),
            "scale": self.scale,
            "mime_type": self.mime_type,
            "image_size": self.image_size.map(kicad_size_value),
            "data_base64": self.data_base64,
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasTable {
    pub uuid: Option<String>,
    pub column_count: usize,
    pub column_widths: Vec<f64>,
    pub row_heights: Vec<f64>,
    pub cells: Vec<KicadCanvasTableCell>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasTable {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "column_count": self.column_count,
            "column_widths": self.column_widths,
            "row_heights": self.row_heights,
            "cell_count": self.cells.len(),
            "cells": self.cells.iter().map(KicadCanvasTableCell::to_json_value).collect::<Vec<_>>(),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasTableCell {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub column_span: usize,
    pub row_span: usize,
    pub fill: Option<KicadFill>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasTableCell {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "size": self.size.map(kicad_size_value),
            "margins": self.margins.map(kicad_margins_value),
            "column_span": self.column_span,
            "row_span": self.row_span,
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasPin {
    pub number: String,
    pub name: String,
    pub electrical_type: String,
    pub start: KicadPoint,
    pub end: KicadPoint,
    pub alternates: Vec<KicadPinAlternate>,
    pub name_effects: Option<KicadTextEffects>,
    pub number_effects: Option<KicadTextEffects>,
}

impl KicadCanvasPin {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "number": self.number,
            "name": self.name,
            "electrical_type": self.electrical_type,
            "start": kicad_point_value(self.start),
            "end": kicad_point_value(self.end),
            "alternate_count": self.alternates.len(),
            "name_effects": self.name_effects.as_ref().map(kicad_text_effects_value),
            "number_effects": self.number_effects.as_ref().map(kicad_text_effects_value),
            "alternates": self.alternates.iter().map(kicad_pin_alternate_value).collect::<Vec<_>>(),
            "bounds": kicad_points_bounds(&[self.start, self.end], KICAD_CANVAS_LINE_BOUNDS_PADDING).map(kicad_bounding_box_value),
        })
    }

    fn from_pin_def(pin: &KicadPinDef, symbol_at: KicadAt, mirror: Option<&str>) -> Option<Self> {
        let pin_at = pin.at?;
        let local_start = pin_at.point();
        let local_end = pin_body_end(pin_at, pin.length.unwrap_or(0.0));

        Some(Self {
            number: pin.number().to_string(),
            name: pin.name().to_string(),
            electrical_type: pin.electrical_type.clone(),
            start: transform_local_point(local_start, symbol_at, mirror),
            end: transform_local_point(local_end, symbol_at, mirror),
            alternates: pin.alternates.clone(),
            name_effects: pin.name_effects().cloned(),
            number_effects: pin.number_effects().cloned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasWire {
    pub uuid: Option<String>,
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasWire {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": kicad_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasBus {
    pub uuid: Option<String>,
    pub points: Vec<KicadPoint>,
    pub stroke: Option<KicadStroke>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasBus {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "points": kicad_points_value(&self.points),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasBusEntry {
    pub uuid: Option<String>,
    pub at: KicadPoint,
    pub size: KicadSize,
    pub stroke: Option<KicadStroke>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasBusEntry {
    pub fn end(&self) -> KicadPoint {
        KicadPoint {
            x: self.at.x + self.size.width,
            y: self.at.y + self.size.height,
        }
    }

    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": kicad_point_value(self.at),
            "size": kicad_size_value(self.size),
            "end": kicad_point_value(self.end()),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasLabel {
    pub uuid: Option<String>,
    pub text: String,
    pub kind: KicadLabelKind,
    pub at: Option<KicadAt>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasLabel {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "kind": self.kind.as_str(),
            "at": self.at.map(kicad_at_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasText {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub is_spice_directive: bool,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasText {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "is_spice_directive": self.is_spice_directive,
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasTextBox {
    pub uuid: Option<String>,
    pub text: String,
    pub at: Option<KicadAt>,
    pub size: Option<KicadSize>,
    pub margins: Option<KicadMargins>,
    pub stroke: Option<KicadStroke>,
    pub fill: Option<KicadFill>,
    pub effects: Option<KicadTextEffects>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasTextBox {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "text": self.text,
            "at": self.at.map(kicad_at_value),
            "size": self.size.map(kicad_size_value),
            "margins": self.margins.map(kicad_margins_value),
            "stroke": self.stroke.as_ref().map(kicad_stroke_value),
            "fill": self.fill.as_ref().map(kicad_fill_value),
            "effects": self.effects.as_ref().map(kicad_text_effects_value),
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasJunction {
    pub uuid: Option<String>,
    pub at: KicadPoint,
    pub diameter: Option<f64>,
    pub color: Option<KicadColor>,
    pub bounds: KicadBoundingBox,
}

impl KicadCanvasJunction {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": kicad_point_value(self.at),
            "diameter": self.diameter,
            "color": self.color.map(kicad_color_value),
            "bounds": kicad_bounding_box_value(self.bounds),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasNoConnect {
    pub uuid: Option<String>,
    pub at: KicadPoint,
    pub bounds: KicadBoundingBox,
}

impl KicadCanvasNoConnect {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "at": kicad_point_value(self.at),
            "bounds": kicad_bounding_box_value(self.bounds),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasGroup {
    pub uuid: Option<String>,
    pub name: String,
    pub locked: Option<bool>,
    pub members: Vec<String>,
    pub bounds: Option<KicadBoundingBox>,
}

impl KicadCanvasGroup {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "uuid": self.uuid,
            "name": self.name,
            "locked": self.locked,
            "member_count": self.members.len(),
            "members": self.members,
            "bounds": self.bounds.map(kicad_bounding_box_value),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn kicad_canvas_item_bounds(
    symbols: &[KicadCanvasSymbol],
    sheets: &[KicadCanvasSheet],
    graphics: &[KicadCanvasGraphic],
    images: &[KicadCanvasImage],
    tables: &[KicadCanvasTable],
    rule_areas: &[KicadCanvasRuleArea],
    wires: &[KicadCanvasWire],
    buses: &[KicadCanvasBus],
    bus_entries: &[KicadCanvasBusEntry],
    directive_labels: &[KicadCanvasDirectiveLabel],
    labels: &[KicadCanvasLabel],
    text_items: &[KicadCanvasText],
    text_boxes: &[KicadCanvasTextBox],
    junctions: &[KicadCanvasJunction],
    no_connects: &[KicadCanvasNoConnect],
) -> BTreeMap<String, KicadBoundingBox> {
    let mut item_bounds = BTreeMap::new();
    for symbol in symbols {
        insert_canvas_item_bounds(&mut item_bounds, symbol.uuid.as_deref(), symbol.bounds);
    }
    for sheet in sheets {
        insert_canvas_item_bounds(&mut item_bounds, sheet.uuid.as_deref(), sheet.bounds);
    }
    for graphic in graphics {
        insert_canvas_item_bounds(
            &mut item_bounds,
            graphic.uuid().as_deref(),
            graphic.bounds(),
        );
    }
    for image in images {
        insert_canvas_item_bounds(&mut item_bounds, image.uuid.as_deref(), image.bounds);
    }
    for table in tables {
        insert_canvas_item_bounds(&mut item_bounds, table.uuid.as_deref(), table.bounds);
        for cell in &table.cells {
            insert_canvas_item_bounds(&mut item_bounds, cell.uuid.as_deref(), cell.bounds);
        }
    }
    for rule_area in rule_areas {
        insert_canvas_item_bounds(
            &mut item_bounds,
            rule_area.uuid.as_deref(),
            rule_area.bounds,
        );
    }
    for wire in wires {
        insert_canvas_item_bounds(&mut item_bounds, wire.uuid.as_deref(), wire.bounds);
    }
    for bus in buses {
        insert_canvas_item_bounds(&mut item_bounds, bus.uuid.as_deref(), bus.bounds);
    }
    for entry in bus_entries {
        insert_canvas_item_bounds(&mut item_bounds, entry.uuid.as_deref(), entry.bounds);
    }
    for label in directive_labels {
        insert_canvas_item_bounds(&mut item_bounds, label.uuid.as_deref(), label.bounds);
    }
    for label in labels {
        insert_canvas_item_bounds(&mut item_bounds, label.uuid.as_deref(), label.bounds);
    }
    for text in text_items {
        insert_canvas_item_bounds(&mut item_bounds, text.uuid.as_deref(), text.bounds);
    }
    for text_box in text_boxes {
        insert_canvas_item_bounds(&mut item_bounds, text_box.uuid.as_deref(), text_box.bounds);
    }
    for junction in junctions {
        insert_canvas_item_bounds(
            &mut item_bounds,
            junction.uuid.as_deref(),
            Some(junction.bounds),
        );
    }
    for marker in no_connects {
        insert_canvas_item_bounds(
            &mut item_bounds,
            marker.uuid.as_deref(),
            Some(marker.bounds),
        );
    }
    item_bounds
}

fn canvas_symbol_bounds(
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

fn insert_canvas_item_bounds(
    item_bounds: &mut BTreeMap<String, KicadBoundingBox>,
    uuid: Option<&str>,
    bounds: Option<KicadBoundingBox>,
) {
    if let (Some(uuid), Some(bounds)) = (uuid, bounds) {
        item_bounds.insert(uuid.to_string(), bounds);
    }
}
