//! Geometry primitives — bounding box, arc sampling, and spatial calculations.

use crate::{
    NspAt, NspFill, NspPoint, NspSize, NspStroke, NspTextEffects, transform::rotate_point,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NspBoundingBox {
    pub min: NspPoint,
    pub max: NspPoint,
}

impl NspBoundingBox {
    /// width。
    pub fn width(self) -> f64 {
        self.max.x - self.min.x
    }

    /// height。
    pub fn height(self) -> f64 {
        self.max.y - self.min.y
    }

    /// padded。
    pub(crate) fn padded(self, padding: f64) -> Self {
        Self {
            min: NspPoint {
                x: self.min.x - padding,
                y: self.min.y - padding,
            },
            max: NspPoint {
                x: self.max.x + padding,
                y: self.max.y + padding,
            },
        }
    }

    /// contains。
    pub fn contains(self, point: NspPoint) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// intersects。
    pub fn intersects(self, other: NspBoundingBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// area。
    pub(crate) fn area(self) -> f64 {
        self.width().abs() * self.height().abs()
    }

    /// union。
    pub(crate) fn union(self, other: NspBoundingBox) -> Self {
        Self {
            min: NspPoint {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
            },
            max: NspPoint {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
            },
        }
    }
}

/// `SCHEMA_CANVAS_POINT_BOUNDS_RADIUS` 常量。
pub(crate) const SCHEMA_CANVAS_POINT_BOUNDS_RADIUS: f64 = 1.27;
/// `SCHEMA_CANVAS_LINE_BOUNDS_PADDING` 常量。
pub(crate) const SCHEMA_CANVAS_LINE_BOUNDS_PADDING: f64 = 0.635;
/// `SCHEMA_SHEET_PIN_STUB_LENGTH` 常量。
pub(crate) const SCHEMA_SHEET_PIN_STUB_LENGTH: f64 = 2.54;
/// `SCHEMA_DEFAULT_JUNCTION_RADIUS` 常量。
pub(crate) const SCHEMA_DEFAULT_JUNCTION_RADIUS: f64 = SCHEMA_CANVAS_POINT_BOUNDS_RADIUS;
/// `SCHEMA_NO_CONNECT_ARM_LENGTH` 常量。
pub(crate) const SCHEMA_NO_CONNECT_ARM_LENGTH: f64 = SCHEMA_CANVAS_POINT_BOUNDS_RADIUS;

#[derive(Debug, Default)]
pub(crate) struct NspBoundingBoxBuilder {
    min: Option<NspPoint>,
    max: Option<NspPoint>,
}

impl NspBoundingBoxBuilder {
    /// include。
    pub(crate) fn include(&mut self, point: NspPoint) {
        self.min = Some(match self.min {
            Some(min) => NspPoint {
                x: min.x.min(point.x),
                y: min.y.min(point.y),
            },
            None => point,
        });
        self.max = Some(match self.max {
            Some(max) => NspPoint {
                x: max.x.max(point.x),
                y: max.y.max(point.y),
            },
            None => point,
        });
    }

    /// include box。
    pub(crate) fn include_box(&mut self, bounds: NspBoundingBox) {
        self.include(bounds.min);
        self.include(bounds.max);
    }

    /// finish。
    pub(crate) fn finish(self) -> Option<NspBoundingBox> {
        Some(NspBoundingBox {
            min: self.min?,
            max: self.max?,
        })
    }
}

/// schema point bounds。
pub(crate) fn schema_point_bounds(point: NspPoint, padding: f64) -> NspBoundingBox {
    NspBoundingBox {
        min: point,
        max: point,
    }
    .padded(padding)
}

/// schema points bounds。
pub(crate) fn schema_points_bounds(points: &[NspPoint], padding: f64) -> Option<NspBoundingBox> {
    let mut bounds = NspBoundingBoxBuilder::default();
    for point in points {
        bounds.include(*point);
    }
    bounds.finish().map(|bounds| bounds.padded(padding))
}

/// schema sheet pin bounds。
pub(crate) fn schema_sheet_pin_bounds(at: NspAt) -> Option<NspBoundingBox> {
    let points = [at.point(), pin_body_end(at, SCHEMA_SHEET_PIN_STUB_LENGTH)];
    schema_points_bounds(&points, SCHEMA_CANVAS_LINE_BOUNDS_PADDING)
}

/// schema junction radius。
pub(crate) fn schema_junction_radius(diameter: Option<f64>) -> f64 {
    diameter
        .filter(|diameter| diameter.is_finite() && *diameter > 0.0)
        .map(|diameter| diameter / 2.0)
        .unwrap_or(SCHEMA_DEFAULT_JUNCTION_RADIUS)
}

/// schema junction bounds。
pub(crate) fn schema_junction_bounds(at: NspPoint, diameter: Option<f64>) -> NspBoundingBox {
    schema_point_bounds(at, schema_junction_radius(diameter))
}

/// schema no connect arms。
pub(crate) fn schema_no_connect_arms(at: NspPoint) -> [[NspPoint; 2]; 2] {
    let arm = SCHEMA_NO_CONNECT_ARM_LENGTH;
    [
        [
            NspPoint {
                x: at.x - arm,
                y: at.y - arm,
            },
            NspPoint {
                x: at.x + arm,
                y: at.y + arm,
            },
        ],
        [
            NspPoint {
                x: at.x - arm,
                y: at.y + arm,
            },
            NspPoint {
                x: at.x + arm,
                y: at.y - arm,
            },
        ],
    ]
}

/// schema no connect bounds。
pub(crate) fn schema_no_connect_bounds(at: NspPoint) -> NspBoundingBox {
    let arms = schema_no_connect_arms(at);
    let points = [arms[0][0], arms[0][1], arms[1][0], arms[1][1]];
    schema_points_bounds(&points, SCHEMA_CANVAS_LINE_BOUNDS_PADDING)
        .expect("schema no-connect marker bounds use four points")
}

/// schema sheet box bounds。
pub(crate) fn schema_sheet_box_bounds(
    at: Option<NspAt>,
    size: Option<NspSize>,
) -> Option<NspBoundingBox> {
    let at = at?;
    let size = size?;
    let corners = [
        at.point(),
        NspPoint {
            x: at.x + size.width,
            y: at.y,
        },
        NspPoint {
            x: at.x + size.width,
            y: at.y + size.height,
        },
        NspPoint {
            x: at.x,
            y: at.y + size.height,
        },
    ];
    schema_points_bounds(&corners, 0.0)
}

/// schema rotated rect corners。
pub(crate) fn schema_rotated_rect_corners(at: NspAt, size: NspSize) -> [NspPoint; 4] {
    let width = size.width.abs();
    let height = size.height.abs();
    let local_corners = [
        NspPoint { x: 0.0, y: 0.0 },
        NspPoint { x: width, y: 0.0 },
        NspPoint {
            x: width,
            y: height,
        },
        NspPoint { x: 0.0, y: height },
    ];
    local_corners.map(|corner| {
        let rotated = rotate_point(corner, at.rotation);
        NspPoint {
            x: at.x + rotated.x,
            y: at.y + rotated.y,
        }
    })
}

/// schema rotated rect bounds。
pub(crate) fn schema_rotated_rect_bounds(at: NspAt, size: NspSize) -> Option<NspBoundingBox> {
    let mut bounds = NspBoundingBoxBuilder::default();
    for corner in schema_rotated_rect_corners(at, size) {
        bounds.include(corner);
    }
    bounds.finish()
}

/// schema rotated rect contains point。
pub(crate) fn schema_rotated_rect_contains_point(
    at: NspAt,
    size: NspSize,
    point: NspPoint,
) -> bool {
    let local = rotate_point(
        NspPoint {
            x: point.x - at.x,
            y: point.y - at.y,
        },
        -at.rotation,
    );
    local.x >= 0.0 && local.x <= size.width.abs() && local.y >= 0.0 && local.y <= size.height.abs()
}

/// schema text bounds。
pub(crate) fn schema_text_bounds(
    text: &str,
    at: Option<NspAt>,
    effects: Option<&NspTextEffects>,
) -> Option<NspBoundingBox> {
    let at = at?;
    let font_size = effects
        .and_then(|effects| effects.font_size)
        .unwrap_or(NspSize {
            width: 1.27,
            height: 1.27,
        });
    let char_width = if font_size.width.is_finite() && font_size.width > 0.0 {
        font_size.width * 0.7
    } else {
        1.27 * 0.7
    };
    let line_height = if font_size.height.is_finite() && font_size.height > 0.0 {
        font_size.height * 1.2
    } else {
        1.27 * 1.2
    };
    let lines = text.split('\n').collect::<Vec<_>>();
    let line_count = lines.len().max(1);
    let longest_line = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .max(1);
    let width = (longest_line as f64 * char_width).max(SCHEMA_CANVAS_POINT_BOUNDS_RADIUS * 2.0);
    let height = (line_count as f64 * line_height).max(SCHEMA_CANVAS_POINT_BOUNDS_RADIUS * 2.0);

    let justify = effects
        .map(|effects| effects.justify.as_slice())
        .unwrap_or(&[]);
    let left = if justify.iter().any(|token| token == "right") {
        -width
    } else if justify.iter().any(|token| token == "center") {
        -width / 2.0
    } else {
        0.0
    };
    let top = if justify.iter().any(|token| token == "bottom") {
        -height
    } else if justify.iter().any(|token| token == "center") {
        -height / 2.0
    } else {
        0.0
    };
    let corners = [
        NspPoint { x: left, y: top },
        NspPoint {
            x: left + width,
            y: top,
        },
        NspPoint {
            x: left + width,
            y: top + height,
        },
        NspPoint {
            x: left,
            y: top + height,
        },
    ];
    let mut bounds = NspBoundingBoxBuilder::default();
    for corner in corners {
        let rotated = rotate_point(corner, at.rotation);
        bounds.include(NspPoint {
            x: at.x + rotated.x,
            y: at.y + rotated.y,
        });
    }
    bounds
        .finish()
        .map(|bounds| bounds.padded(SCHEMA_CANVAS_LINE_BOUNDS_PADDING))
}

/// schema polyline hits point。
pub(crate) fn schema_polyline_hits_point(
    points: &[NspPoint],
    stroke: Option<&NspStroke>,
    point: NspPoint,
) -> bool {
    if points.is_empty() {
        return false;
    }
    let tolerance = schema_stroke_hit_tolerance(stroke);
    if points.len() == 1 {
        return schema_point_distance(points[0], point) <= tolerance;
    }
    points
        .windows(2)
        .any(|segment| schema_point_segment_distance(point, segment[0], segment[1]) <= tolerance)
}

/// schema closed polyline hits point。
pub(crate) fn schema_closed_polyline_hits_point(
    points: &[NspPoint],
    stroke: Option<&NspStroke>,
    point: NspPoint,
) -> bool {
    if schema_polyline_hits_point(points, stroke, point) {
        return true;
    }
    if points.len() < 3 {
        return false;
    }
    schema_point_segment_distance(point, *points.last().unwrap(), points[0])
        <= schema_stroke_hit_tolerance(stroke)
}

/// schema bezier hits point。
pub(crate) fn schema_bezier_hits_point(
    points: &[NspPoint],
    stroke: Option<&NspStroke>,
    point: NspPoint,
) -> bool {
    if points.len() != 4 {
        return schema_polyline_hits_point(points, stroke, point);
    }

    let sampled = schema_cubic_bezier_samples(points[0], points[1], points[2], points[3]);
    schema_polyline_hits_point(&sampled, stroke, point)
}

/// schema cubic bezier samples。
pub(crate) fn schema_cubic_bezier_samples(
    start: NspPoint,
    control_1: NspPoint,
    control_2: NspPoint,
    end: NspPoint,
) -> Vec<NspPoint> {
    const SCHEMA_BEZIER_HIT_SEGMENTS: usize = 32;
    let mut points = Vec::with_capacity(SCHEMA_BEZIER_HIT_SEGMENTS + 1);
    for index in 0..=SCHEMA_BEZIER_HIT_SEGMENTS {
        let t = index as f64 / SCHEMA_BEZIER_HIT_SEGMENTS as f64;
        let one_minus_t = 1.0 - t;
        let start_weight = one_minus_t * one_minus_t * one_minus_t;
        let control_1_weight = 3.0 * one_minus_t * one_minus_t * t;
        let control_2_weight = 3.0 * one_minus_t * t * t;
        let end_weight = t * t * t;
        points.push(NspPoint {
            x: start.x * start_weight
                + control_1.x * control_1_weight
                + control_2.x * control_2_weight
                + end.x * end_weight,
            y: start.y * start_weight
                + control_1.y * control_1_weight
                + control_2.y * control_2_weight
                + end.y * end_weight,
        });
    }
    points
}

/// sample schema arc points。
pub fn sample_arc_points(start: NspPoint, mid: Option<NspPoint>, end: NspPoint) -> Vec<NspPoint> {
    let Some(mid) = mid else {
        return vec![start, end];
    };
    let Some((center, radius)) = schema_circle_from_three_points(start, mid, end) else {
        return vec![start, mid, end];
    };

    let start_angle = schema_angle(center, start);
    let mid_angle = schema_angle(center, mid);
    let end_angle = schema_angle(center, end);
    let ccw_sweep = schema_positive_angle_delta(start_angle, end_angle);
    let mid_on_ccw = schema_positive_angle_delta(start_angle, mid_angle) <= ccw_sweep;
    let sweep = if mid_on_ccw {
        ccw_sweep
    } else {
        ccw_sweep - std::f64::consts::TAU
    };
    let segments = ((sweep.abs() / (std::f64::consts::PI / 32.0)).ceil() as usize).clamp(8, 96);
    let mut points = Vec::with_capacity(segments + 1);
    for index in 0..=segments {
        let angle = start_angle + sweep * index as f64 / segments as f64;
        points.push(NspPoint {
            x: center.x + radius * angle.cos(),
            y: center.y + radius * angle.sin(),
        });
    }
    points
}

/// schema arc hits point。
pub(crate) fn schema_arc_hits_point(
    start: NspPoint,
    mid: Option<NspPoint>,
    end: NspPoint,
    stroke: Option<&NspStroke>,
    point: NspPoint,
) -> bool {
    let sampled = sample_arc_points(start, mid, end);
    schema_polyline_hits_point(&sampled, stroke, point)
}

/// schema circle from three points。
pub(crate) fn schema_circle_from_three_points(
    first: NspPoint,
    second: NspPoint,
    third: NspPoint,
) -> Option<(NspPoint, f64)> {
    let determinant = 2.0
        * (first.x * (second.y - third.y)
            + second.x * (third.y - first.y)
            + third.x * (first.y - second.y));
    if determinant.abs() <= f64::EPSILON {
        return None;
    }

    let first_len = first.x * first.x + first.y * first.y;
    let second_len = second.x * second.x + second.y * second.y;
    let third_len = third.x * third.x + third.y * third.y;
    let center = NspPoint {
        x: (first_len * (second.y - third.y)
            + second_len * (third.y - first.y)
            + third_len * (first.y - second.y))
            / determinant,
        y: (first_len * (third.x - second.x)
            + second_len * (first.x - third.x)
            + third_len * (second.x - first.x))
            / determinant,
    };
    Some((center, schema_point_distance(center, first)))
}

/// schema angle。
pub(crate) fn schema_angle(center: NspPoint, point: NspPoint) -> f64 {
    (point.y - center.y).atan2(point.x - center.x)
}

/// schema positive angle delta。
pub(crate) fn schema_positive_angle_delta(start: f64, end: f64) -> f64 {
    (end - start).rem_euclid(std::f64::consts::TAU)
}

/// schema polygon contains point。
pub(crate) fn schema_polygon_contains_point(points: &[NspPoint], point: NspPoint) -> bool {
    if points.len() < 3 {
        return false;
    }
    if schema_closed_polyline_hits_point(points, None, point) {
        return true;
    }

    let mut inside = false;
    let mut previous = *points.last().unwrap();
    for current in points {
        let crosses_vertical_range = (current.y > point.y) != (previous.y > point.y);
        if crosses_vertical_range {
            let intersection_x = (previous.x - current.x) * (point.y - current.y)
                / (previous.y - current.y)
                + current.x;
            if point.x < intersection_x {
                inside = !inside;
            }
        }
        previous = *current;
    }
    inside
}

/// schema rectangle hits point。
pub(crate) fn schema_rectangle_hits_point(
    start: NspPoint,
    end: NspPoint,
    stroke: Option<&NspStroke>,
    fill: Option<&NspFill>,
    point: NspPoint,
) -> bool {
    let min = NspPoint {
        x: start.x.min(end.x),
        y: start.y.min(end.y),
    };
    let max = NspPoint {
        x: start.x.max(end.x),
        y: start.y.max(end.y),
    };
    let shape_bounds = NspBoundingBox { min, max };
    if schema_fill_is_solid(fill) && shape_bounds.contains(point) {
        return true;
    }

    let corners = [
        min,
        NspPoint { x: max.x, y: min.y },
        max,
        NspPoint { x: min.x, y: max.y },
        min,
    ];
    schema_polyline_hits_point(&corners, stroke, point)
}

/// schema circle hits point。
pub(crate) fn schema_circle_hits_point(
    center: NspPoint,
    radius: f64,
    stroke: Option<&NspStroke>,
    fill: Option<&NspFill>,
    point: NspPoint,
) -> bool {
    let distance = schema_point_distance(center, point);
    let radius = radius.abs();
    if schema_fill_is_solid(fill) && distance <= radius {
        return true;
    }
    (distance - radius).abs() <= schema_stroke_hit_tolerance(stroke)
}

/// schema fill is solid。
pub(crate) fn schema_fill_is_solid(fill: Option<&NspFill>) -> bool {
    fill.and_then(|fill| fill.fill_type.as_deref())
        .is_some_and(|fill_type| !fill_type.eq_ignore_ascii_case("none"))
}

/// schema stroke hit tolerance。
pub(crate) fn schema_stroke_hit_tolerance(stroke: Option<&NspStroke>) -> f64 {
    let stroke_radius = stroke.and_then(|stroke| stroke.width).unwrap_or(0.0).abs() / 2.0;
    SCHEMA_CANVAS_LINE_BOUNDS_PADDING.max(stroke_radius)
}

/// schema point distance。
pub(crate) fn schema_point_distance(left: NspPoint, right: NspPoint) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    (dx * dx + dy * dy).sqrt()
}

/// schema point segment distance。
pub(crate) fn schema_point_segment_distance(
    point: NspPoint,
    start: NspPoint,
    end: NspPoint,
) -> f64 {
    let segment_x = end.x - start.x;
    let segment_y = end.y - start.y;
    let segment_len_sq = segment_x * segment_x + segment_y * segment_y;
    if segment_len_sq <= f64::EPSILON {
        return schema_point_distance(point, start);
    }

    let projection =
        ((point.x - start.x) * segment_x + (point.y - start.y) * segment_y) / segment_len_sq;
    let projection = projection.clamp(0.0, 1.0);
    let closest = NspPoint {
        x: start.x + projection * segment_x,
        y: start.y + projection * segment_y,
    };
    schema_point_distance(point, closest)
}

/// schema at bounds。
pub(crate) fn schema_at_bounds(at: Option<NspAt>, padding: f64) -> Option<NspBoundingBox> {
    at.map(|at| schema_point_bounds(at.point(), padding))
}

/// pin body end。
pub(crate) fn pin_body_end(at: NspAt, length: f64) -> NspPoint {
    let radians = at.rotation.to_radians();
    NspPoint {
        x: at.x + length * radians.cos(),
        y: at.y + length * radians.sin(),
    }
}
