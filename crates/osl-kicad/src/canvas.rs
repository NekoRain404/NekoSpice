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
                reference_at: None,
                reference_effects: None,
                value_at: None,
                value_effects: None,
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

                // Extract property positions for reference and value labels
                let (reference_at, reference_effects, value_at, value_effects) = {
                    let ref_prop = symbol.properties.iter().find(|p| p.name == "Reference");
                    let val_prop = symbol.properties.iter().find(|p| p.name == "Value");
                    (
                        ref_prop.and_then(|p| p.at),
                        ref_prop.and_then(|p| p.effects.clone()),
                        val_prop.and_then(|p| p.at),
                        val_prop.and_then(|p| p.effects.clone()),
                    )
                };

                Some(KicadCanvasSymbol {
                    uuid: symbol.uuid.clone(),
                    lib_id: symbol.lib_id.clone(),
                    reference: symbol.reference().unwrap_or_default().to_string(),
                    value: symbol.value().unwrap_or_default().to_string(),
                    reference_at,
                    reference_effects,
                    value_at,
                    value_effects,
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

include!("canvas_items.rs");
