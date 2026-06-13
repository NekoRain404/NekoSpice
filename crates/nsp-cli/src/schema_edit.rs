//! CLI schema edit commands — batch schematic editing operations.

use crate::{parse_number, parse_optional_positive_u32, parse_positive_u32, trailing_positionals};
use nsp_core::{OslError, OslResult};
use nsp_schema::{
    NspAt, NspLabelKind, NspPoint, NspSchematicEdit, NspSheetPin, NspSimulationDirectiveKind,
    NspSize, NspSymbolDef, normalize_symbol_mirror,
};
use std::collections::BTreeMap;

/// parse schema edit ops。
pub(crate) fn parse_edit_ops(
    args: &[String],
    symbol_definitions: &[NspSymbolDef],
) -> OslResult<Vec<NspSchematicEdit>> {
    trailing_positionals(args, 1)
        .into_iter()
        .map(|op| parse_edit_op(op, symbol_definitions))
        .collect()
}

fn parse_edit_op(op: &str, symbol_definitions: &[NspSymbolDef]) -> OslResult<NspSchematicEdit> {
    let (name, payload) = op.split_once(':').ok_or_else(|| {
        OslError::InvalidInput(format!(
            "invalid schema-edit op '{op}', expected <op>:<payload>"
        ))
    })?;
    match name {
        "move-symbol" => parse_move_symbol_edit(payload),
        "move-item" => parse_move_item_edit(payload),
        "delete-item" => parse_delete_item_edit(payload),
        "configure-symbol" => parse_configure_symbol_edit(payload),
        "set-property" => parse_set_property_edit(payload),
        "place-symbol" => parse_place_symbol_edit(payload, symbol_definitions),
        "add-wire" => parse_add_wire_edit(payload),
        "add-bus" => parse_add_bus_edit(payload),
        "add-bus-entry" => parse_add_bus_entry_edit(payload),
        "add-junction" => parse_add_junction_edit(payload),
        "add-no-connect" => parse_add_no_connect_edit(payload),
        "add-label" => parse_add_label_edit(payload),
        "add-global-label" => parse_add_label_edit_with_kind(payload, NspLabelKind::Global),
        "add-hierarchical-label" => {
            parse_add_label_edit_with_kind(payload, NspLabelKind::Hierarchical)
        }
        "add-sheet" => parse_add_sheet_edit(payload),
        "add-text" => parse_add_text_edit(payload),
        "set-simulation-directive" => parse_set_simulation_directive_edit(payload),
        _ => Err(OslError::InvalidInput(format!(
            "unsupported schema-edit op '{name}'"
        ))),
    }
}

fn parse_delete_item_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let uuid = payload.trim();
    if uuid.is_empty() {
        return Err(OslError::InvalidInput(
            "delete-item expects delete-item:<uuid>".to_string(),
        ));
    }

    Ok(NspSchematicEdit::DeleteItem {
        uuid: uuid.to_string(),
    })
}

fn parse_move_item_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (uuid, delta) = payload.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput("move-item expects move-item:<uuid>:<dx,dy>".to_string())
    })?;
    let uuid = uuid.trim();
    if uuid.is_empty() {
        return Err(OslError::InvalidInput(
            "move-item expects move-item:<uuid>:<dx,dy>".to_string(),
        ));
    }

    Ok(NspSchematicEdit::MoveItem {
        uuid: uuid.to_string(),
        delta: parse_point(delta, "item move delta")?,
    })
}

fn parse_configure_symbol_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let options = split_schema_configure_symbol_options(payload)?;
    let reference = options.payload.trim();
    if reference.is_empty() {
        return Err(OslError::InvalidInput(
            "configure-symbol expects configure-symbol:<reference>[:unit=<n>][:body-style=<n|none>][:mirror=<x|y|xy|none>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        ));
    }
    if options.unit.is_none()
        && options.body_style.is_none()
        && options.mirror.is_none()
        && options.pin_alternates.is_none()
    {
        return Err(OslError::InvalidInput(
            "configure-symbol requires at least one unit, body-style, mirror, or alt option"
                .to_string(),
        ));
    }

    Ok(NspSchematicEdit::ConfigureSymbol {
        reference: reference.to_string(),
        unit: options.unit,
        body_style: options.body_style,
        mirror: options.mirror,
        pin_alternates: options.pin_alternates,
    })
}

fn parse_move_symbol_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(OslError::InvalidInput(
            "move-symbol expects move-symbol:<reference>:<x,y>[:rotation]".to_string(),
        ));
    }
    let reference = parts[0].to_string();
    let to = parse_point(parts[1], "move-symbol target")?;
    let rotation = parts
        .get(2)
        .map(|value| parse_number(value, "move-symbol rotation"))
        .transpose()?;

    Ok(NspSchematicEdit::MoveSymbol {
        reference,
        to,
        rotation,
    })
}

fn parse_set_property_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (reference, rest) = payload.split_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "set-property expects set-property:<reference>:<name>=<value>[:x,y[,rotation]]"
                .to_string(),
        )
    })?;
    let (assignment, at) = match rest.split_once(':') {
        Some((assignment, at)) => (assignment, Some(parse_at(at, "property position")?)),
        None => (rest, None),
    };
    let (name, value) = assignment.split_once('=').ok_or_else(|| {
        OslError::InvalidInput(
            "set-property expects set-property:<reference>:<name>=<value>[:x,y[,rotation]]"
                .to_string(),
        )
    })?;

    Ok(NspSchematicEdit::SetSymbolProperty {
        reference: reference.to_string(),
        name: name.to_string(),
        value: value.to_string(),
        at,
    })
}

fn parse_place_symbol_edit(
    payload: &str,
    symbol_definitions: &[NspSymbolDef],
) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let options = split_schema_place_symbol_options(payload)?;
    let (rest, at) = options.payload.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "place-symbol expects place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        )
    })?;
    let (rest, value) = rest.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "place-symbol expects place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        )
    })?;
    let (lib_id, reference) = rest.rsplit_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "place-symbol expects place-symbol:<lib_id>:<reference>:<value>:<x,y[,rotation]>[:unit=<n>][:body-style=<n>][:alt=<pin>=<alternate>[,<pin>=<alternate>...]]"
                .to_string(),
        )
    })?;
    let definition = symbol_definitions
        .iter()
        .find(|definition| definition.name == lib_id || definition.local_name() == lib_id)
        .cloned()
        .ok_or_else(|| {
            OslError::InvalidInput(format!(
                "schema symbol definition '{lib_id}' was not found; pass --library <file.kicad_sym>"
            ))
        })?;

    Ok(NspSchematicEdit::PlaceSymbol {
        definition: Box::new(definition),
        library_symbols: symbol_definitions.to_vec(),
        reference: reference.to_string(),
        value: value.to_string(),
        at: parse_at(at, "symbol placement")?,
        unit: Some(options.unit.unwrap_or(1)),
        body_style: options.body_style,
        pin_alternates: options.pin_alternates,
        uuid,
    })
}

fn parse_add_wire_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (points, uuid) = split_payload_uuid(payload);
    let points = points
        .split(';')
        .map(|point| parse_point(point, "wire point"))
        .collect::<OslResult<Vec<_>>>()?;
    Ok(NspSchematicEdit::AddWire { points, uuid })
}

fn parse_add_bus_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (points, uuid) = split_payload_uuid(payload);
    let points = points
        .split(';')
        .map(|point| parse_point(point, "bus point"))
        .collect::<OslResult<Vec<_>>>()?;
    Ok(NspSchematicEdit::AddBus { points, uuid })
}

fn parse_add_bus_entry_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let (at, size) = payload.split_once(':').ok_or_else(|| {
        OslError::InvalidInput("add-bus-entry expects add-bus-entry:<x,y>:<dx,dy>".to_string())
    })?;
    Ok(NspSchematicEdit::AddBusEntry {
        at: parse_point(at, "bus entry position")?,
        size: parse_size(size, "bus entry size")?,
        uuid,
    })
}

fn parse_add_junction_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    Ok(NspSchematicEdit::AddJunction {
        at: parse_point(payload, "junction position")?,
        uuid,
    })
}

fn parse_add_no_connect_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    Ok(NspSchematicEdit::AddNoConnect {
        at: parse_point(payload, "no-connect position")?,
        uuid,
    })
}

fn parse_add_label_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    parse_add_label_edit_with_kind(payload, NspLabelKind::Local)
}

fn parse_add_label_edit_with_kind(
    payload: &str,
    default_kind: NspLabelKind,
) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(OslError::InvalidInput(
            "add-label expects add-label:<text>:<x,y[,rotation]>[:local|global|hierarchical]"
                .to_string(),
        ));
    }
    let kind = parts
        .get(2)
        .map(|kind| parse_label_kind(kind))
        .transpose()?
        .unwrap_or(default_kind);
    Ok(NspSchematicEdit::AddLabel {
        text: parts[0].to_string(),
        kind,
        at: parse_at(parts[1], "label position")?,
        uuid,
    })
}

fn parse_add_text_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let (text, at) = payload.split_once(':').ok_or_else(|| {
        OslError::InvalidInput("add-text expects add-text:<text>:<x,y[,rotation]>".to_string())
    })?;
    Ok(NspSchematicEdit::AddText {
        text: text.to_string(),
        at: parse_at(at, "text position")?,
        uuid,
    })
}

fn parse_set_simulation_directive_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let (kind, rest) = payload.split_once(':').ok_or_else(|| {
        OslError::InvalidInput(
            "set-simulation-directive expects set-simulation-directive:<kind>:<body>[:x,y[,rotation]]"
                .to_string(),
        )
    })?;
    let kind = kind.parse::<NspSimulationDirectiveKind>()?;
    let (body, at) = match rest.rsplit_once(':') {
        Some((body, at)) => match parse_at(at, "simulation directive position") {
            Ok(at) => (body, Some(at)),
            Err(_) => (rest, None),
        },
        None => (rest, None),
    };
    Ok(NspSchematicEdit::SetSimulationDirective {
        kind,
        body: body.to_string(),
        at,
        uuid,
    })
}

fn parse_add_sheet_edit(payload: &str) -> OslResult<NspSchematicEdit> {
    let (payload, uuid) = split_payload_uuid(payload);
    let parts = payload.split(':').collect::<Vec<_>>();
    if parts.len() < 4 || parts.len() > 5 {
        return Err(OslError::InvalidInput(
            "add-sheet expects add-sheet:<name>:<file>:<x,y>:<w,h>[:<pin@x,y[,rotation],type;...>]"
                .to_string(),
        ));
    }
    let pins = parts
        .get(4)
        .filter(|pins| !pins.trim().is_empty())
        .map(|pins| {
            pins.split(';')
                .map(parse_sheet_pin)
                .collect::<OslResult<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    Ok(NspSchematicEdit::AddSheet {
        name: parts[0].to_string(),
        file: parts[1].to_string(),
        at: sheet_at_from_point(parse_point(parts[2], "sheet position")?),
        size: parse_size(parts[3], "sheet size")?,
        pins,
        uuid,
    })
}

fn parse_sheet_pin(value: &str) -> OslResult<NspSheetPin> {
    let (name, rest) = value.split_once('@').ok_or_else(|| {
        OslError::InvalidInput("sheet pin expects <name>@<x,y[,rotation],type>".to_string())
    })?;
    let parts = rest.split(',').collect::<Vec<_>>();
    if parts.len() < 3 || parts.len() > 4 {
        return Err(OslError::InvalidInput(
            "sheet pin expects <name>@<x,y[,rotation],type>".to_string(),
        ));
    }
    let pin_type = parts.last().copied().unwrap_or_default().to_string();
    let at = if parts.len() == 3 {
        NspAt {
            x: parse_number(parts[0], "sheet pin position")?,
            y: parse_number(parts[1], "sheet pin position")?,
            rotation: 0.0,
        }
    } else {
        NspAt {
            x: parse_number(parts[0], "sheet pin position")?,
            y: parse_number(parts[1], "sheet pin position")?,
            rotation: parse_number(parts[2], "sheet pin rotation")?,
        }
    };
    Ok(NspSheetPin {
        name: name.to_string(),
        pin_type,
        at: Some(at),
        uuid: None,
        effects: None,
    })
}

fn split_payload_uuid(payload: &str) -> (&str, Option<String>) {
    match payload.rsplit_once(":uuid=") {
        Some((payload, uuid)) => (payload, Some(uuid.to_string())),
        None => (payload, None),
    }
}

struct NspPlaceSymbolOptions<'a> {
    payload: &'a str,
    unit: Option<u32>,
    body_style: Option<u32>,
    pin_alternates: BTreeMap<String, String>,
}

struct NspConfigureSymbolOptions<'a> {
    payload: &'a str,
    unit: Option<u32>,
    body_style: Option<Option<u32>>,
    mirror: Option<Option<String>>,
    pin_alternates: Option<BTreeMap<String, String>>,
}

fn split_schema_place_symbol_options(mut payload: &str) -> OslResult<NspPlaceSymbolOptions<'_>> {
    let mut unit = None;
    let mut body_style = None;
    let mut pin_alternates = BTreeMap::new();

    while let Some((rest, suffix)) = payload.rsplit_once(':') {
        if let Some(value) = suffix.strip_prefix("unit=") {
            if unit.is_some() {
                return Err(OslError::InvalidInput(
                    "place-symbol unit option was provided more than once".to_string(),
                ));
            }
            unit = Some(parse_positive_u32(value, "symbol unit")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("body-style=") {
            if body_style.is_some() {
                return Err(OslError::InvalidInput(
                    "place-symbol body-style option was provided more than once".to_string(),
                ));
            }
            body_style = Some(parse_positive_u32(value, "symbol body style")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("alt=") {
            if !pin_alternates.is_empty() {
                return Err(OslError::InvalidInput(
                    "place-symbol alt option was provided more than once".to_string(),
                ));
            }
            pin_alternates = parse_pin_alternates(value)?;
            payload = rest;
        } else {
            break;
        }
    }

    Ok(NspPlaceSymbolOptions {
        payload,
        unit,
        body_style,
        pin_alternates,
    })
}

fn split_schema_configure_symbol_options(
    mut payload: &str,
) -> OslResult<NspConfigureSymbolOptions<'_>> {
    let mut unit = None;
    let mut body_style = None;
    let mut mirror = None;
    let mut pin_alternates = None;

    while let Some((rest, suffix)) = payload.rsplit_once(':') {
        if let Some(value) = suffix.strip_prefix("unit=") {
            if unit.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol unit option was provided more than once".to_string(),
                ));
            }
            unit = Some(parse_positive_u32(value, "symbol unit")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("body-style=") {
            if body_style.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol body-style option was provided more than once".to_string(),
                ));
            }
            body_style = Some(parse_optional_positive_u32(value, "symbol body style")?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("mirror=") {
            if mirror.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol mirror option was provided more than once".to_string(),
                ));
            }
            mirror = Some(normalize_symbol_mirror(value)?);
            payload = rest;
        } else if let Some(value) = suffix.strip_prefix("alt=") {
            if pin_alternates.is_some() {
                return Err(OslError::InvalidInput(
                    "configure-symbol alt option was provided more than once".to_string(),
                ));
            }
            pin_alternates = Some(parse_pin_alternates(value)?);
            payload = rest;
        } else {
            break;
        }
    }

    Ok(NspConfigureSymbolOptions {
        payload,
        unit,
        body_style,
        mirror,
        pin_alternates,
    })
}

fn parse_pin_alternates(value: &str) -> OslResult<BTreeMap<String, String>> {
    if value.trim().is_empty() {
        return Err(OslError::InvalidInput(
            "place-symbol alt expects <pin>=<alternate>[,<pin>=<alternate>...]".to_string(),
        ));
    }

    let mut alternates = BTreeMap::new();
    for entry in value.split(',') {
        let (pin, alternate) = entry.split_once('=').ok_or_else(|| {
            OslError::InvalidInput(
                "place-symbol alt expects <pin>=<alternate>[,<pin>=<alternate>...]".to_string(),
            )
        })?;
        if pin.trim().is_empty() || alternate.trim().is_empty() {
            return Err(OslError::InvalidInput(
                "place-symbol alt pin and alternate names must not be empty".to_string(),
            ));
        }
        if alternates
            .insert(pin.to_string(), alternate.to_string())
            .is_some()
        {
            return Err(OslError::InvalidInput(format!(
                "place-symbol alt pin '{pin}' was provided more than once"
            )));
        }
    }

    Ok(alternates)
}

/// parse schema point。
pub(crate) fn parse_point(value: &str, context: &str) -> OslResult<NspPoint> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(OslError::InvalidInput(format!(
            "{context} expects x,y coordinates"
        )));
    }
    Ok(NspPoint {
        x: parse_number(parts[0], context)?,
        y: parse_number(parts[1], context)?,
    })
}

fn sheet_at_from_point(point: NspPoint) -> NspAt {
    NspAt {
        x: point.x,
        y: point.y,
        rotation: 0.0,
    }
}

fn parse_at(value: &str, context: &str) -> OslResult<NspAt> {
    let parts = value.split(',').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return Err(OslError::InvalidInput(format!(
            "{context} expects x,y or x,y,rotation"
        )));
    }
    Ok(NspAt {
        x: parse_number(parts[0], context)?,
        y: parse_number(parts[1], context)?,
        rotation: parts
            .get(2)
            .map(|value| parse_number(value, context))
            .transpose()?
            .unwrap_or(0.0),
    })
}

fn parse_size(value: &str, context: &str) -> OslResult<NspSize> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(OslError::InvalidInput(format!(
            "{context} expects width,height"
        )));
    }
    Ok(NspSize {
        width: parse_number(parts[0], context)?,
        height: parse_number(parts[1], context)?,
    })
}

fn parse_label_kind(value: &str) -> OslResult<NspLabelKind> {
    match value {
        "local" => Ok(NspLabelKind::Local),
        "global" => Ok(NspLabelKind::Global),
        "hierarchical" => Ok(NspLabelKind::Hierarchical),
        _ => Err(OslError::InvalidInput(format!(
            "unsupported schema label kind '{value}'"
        ))),
    }
}
