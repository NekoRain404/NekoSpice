//! Canvas hit testing — ray-cast selection for symbols, wires, labels, and other items.

use crate::geometry::{
    SCHEMA_SHEET_PIN_STUB_LENGTH, pin_body_end, schema_junction_radius, schema_no_connect_arms,
    schema_point_distance, schema_polyline_hits_point, schema_rotated_rect_contains_point,
    schema_sheet_box_bounds, schema_text_bounds,
};
use crate::json::schema_bounding_box_value;
use crate::{
    NspBoundingBox, NspCanvasDirectiveLabel, NspCanvasGraphic, NspCanvasJunction,
    NspCanvasNoConnect, NspCanvasPin, NspCanvasRuleArea, NspCanvasScene, NspCanvasSheet,
    NspCanvasSheetPin, NspCanvasSymbol, NspCanvasTable, NspCanvasTableCell, NspCanvasTextBox,
    NspPoint, NspStroke, schema_point_value,
};

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasHit {
    pub kind: String,
    pub uuid: Option<String>,
    pub label: String,
    pub bounds: NspBoundingBox,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspCanvasHitReport {
    pub source: String,
    pub at: NspPoint,
    pub hit_count: usize,
    pub hits: Vec<NspCanvasHit>,
}

impl NspCanvasHitReport {
    /// to json。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&serde_json::json!({
            "source": self.source,
            "at": schema_point_value(self.at),
            "hit_count": self.hit_count,
            "hits": self.hits.iter().map(NspCanvasHit::to_json_value).collect::<Vec<_>>(),
        }))
        .expect("schema canvas hit-test JSON should serialize")
    }
}

impl NspCanvasHit {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": self.kind,
            "uuid": self.uuid,
            "label": self.label,
            "bounds": schema_bounding_box_value(self.bounds),
            "area": self.bounds.area(),
        })
    }
}

impl NspCanvasScene {
    /// hit test。
    pub fn hit_test(&self, point: NspPoint) -> NspCanvasHitReport {
        let mut hits = Vec::new();
        for symbol in &self.symbols {
            push_canvas_symbol_hit(&mut hits, symbol, point);
        }
        for sheet in &self.sheets {
            push_canvas_sheet_hit(&mut hits, sheet, point);
            for pin in &sheet.pins {
                push_canvas_sheet_pin_hit(&mut hits, pin, point);
            }
        }
        for graphic in &self.graphics {
            push_canvas_graphic_hit(&mut hits, graphic, point);
        }
        for image in &self.images {
            push_canvas_hit(
                &mut hits,
                "image",
                image.uuid.clone(),
                image.mime_type.clone(),
                image.bounds,
                point,
            );
        }
        for table in &self.tables {
            push_canvas_table_hit(&mut hits, table, point);
            for cell in &table.cells {
                push_canvas_table_cell_hit(&mut hits, cell, point);
            }
        }
        for rule_area in &self.rule_areas {
            push_canvas_rule_area_hit(&mut hits, rule_area, point);
        }
        for wire in &self.wires {
            push_canvas_polyline_hit(
                &mut hits,
                NspCanvasPolylineHit {
                    kind: "wire",
                    uuid: wire.uuid.clone(),
                    label: "wire".to_string(),
                    bounds: wire.bounds,
                    points: &wire.points,
                    stroke: wire.stroke.as_ref(),
                },
                point,
            );
        }
        for bus in &self.buses {
            push_canvas_polyline_hit(
                &mut hits,
                NspCanvasPolylineHit {
                    kind: "bus",
                    uuid: bus.uuid.clone(),
                    label: "bus".to_string(),
                    bounds: bus.bounds,
                    points: &bus.points,
                    stroke: bus.stroke.as_ref(),
                },
                point,
            );
        }
        for entry in &self.bus_entries {
            push_canvas_polyline_hit(
                &mut hits,
                NspCanvasPolylineHit {
                    kind: "bus-entry",
                    uuid: entry.uuid.clone(),
                    label: "bus-entry".to_string(),
                    bounds: entry.bounds,
                    points: &[entry.at, entry.end()],
                    stroke: entry.stroke.as_ref(),
                },
                point,
            );
        }
        for label in &self.directive_labels {
            push_canvas_directive_label_hit(&mut hits, label, point);
        }
        for label in &self.labels {
            push_canvas_hit(
                &mut hits,
                "label",
                label.uuid.clone(),
                label.text.clone(),
                label.bounds,
                point,
            );
        }
        for text in &self.text_items {
            push_canvas_hit(
                &mut hits,
                "text",
                text.uuid.clone(),
                text.text.clone(),
                text.bounds,
                point,
            );
        }
        for text_box in &self.text_boxes {
            push_canvas_text_box_hit(&mut hits, text_box, point);
        }
        for junction in &self.junctions {
            push_canvas_junction_hit(&mut hits, junction, point);
        }
        for marker in &self.no_connects {
            push_canvas_no_connect_hit(&mut hits, marker, point);
        }
        for group in &self.groups {
            push_canvas_hit(
                &mut hits,
                "group",
                group.uuid.clone(),
                group.name.clone(),
                group.bounds,
                point,
            );
        }
        hits.sort_by(|left, right| {
            left.bounds
                .area()
                .total_cmp(&right.bounds.area())
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.uuid.cmp(&right.uuid))
                .then_with(|| left.label.cmp(&right.label))
        });

        NspCanvasHitReport {
            source: self.source.clone(),
            at: point,
            hit_count: hits.len(),
            hits,
        }
    }

    /// Returns canvas hit metadata for an already selected schema UUID.
    ///
    /// This is used by editor frontends to refresh selection state after a
    /// document edit while keeping all schema geometry knowledge inside
    /// `osl-schema`.
    pub fn item_hit_by_uuid(&self, uuid: &str) -> Option<NspCanvasHit> {
        let uuid = uuid.trim();
        if uuid.is_empty() {
            return None;
        }

        for symbol in &self.symbols {
            if let Some(hit) = uuid_hit(
                "symbol",
                symbol.uuid.as_deref(),
                uuid,
                &symbol.reference,
                symbol.bounds,
            ) {
                return Some(hit);
            }
        }
        for sheet in &self.sheets {
            if let Some(hit) = uuid_hit(
                "sheet",
                sheet.uuid.as_deref(),
                uuid,
                &sheet.name,
                sheet.bounds,
            ) {
                return Some(hit);
            }
            for pin in &sheet.pins {
                if let Some(hit) = uuid_hit(
                    "sheet-pin",
                    pin.uuid.as_deref(),
                    uuid,
                    &pin.name,
                    pin.bounds,
                ) {
                    return Some(hit);
                }
            }
        }
        for graphic in &self.graphics {
            if let Some(hit) = uuid_hit(
                "graphic",
                graphic.uuid().as_deref(),
                uuid,
                graphic.kind(),
                graphic.bounds(),
            ) {
                return Some(hit);
            }
        }
        for image in &self.images {
            if let Some(hit) = uuid_hit(
                "image",
                image.uuid.as_deref(),
                uuid,
                &image.mime_type,
                image.bounds,
            ) {
                return Some(hit);
            }
        }
        for table in &self.tables {
            if let Some(hit) = uuid_hit("table", table.uuid.as_deref(), uuid, "table", table.bounds)
            {
                return Some(hit);
            }
            for cell in &table.cells {
                if let Some(hit) = uuid_hit(
                    "table-cell",
                    cell.uuid.as_deref(),
                    uuid,
                    &cell.text,
                    cell.bounds,
                ) {
                    return Some(hit);
                }
            }
        }
        for rule_area in &self.rule_areas {
            if let Some(hit) = uuid_hit(
                "rule-area",
                rule_area.uuid.as_deref(),
                uuid,
                "rule-area",
                rule_area.bounds,
            ) {
                return Some(hit);
            }
        }
        for wire in &self.wires {
            if let Some(hit) = uuid_hit("wire", wire.uuid.as_deref(), uuid, "wire", wire.bounds) {
                return Some(hit);
            }
        }
        for bus in &self.buses {
            if let Some(hit) = uuid_hit("bus", bus.uuid.as_deref(), uuid, "bus", bus.bounds) {
                return Some(hit);
            }
        }
        for entry in &self.bus_entries {
            if let Some(hit) = uuid_hit(
                "bus-entry",
                entry.uuid.as_deref(),
                uuid,
                "bus-entry",
                entry.bounds,
            ) {
                return Some(hit);
            }
        }
        for label in &self.directive_labels {
            if let Some(hit) = uuid_hit(
                "directive-label",
                label.uuid.as_deref(),
                uuid,
                &label.text,
                label.bounds,
            ) {
                return Some(hit);
            }
        }
        for label in &self.labels {
            if let Some(hit) = uuid_hit(
                "label",
                label.uuid.as_deref(),
                uuid,
                &label.text,
                label.bounds,
            ) {
                return Some(hit);
            }
        }
        for text in &self.text_items {
            if let Some(hit) = uuid_hit("text", text.uuid.as_deref(), uuid, &text.text, text.bounds)
            {
                return Some(hit);
            }
        }
        for text_box in &self.text_boxes {
            if let Some(hit) = uuid_hit(
                "text-box",
                text_box.uuid.as_deref(),
                uuid,
                &text_box.text,
                text_box.bounds,
            ) {
                return Some(hit);
            }
        }
        for junction in &self.junctions {
            if let Some(hit) = uuid_hit(
                "junction",
                junction.uuid.as_deref(),
                uuid,
                "junction",
                Some(junction.bounds),
            ) {
                return Some(hit);
            }
        }
        for marker in &self.no_connects {
            if let Some(hit) = uuid_hit(
                "no-connect",
                marker.uuid.as_deref(),
                uuid,
                "no-connect",
                Some(marker.bounds),
            ) {
                return Some(hit);
            }
        }
        for group in &self.groups {
            if let Some(hit) = uuid_hit(
                "group",
                group.uuid.as_deref(),
                uuid,
                &group.name,
                group.bounds,
            ) {
                return Some(hit);
            }
        }
        None
    }
}

fn push_canvas_hit(
    hits: &mut Vec<NspCanvasHit>,
    kind: &str,
    uuid: Option<String>,
    label: String,
    bounds: Option<NspBoundingBox>,
    point: NspPoint,
) {
    let Some(bounds) = bounds else {
        return;
    };
    if bounds.contains(point) {
        hits.push(NspCanvasHit {
            kind: kind.to_string(),
            uuid,
            label,
            bounds,
        });
    }
}

fn uuid_hit(
    kind: &str,
    candidate_uuid: Option<&str>,
    uuid: &str,
    label: &str,
    bounds: Option<NspBoundingBox>,
) -> Option<NspCanvasHit> {
    if candidate_uuid != Some(uuid) {
        return None;
    }
    Some(NspCanvasHit {
        kind: kind.to_string(),
        uuid: Some(uuid.to_string()),
        label: label.to_string(),
        bounds: bounds?,
    })
}

fn push_canvas_graphic_hit(
    hits: &mut Vec<NspCanvasHit>,
    graphic: &NspCanvasGraphic,
    point: NspPoint,
) {
    let Some(bounds) = graphic.bounds() else {
        return;
    };
    if bounds.contains(point) && graphic.hits_point(point) {
        hits.push(NspCanvasHit {
            kind: "graphic".to_string(),
            uuid: graphic.uuid(),
            label: graphic.kind().to_string(),
            bounds,
        });
    }
}

fn push_canvas_symbol_hit(hits: &mut Vec<NspCanvasHit>, symbol: &NspCanvasSymbol, point: NspPoint) {
    let Some(bounds) = symbol.bounds else {
        return;
    };
    if bounds.contains(point)
        && (symbol
            .graphics
            .iter()
            .any(|graphic| graphic.hits_point(point))
            || symbol
                .pins
                .iter()
                .any(|pin| schema_canvas_pin_hits_point(pin, point)))
    {
        hits.push(NspCanvasHit {
            kind: "symbol".to_string(),
            uuid: symbol.uuid.clone(),
            label: symbol.reference.clone(),
            bounds,
        });
    }
}

fn push_canvas_rule_area_hit(
    hits: &mut Vec<NspCanvasHit>,
    rule_area: &NspCanvasRuleArea,
    point: NspPoint,
) {
    let Some(bounds) = rule_area.bounds else {
        return;
    };
    if bounds.contains(point) && rule_area.hits_point(point) {
        hits.push(NspCanvasHit {
            kind: "rule-area".to_string(),
            uuid: rule_area.uuid.clone(),
            label: "rule-area".to_string(),
            bounds,
        });
    }
}

fn push_canvas_sheet_hit(hits: &mut Vec<NspCanvasHit>, sheet: &NspCanvasSheet, point: NspPoint) {
    let Some(sheet_box) = schema_sheet_box_bounds(sheet.at, sheet.size) else {
        return;
    };
    if sheet_box.contains(point) {
        hits.push(NspCanvasHit {
            kind: "sheet".to_string(),
            uuid: sheet.uuid.clone(),
            label: sheet.name.clone(),
            bounds: sheet.bounds.unwrap_or(sheet_box),
        });
    }
}

fn push_canvas_sheet_pin_hit(
    hits: &mut Vec<NspCanvasHit>,
    pin: &NspCanvasSheetPin,
    point: NspPoint,
) {
    let Some(at) = pin.at else {
        return;
    };
    let points = [at.point(), pin_body_end(at, SCHEMA_SHEET_PIN_STUB_LENGTH)];
    push_canvas_polyline_hit(
        hits,
        NspCanvasPolylineHit {
            kind: "sheet-pin",
            uuid: pin.uuid.clone(),
            label: pin.name.clone(),
            bounds: pin.bounds,
            points: &points,
            stroke: None,
        },
        point,
    );
}

fn push_canvas_directive_label_hit(
    hits: &mut Vec<NspCanvasHit>,
    label: &NspCanvasDirectiveLabel,
    point: NspPoint,
) {
    let Some(bounds) = label.bounds else {
        return;
    };
    if bounds.contains(point) && schema_canvas_directive_label_hits_point(label, point) {
        hits.push(NspCanvasHit {
            kind: "directive-label".to_string(),
            uuid: label.uuid.clone(),
            label: label.text.clone(),
            bounds,
        });
    }
}

fn schema_canvas_directive_label_hits_point(
    label: &NspCanvasDirectiveLabel,
    point: NspPoint,
) -> bool {
    if let Some(at) = label.at {
        let points = [at.point(), pin_body_end(at, label.length.unwrap_or(2.54))];
        if schema_polyline_hits_point(&points, None, point) {
            return true;
        }
    }

    schema_text_bounds(&label.text, label.at, label.effects.as_ref())
        .is_some_and(|bounds| bounds.contains(point))
}

fn push_canvas_table_hit(hits: &mut Vec<NspCanvasHit>, table: &NspCanvasTable, point: NspPoint) {
    let Some(bounds) = table.bounds else {
        return;
    };
    if bounds.contains(point)
        && table
            .cells
            .iter()
            .any(|cell| schema_table_cell_hits_point(cell, point))
    {
        hits.push(NspCanvasHit {
            kind: "table".to_string(),
            uuid: table.uuid.clone(),
            label: "table".to_string(),
            bounds,
        });
    }
}

fn push_canvas_table_cell_hit(
    hits: &mut Vec<NspCanvasHit>,
    cell: &NspCanvasTableCell,
    point: NspPoint,
) {
    let Some(bounds) = cell.bounds else {
        return;
    };
    if bounds.contains(point) && schema_table_cell_hits_point(cell, point) {
        hits.push(NspCanvasHit {
            kind: "table-cell".to_string(),
            uuid: cell.uuid.clone(),
            label: cell.text.clone(),
            bounds,
        });
    }
}

fn schema_table_cell_hits_point(cell: &NspCanvasTableCell, point: NspPoint) -> bool {
    match (cell.at, cell.size) {
        (Some(at), Some(size)) => schema_rotated_rect_contains_point(at, size, point),
        _ => cell.bounds.is_some_and(|bounds| bounds.contains(point)),
    }
}

fn schema_canvas_pin_hits_point(pin: &NspCanvasPin, point: NspPoint) -> bool {
    schema_polyline_hits_point(&[pin.start, pin.end], None, point)
}

fn push_canvas_text_box_hit(
    hits: &mut Vec<NspCanvasHit>,
    text_box: &NspCanvasTextBox,
    point: NspPoint,
) {
    let Some(bounds) = text_box.bounds else {
        return;
    };
    let hit = match (text_box.at, text_box.size) {
        (Some(at), Some(size)) => schema_rotated_rect_contains_point(at, size, point),
        _ => bounds.contains(point),
    };
    if hit {
        hits.push(NspCanvasHit {
            kind: "text-box".to_string(),
            uuid: text_box.uuid.clone(),
            label: text_box.text.clone(),
            bounds,
        });
    }
}

fn push_canvas_junction_hit(
    hits: &mut Vec<NspCanvasHit>,
    junction: &NspCanvasJunction,
    point: NspPoint,
) {
    if junction.bounds.contains(point)
        && schema_point_distance(junction.at, point) <= schema_junction_radius(junction.diameter)
    {
        hits.push(NspCanvasHit {
            kind: "junction".to_string(),
            uuid: junction.uuid.clone(),
            label: "junction".to_string(),
            bounds: junction.bounds,
        });
    }
}

fn push_canvas_no_connect_hit(
    hits: &mut Vec<NspCanvasHit>,
    marker: &NspCanvasNoConnect,
    point: NspPoint,
) {
    let arms = schema_no_connect_arms(marker.at);
    if marker.bounds.contains(point)
        && arms
            .iter()
            .any(|arm| schema_polyline_hits_point(arm, None, point))
    {
        hits.push(NspCanvasHit {
            kind: "no-connect".to_string(),
            uuid: marker.uuid.clone(),
            label: "no-connect".to_string(),
            bounds: marker.bounds,
        });
    }
}

struct NspCanvasPolylineHit<'a> {
    kind: &'a str,
    uuid: Option<String>,
    label: String,
    bounds: Option<NspBoundingBox>,
    points: &'a [NspPoint],
    stroke: Option<&'a NspStroke>,
}

fn push_canvas_polyline_hit(
    hits: &mut Vec<NspCanvasHit>,
    item: NspCanvasPolylineHit<'_>,
    point: NspPoint,
) {
    let Some(bounds) = item.bounds else {
        return;
    };
    if bounds.contains(point) && schema_polyline_hits_point(item.points, item.stroke, point) {
        hits.push(NspCanvasHit {
            kind: item.kind.to_string(),
            uuid: item.uuid,
            label: item.label,
            bounds,
        });
    }
}
