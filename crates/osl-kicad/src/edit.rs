use crate::{
    KicadAt, KicadGraphic, KicadLabelKind, KicadPoint, KicadProperty, KicadSheet, KicadSheetPin,
    KicadSize, KicadSymbolDef, KicadTable, coordinate_key, format_number,
};
use osl_core::{OslError, OslResult};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum KicadSchematicEdit {
    MoveSymbol {
        reference: String,
        to: KicadPoint,
        rotation: Option<f64>,
    },
    MoveItem {
        uuid: String,
        delta: KicadPoint,
    },
    DeleteItem {
        uuid: String,
    },
    ConfigureSymbol {
        reference: String,
        unit: Option<u32>,
        body_style: Option<Option<u32>>,
        mirror: Option<Option<String>>,
        pin_alternates: Option<BTreeMap<String, String>>,
    },
    SetSymbolProperty {
        reference: String,
        name: String,
        value: String,
        at: Option<KicadAt>,
    },
    PlaceSymbol {
        definition: Box<KicadSymbolDef>,
        library_symbols: Vec<KicadSymbolDef>,
        reference: String,
        value: String,
        at: KicadAt,
        unit: Option<u32>,
        body_style: Option<u32>,
        pin_alternates: BTreeMap<String, String>,
        uuid: Option<String>,
    },
    AddWire {
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    },
    AddBus {
        points: Vec<KicadPoint>,
        uuid: Option<String>,
    },
    AddBusEntry {
        at: KicadPoint,
        size: KicadSize,
        uuid: Option<String>,
    },
    AddJunction {
        at: KicadPoint,
        uuid: Option<String>,
    },
    AddNoConnect {
        at: KicadPoint,
        uuid: Option<String>,
    },
    AddLabel {
        text: String,
        kind: KicadLabelKind,
        at: KicadAt,
        uuid: Option<String>,
    },
    AddSheet {
        name: String,
        file: String,
        at: KicadAt,
        size: KicadSize,
        pins: Vec<KicadSheetPin>,
        uuid: Option<String>,
    },
    AddText {
        text: String,
        at: KicadAt,
        uuid: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadEditSummary {
    pub operation: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadSymbolPlacement {
    pub definition: KicadSymbolDef,
    pub library_symbols: Vec<KicadSymbolDef>,
    pub reference: String,
    pub value: String,
    pub at: KicadAt,
    pub unit: Option<u32>,
    pub body_style: Option<u32>,
    pub pin_alternates: BTreeMap<String, String>,
    pub uuid: Option<String>,
}

pub(crate) fn remove_by_uuid<T>(
    items: &mut Vec<T>,
    uuid: &str,
    item_uuid: impl Fn(&T) -> Option<&str>,
) -> bool {
    if let Some(index) = items.iter().position(|item| item_uuid(item) == Some(uuid)) {
        items.remove(index);
        true
    } else {
        false
    }
}

pub(crate) fn remove_table_cell_by_uuid(tables: &mut [KicadTable], uuid: &str) -> bool {
    for table in tables {
        if remove_by_uuid(&mut table.cells, uuid, |cell| cell.uuid.as_deref()) {
            return true;
        }
    }
    false
}

pub(crate) fn remove_sheet_pin_by_uuid(sheets: &mut [KicadSheet], uuid: &str) -> bool {
    for sheet in sheets {
        if remove_by_uuid(&mut sheet.pins, uuid, |pin| pin.uuid.as_deref()) {
            return true;
        }
    }
    false
}

pub(crate) fn move_table_cell_by_uuid(
    tables: &mut [KicadTable],
    uuid: &str,
    delta: KicadPoint,
) -> bool {
    for table in tables {
        if let Some(cell) = table
            .cells
            .iter_mut()
            .find(|cell| cell.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut cell.at, delta);
            return true;
        }
    }
    false
}

pub(crate) fn move_sheet_pin_by_uuid(
    sheets: &mut [KicadSheet],
    uuid: &str,
    delta: KicadPoint,
) -> bool {
    for sheet in sheets {
        if let Some(pin) = sheet
            .pins
            .iter_mut()
            .find(|pin| pin.uuid.as_deref() == Some(uuid))
        {
            translate_optional_at(&mut pin.at, delta);
            return true;
        }
    }
    false
}

pub(crate) fn translate_point(point: &mut KicadPoint, delta: KicadPoint) {
    point.x += delta.x;
    point.y += delta.y;
}

pub(crate) fn translate_optional_point(point: &mut Option<KicadPoint>, delta: KicadPoint) {
    if let Some(point) = point {
        translate_point(point, delta);
    }
}

pub(crate) fn translate_points(points: &mut [KicadPoint], delta: KicadPoint) {
    for point in points {
        translate_point(point, delta);
    }
}

pub(crate) fn translate_at(at: &mut KicadAt, delta: KicadPoint) {
    at.x += delta.x;
    at.y += delta.y;
}

pub(crate) fn translate_optional_at(at: &mut Option<KicadAt>, delta: KicadPoint) {
    if let Some(at) = at {
        translate_at(at, delta);
    }
}

pub(crate) fn translate_properties(properties: &mut [KicadProperty], delta: KicadPoint) {
    for property in properties {
        translate_optional_at(&mut property.at, delta);
    }
}

pub(crate) fn translate_graphic(graphic: &mut KicadGraphic, delta: KicadPoint) {
    match graphic {
        KicadGraphic::Polyline { points } | KicadGraphic::Bezier { points } => {
            translate_points(points, delta);
        }
        KicadGraphic::Rectangle { start, end } => {
            translate_point(start, delta);
            translate_point(end, delta);
        }
        KicadGraphic::Circle { center, .. } => translate_point(center, delta),
        KicadGraphic::Arc { start, mid, end } => {
            translate_point(start, delta);
            translate_optional_point(mid, delta);
            translate_point(end, delta);
        }
        KicadGraphic::Text { at, .. } => translate_optional_at(at, delta),
    }
}

pub(crate) fn move_summary(kind: &str, uuid: &str) -> KicadEditSummary {
    KicadEditSummary {
        operation: format!("move-{kind}"),
        target: uuid.to_string(),
    }
}

pub(crate) fn delete_summary(kind: &str, uuid: &str) -> KicadEditSummary {
    KicadEditSummary {
        operation: format!("delete-{kind}"),
        target: uuid.to_string(),
    }
}

pub(crate) fn validate_point(point: KicadPoint, context: &str) -> OslResult<()> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(())
    } else {
        Err(OslError::InvalidInput(format!(
            "{context} coordinates must be finite"
        )))
    }
}

pub(crate) fn validate_at(at: KicadAt, context: &str) -> OslResult<()> {
    validate_point(KicadPoint { x: at.x, y: at.y }, context)?;
    if at.rotation.is_finite() {
        Ok(())
    } else {
        Err(OslError::InvalidInput(format!(
            "{context} rotation must be finite"
        )))
    }
}

pub(crate) fn validate_size(size: KicadSize, context: &str) -> OslResult<()> {
    if size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0 {
        Ok(())
    } else {
        Err(OslError::InvalidInput(format!(
            "{context} size must contain finite positive width and height"
        )))
    }
}

pub(crate) fn validate_bus_entry_size(size: KicadSize, context: &str) -> OslResult<()> {
    if is_valid_bus_entry_size(size) {
        Ok(())
    } else {
        Err(OslError::InvalidInput(format!(
            "{context} size must contain finite non-zero x and y deltas"
        )))
    }
}

pub(crate) fn is_valid_bus_entry_size(size: KicadSize) -> bool {
    size.width.is_finite()
        && size.height.is_finite()
        && coordinate_key(size.width) != 0
        && coordinate_key(size.height) != 0
}

pub(crate) fn points_payload(points: &[KicadPoint]) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", format_number(point.x), format_number(point.y)))
        .collect::<Vec<_>>()
        .join(";")
}

pub(crate) fn fnv1a64(input: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(crate) fn uuid_from_hashes(left: u64, right: u64) -> String {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&left.to_be_bytes());
    bytes[8..].copy_from_slice(&right.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}
