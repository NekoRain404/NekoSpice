use crate::{KicadAt, KicadPoint};
use osl_core::{OslError, OslResult};
use std::collections::BTreeSet;

pub(crate) fn transform_symbol_point(
    pin_at: KicadAt,
    symbol_at: KicadAt,
    mirror: Option<&str>,
) -> KicadPoint {
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

pub(crate) fn rotate_point(point: KicadPoint, rotation: f64) -> KicadPoint {
    let normalized = normalized_rotation(rotation).round() as i32;
    match normalized {
        0 => point,
        90 => KicadPoint {
            x: -point.y,
            y: point.x,
        },
        180 => KicadPoint {
            x: -point.x,
            y: -point.y,
        },
        270 => KicadPoint {
            x: point.y,
            y: -point.x,
        },
        _ => {
            let radians = rotation.to_radians();
            KicadPoint {
                x: point.x * radians.cos() - point.y * radians.sin(),
                y: point.x * radians.sin() + point.y * radians.cos(),
            }
        }
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

pub(crate) fn normalized_rotation(rotation: f64) -> f64 {
    let normalized = rotation % 360.0;
    if normalized < 0.0 {
        normalized + 360.0
    } else {
        normalized
    }
}
