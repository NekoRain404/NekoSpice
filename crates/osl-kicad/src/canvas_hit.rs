use crate::geometry::{
    KICAD_SHEET_PIN_STUB_LENGTH, kicad_junction_radius, kicad_no_connect_arms,
    kicad_point_distance, kicad_polyline_hits_point, kicad_rotated_rect_contains_point,
    kicad_sheet_box_bounds, kicad_text_bounds, pin_body_end,
};
use crate::{
    KicadBoundingBox, KicadCanvasDirectiveLabel, KicadCanvasGraphic, KicadCanvasJunction,
    KicadCanvasNoConnect, KicadCanvasPin, KicadCanvasRuleArea, KicadCanvasScene, KicadCanvasSheet,
    KicadCanvasSheetPin, KicadCanvasSymbol, KicadCanvasTable, KicadCanvasTableCell,
    KicadCanvasTextBox, KicadPoint, KicadStroke, kicad_bounding_box_value, kicad_point_value,
};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasHit {
    pub kind: String,
    pub uuid: Option<String>,
    pub label: String,
    pub bounds: KicadBoundingBox,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadCanvasHitReport {
    pub source: String,
    pub at: KicadPoint,
    pub hit_count: usize,
    pub hits: Vec<KicadCanvasHit>,
}

impl KicadCanvasHitReport {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&serde_json::json!({
            "source": self.source,
            "at": kicad_point_value(self.at),
            "hit_count": self.hit_count,
            "hits": self.hits.iter().map(KicadCanvasHit::to_json_value).collect::<Vec<_>>(),
        }))
        .expect("KiCad canvas hit-test JSON should serialize")
    }
}

impl KicadCanvasHit {
    fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": self.kind,
            "uuid": self.uuid,
            "label": self.label,
            "bounds": kicad_bounding_box_value(self.bounds),
            "area": self.bounds.area(),
        })
    }
}

impl KicadCanvasScene {
    pub fn hit_test(&self, point: KicadPoint) -> KicadCanvasHitReport {
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
                KicadCanvasPolylineHit {
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
                KicadCanvasPolylineHit {
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
                KicadCanvasPolylineHit {
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

        KicadCanvasHitReport {
            source: self.source.clone(),
            at: point,
            hit_count: hits.len(),
            hits,
        }
    }

    /// Returns canvas hit metadata for an already selected KiCad UUID.
    ///
    /// This is used by editor frontends to refresh selection state after a
    /// document edit while keeping all KiCad geometry knowledge inside
    /// `osl-kicad`.
    pub fn item_hit_by_uuid(&self, uuid: &str) -> Option<KicadCanvasHit> {
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
    hits: &mut Vec<KicadCanvasHit>,
    kind: &str,
    uuid: Option<String>,
    label: String,
    bounds: Option<KicadBoundingBox>,
    point: KicadPoint,
) {
    let Some(bounds) = bounds else {
        return;
    };
    if bounds.contains(point) {
        hits.push(KicadCanvasHit {
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
    bounds: Option<KicadBoundingBox>,
) -> Option<KicadCanvasHit> {
    if candidate_uuid != Some(uuid) {
        return None;
    }
    Some(KicadCanvasHit {
        kind: kind.to_string(),
        uuid: Some(uuid.to_string()),
        label: label.to_string(),
        bounds: bounds?,
    })
}

fn push_canvas_graphic_hit(
    hits: &mut Vec<KicadCanvasHit>,
    graphic: &KicadCanvasGraphic,
    point: KicadPoint,
) {
    let Some(bounds) = graphic.bounds() else {
        return;
    };
    if bounds.contains(point) && graphic.hits_point(point) {
        hits.push(KicadCanvasHit {
            kind: "graphic".to_string(),
            uuid: graphic.uuid(),
            label: graphic.kind().to_string(),
            bounds,
        });
    }
}

fn push_canvas_symbol_hit(
    hits: &mut Vec<KicadCanvasHit>,
    symbol: &KicadCanvasSymbol,
    point: KicadPoint,
) {
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
                .any(|pin| kicad_canvas_pin_hits_point(pin, point)))
    {
        hits.push(KicadCanvasHit {
            kind: "symbol".to_string(),
            uuid: symbol.uuid.clone(),
            label: symbol.reference.clone(),
            bounds,
        });
    }
}

fn push_canvas_rule_area_hit(
    hits: &mut Vec<KicadCanvasHit>,
    rule_area: &KicadCanvasRuleArea,
    point: KicadPoint,
) {
    let Some(bounds) = rule_area.bounds else {
        return;
    };
    if bounds.contains(point) && rule_area.hits_point(point) {
        hits.push(KicadCanvasHit {
            kind: "rule-area".to_string(),
            uuid: rule_area.uuid.clone(),
            label: "rule-area".to_string(),
            bounds,
        });
    }
}

fn push_canvas_sheet_hit(
    hits: &mut Vec<KicadCanvasHit>,
    sheet: &KicadCanvasSheet,
    point: KicadPoint,
) {
    let Some(sheet_box) = kicad_sheet_box_bounds(sheet.at, sheet.size) else {
        return;
    };
    if sheet_box.contains(point) {
        hits.push(KicadCanvasHit {
            kind: "sheet".to_string(),
            uuid: sheet.uuid.clone(),
            label: sheet.name.clone(),
            bounds: sheet.bounds.unwrap_or(sheet_box),
        });
    }
}

fn push_canvas_sheet_pin_hit(
    hits: &mut Vec<KicadCanvasHit>,
    pin: &KicadCanvasSheetPin,
    point: KicadPoint,
) {
    let Some(at) = pin.at else {
        return;
    };
    let points = [at.point(), pin_body_end(at, KICAD_SHEET_PIN_STUB_LENGTH)];
    push_canvas_polyline_hit(
        hits,
        KicadCanvasPolylineHit {
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
    hits: &mut Vec<KicadCanvasHit>,
    label: &KicadCanvasDirectiveLabel,
    point: KicadPoint,
) {
    let Some(bounds) = label.bounds else {
        return;
    };
    if bounds.contains(point) && kicad_canvas_directive_label_hits_point(label, point) {
        hits.push(KicadCanvasHit {
            kind: "directive-label".to_string(),
            uuid: label.uuid.clone(),
            label: label.text.clone(),
            bounds,
        });
    }
}

fn kicad_canvas_directive_label_hits_point(
    label: &KicadCanvasDirectiveLabel,
    point: KicadPoint,
) -> bool {
    if let Some(at) = label.at {
        let points = [at.point(), pin_body_end(at, label.length.unwrap_or(2.54))];
        if kicad_polyline_hits_point(&points, None, point) {
            return true;
        }
    }

    kicad_text_bounds(&label.text, label.at, label.effects.as_ref())
        .is_some_and(|bounds| bounds.contains(point))
}

fn push_canvas_table_hit(
    hits: &mut Vec<KicadCanvasHit>,
    table: &KicadCanvasTable,
    point: KicadPoint,
) {
    let Some(bounds) = table.bounds else {
        return;
    };
    if bounds.contains(point)
        && table
            .cells
            .iter()
            .any(|cell| kicad_table_cell_hits_point(cell, point))
    {
        hits.push(KicadCanvasHit {
            kind: "table".to_string(),
            uuid: table.uuid.clone(),
            label: "table".to_string(),
            bounds,
        });
    }
}

fn push_canvas_table_cell_hit(
    hits: &mut Vec<KicadCanvasHit>,
    cell: &KicadCanvasTableCell,
    point: KicadPoint,
) {
    let Some(bounds) = cell.bounds else {
        return;
    };
    if bounds.contains(point) && kicad_table_cell_hits_point(cell, point) {
        hits.push(KicadCanvasHit {
            kind: "table-cell".to_string(),
            uuid: cell.uuid.clone(),
            label: cell.text.clone(),
            bounds,
        });
    }
}

fn kicad_table_cell_hits_point(cell: &KicadCanvasTableCell, point: KicadPoint) -> bool {
    match (cell.at, cell.size) {
        (Some(at), Some(size)) => kicad_rotated_rect_contains_point(at, size, point),
        _ => cell.bounds.is_some_and(|bounds| bounds.contains(point)),
    }
}

fn kicad_canvas_pin_hits_point(pin: &KicadCanvasPin, point: KicadPoint) -> bool {
    kicad_polyline_hits_point(&[pin.start, pin.end], None, point)
}

fn push_canvas_text_box_hit(
    hits: &mut Vec<KicadCanvasHit>,
    text_box: &KicadCanvasTextBox,
    point: KicadPoint,
) {
    let Some(bounds) = text_box.bounds else {
        return;
    };
    let hit = match (text_box.at, text_box.size) {
        (Some(at), Some(size)) => kicad_rotated_rect_contains_point(at, size, point),
        _ => bounds.contains(point),
    };
    if hit {
        hits.push(KicadCanvasHit {
            kind: "text-box".to_string(),
            uuid: text_box.uuid.clone(),
            label: text_box.text.clone(),
            bounds,
        });
    }
}

fn push_canvas_junction_hit(
    hits: &mut Vec<KicadCanvasHit>,
    junction: &KicadCanvasJunction,
    point: KicadPoint,
) {
    if junction.bounds.contains(point)
        && kicad_point_distance(junction.at, point) <= kicad_junction_radius(junction.diameter)
    {
        hits.push(KicadCanvasHit {
            kind: "junction".to_string(),
            uuid: junction.uuid.clone(),
            label: "junction".to_string(),
            bounds: junction.bounds,
        });
    }
}

fn push_canvas_no_connect_hit(
    hits: &mut Vec<KicadCanvasHit>,
    marker: &KicadCanvasNoConnect,
    point: KicadPoint,
) {
    let arms = kicad_no_connect_arms(marker.at);
    if marker.bounds.contains(point)
        && arms
            .iter()
            .any(|arm| kicad_polyline_hits_point(arm, None, point))
    {
        hits.push(KicadCanvasHit {
            kind: "no-connect".to_string(),
            uuid: marker.uuid.clone(),
            label: "no-connect".to_string(),
            bounds: marker.bounds,
        });
    }
}

struct KicadCanvasPolylineHit<'a> {
    kind: &'a str,
    uuid: Option<String>,
    label: String,
    bounds: Option<KicadBoundingBox>,
    points: &'a [KicadPoint],
    stroke: Option<&'a KicadStroke>,
}

fn push_canvas_polyline_hit(
    hits: &mut Vec<KicadCanvasHit>,
    item: KicadCanvasPolylineHit<'_>,
    point: KicadPoint,
) {
    let Some(bounds) = item.bounds else {
        return;
    };
    if bounds.contains(point) && kicad_polyline_hits_point(item.points, item.stroke, point) {
        hits.push(KicadCanvasHit {
            kind: item.kind.to_string(),
            uuid: item.uuid,
            label: item.label,
            bounds,
        });
    }
}
