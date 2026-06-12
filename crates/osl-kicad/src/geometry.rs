use crate::{
    KicadAt, KicadFill, KicadPoint, KicadSize, KicadStroke, KicadTextEffects,
    transform::rotate_point,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KicadBoundingBox {
    pub min: KicadPoint,
    pub max: KicadPoint,
}

impl KicadBoundingBox {
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
            min: KicadPoint {
                x: self.min.x - padding,
                y: self.min.y - padding,
            },
            max: KicadPoint {
                x: self.max.x + padding,
                y: self.max.y + padding,
            },
        }
    }

    /// contains。
    pub fn contains(self, point: KicadPoint) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// intersects。
    pub fn intersects(self, other: KicadBoundingBox) -> bool {
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
    pub(crate) fn union(self, other: KicadBoundingBox) -> Self {
        Self {
            min: KicadPoint {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
            },
            max: KicadPoint {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
            },
        }
    }
}

/// `KICAD_CANVAS_POINT_BOUNDS_RADIUS` 常量。
pub(crate) const KICAD_CANVAS_POINT_BOUNDS_RADIUS: f64 = 1.27;
/// `KICAD_CANVAS_LINE_BOUNDS_PADDING` 常量。
pub(crate) const KICAD_CANVAS_LINE_BOUNDS_PADDING: f64 = 0.635;
/// `KICAD_SHEET_PIN_STUB_LENGTH` 常量。
pub(crate) const KICAD_SHEET_PIN_STUB_LENGTH: f64 = 2.54;
/// `KICAD_DEFAULT_JUNCTION_RADIUS` 常量。
pub(crate) const KICAD_DEFAULT_JUNCTION_RADIUS: f64 = KICAD_CANVAS_POINT_BOUNDS_RADIUS;
/// `KICAD_NO_CONNECT_ARM_LENGTH` 常量。
pub(crate) const KICAD_NO_CONNECT_ARM_LENGTH: f64 = KICAD_CANVAS_POINT_BOUNDS_RADIUS;

#[derive(Debug, Default)]
pub(crate) struct KicadBoundingBoxBuilder {
    min: Option<KicadPoint>,
    max: Option<KicadPoint>,
}

impl KicadBoundingBoxBuilder {
    /// include。
    pub(crate) fn include(&mut self, point: KicadPoint) {
        self.min = Some(match self.min {
            Some(min) => KicadPoint {
                x: min.x.min(point.x),
                y: min.y.min(point.y),
            },
            None => point,
        });
        self.max = Some(match self.max {
            Some(max) => KicadPoint {
                x: max.x.max(point.x),
                y: max.y.max(point.y),
            },
            None => point,
        });
    }

    /// include box。
    pub(crate) fn include_box(&mut self, bounds: KicadBoundingBox) {
        self.include(bounds.min);
        self.include(bounds.max);
    }

    /// finish。
    pub(crate) fn finish(self) -> Option<KicadBoundingBox> {
        Some(KicadBoundingBox {
            min: self.min?,
            max: self.max?,
        })
    }
}

/// kicad point bounds。
pub(crate) fn kicad_point_bounds(point: KicadPoint, padding: f64) -> KicadBoundingBox {
    KicadBoundingBox {
        min: point,
        max: point,
    }
    .padded(padding)
}

/// kicad points bounds。
pub(crate) fn kicad_points_bounds(points: &[KicadPoint], padding: f64) -> Option<KicadBoundingBox> {
    let mut bounds = KicadBoundingBoxBuilder::default();
    for point in points {
        bounds.include(*point);
    }
    bounds.finish().map(|bounds| bounds.padded(padding))
}

/// kicad sheet pin bounds。
pub(crate) fn kicad_sheet_pin_bounds(at: KicadAt) -> Option<KicadBoundingBox> {
    let points = [at.point(), pin_body_end(at, KICAD_SHEET_PIN_STUB_LENGTH)];
    kicad_points_bounds(&points, KICAD_CANVAS_LINE_BOUNDS_PADDING)
}

/// kicad junction radius。
pub(crate) fn kicad_junction_radius(diameter: Option<f64>) -> f64 {
    diameter
        .filter(|diameter| diameter.is_finite() && *diameter > 0.0)
        .map(|diameter| diameter / 2.0)
        .unwrap_or(KICAD_DEFAULT_JUNCTION_RADIUS)
}

/// kicad junction bounds。
pub(crate) fn kicad_junction_bounds(at: KicadPoint, diameter: Option<f64>) -> KicadBoundingBox {
    kicad_point_bounds(at, kicad_junction_radius(diameter))
}

/// kicad no connect arms。
pub(crate) fn kicad_no_connect_arms(at: KicadPoint) -> [[KicadPoint; 2]; 2] {
    let arm = KICAD_NO_CONNECT_ARM_LENGTH;
    [
        [
            KicadPoint {
                x: at.x - arm,
                y: at.y - arm,
            },
            KicadPoint {
                x: at.x + arm,
                y: at.y + arm,
            },
        ],
        [
            KicadPoint {
                x: at.x - arm,
                y: at.y + arm,
            },
            KicadPoint {
                x: at.x + arm,
                y: at.y - arm,
            },
        ],
    ]
}

/// kicad no connect bounds。
pub(crate) fn kicad_no_connect_bounds(at: KicadPoint) -> KicadBoundingBox {
    let arms = kicad_no_connect_arms(at);
    let points = [arms[0][0], arms[0][1], arms[1][0], arms[1][1]];
    kicad_points_bounds(&points, KICAD_CANVAS_LINE_BOUNDS_PADDING)
        .expect("KiCad no-connect marker bounds use four points")
}

/// kicad sheet box bounds。
pub(crate) fn kicad_sheet_box_bounds(
    at: Option<KicadAt>,
    size: Option<KicadSize>,
) -> Option<KicadBoundingBox> {
    let at = at?;
    let size = size?;
    let corners = [
        at.point(),
        KicadPoint {
            x: at.x + size.width,
            y: at.y,
        },
        KicadPoint {
            x: at.x + size.width,
            y: at.y + size.height,
        },
        KicadPoint {
            x: at.x,
            y: at.y + size.height,
        },
    ];
    kicad_points_bounds(&corners, 0.0)
}

/// kicad rotated rect corners。
pub(crate) fn kicad_rotated_rect_corners(at: KicadAt, size: KicadSize) -> [KicadPoint; 4] {
    let width = size.width.abs();
    let height = size.height.abs();
    let local_corners = [
        KicadPoint { x: 0.0, y: 0.0 },
        KicadPoint { x: width, y: 0.0 },
        KicadPoint {
            x: width,
            y: height,
        },
        KicadPoint { x: 0.0, y: height },
    ];
    local_corners.map(|corner| {
        let rotated = rotate_point(corner, at.rotation);
        KicadPoint {
            x: at.x + rotated.x,
            y: at.y + rotated.y,
        }
    })
}

/// kicad rotated rect bounds。
pub(crate) fn kicad_rotated_rect_bounds(at: KicadAt, size: KicadSize) -> Option<KicadBoundingBox> {
    let mut bounds = KicadBoundingBoxBuilder::default();
    for corner in kicad_rotated_rect_corners(at, size) {
        bounds.include(corner);
    }
    bounds.finish()
}

/// kicad rotated rect contains point。
pub(crate) fn kicad_rotated_rect_contains_point(
    at: KicadAt,
    size: KicadSize,
    point: KicadPoint,
) -> bool {
    let local = rotate_point(
        KicadPoint {
            x: point.x - at.x,
            y: point.y - at.y,
        },
        -at.rotation,
    );
    local.x >= 0.0 && local.x <= size.width.abs() && local.y >= 0.0 && local.y <= size.height.abs()
}

/// kicad text bounds。
pub(crate) fn kicad_text_bounds(
    text: &str,
    at: Option<KicadAt>,
    effects: Option<&KicadTextEffects>,
) -> Option<KicadBoundingBox> {
    let at = at?;
    let font_size = effects
        .and_then(|effects| effects.font_size)
        .unwrap_or(KicadSize {
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
    let width = (longest_line as f64 * char_width).max(KICAD_CANVAS_POINT_BOUNDS_RADIUS * 2.0);
    let height = (line_count as f64 * line_height).max(KICAD_CANVAS_POINT_BOUNDS_RADIUS * 2.0);

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
        KicadPoint { x: left, y: top },
        KicadPoint {
            x: left + width,
            y: top,
        },
        KicadPoint {
            x: left + width,
            y: top + height,
        },
        KicadPoint {
            x: left,
            y: top + height,
        },
    ];
    let mut bounds = KicadBoundingBoxBuilder::default();
    for corner in corners {
        let rotated = rotate_point(corner, at.rotation);
        bounds.include(KicadPoint {
            x: at.x + rotated.x,
            y: at.y + rotated.y,
        });
    }
    bounds
        .finish()
        .map(|bounds| bounds.padded(KICAD_CANVAS_LINE_BOUNDS_PADDING))
}

/// kicad polyline hits point。
pub(crate) fn kicad_polyline_hits_point(
    points: &[KicadPoint],
    stroke: Option<&KicadStroke>,
    point: KicadPoint,
) -> bool {
    if points.is_empty() {
        return false;
    }
    let tolerance = kicad_stroke_hit_tolerance(stroke);
    if points.len() == 1 {
        return kicad_point_distance(points[0], point) <= tolerance;
    }
    points
        .windows(2)
        .any(|segment| kicad_point_segment_distance(point, segment[0], segment[1]) <= tolerance)
}

/// kicad closed polyline hits point。
pub(crate) fn kicad_closed_polyline_hits_point(
    points: &[KicadPoint],
    stroke: Option<&KicadStroke>,
    point: KicadPoint,
) -> bool {
    if kicad_polyline_hits_point(points, stroke, point) {
        return true;
    }
    if points.len() < 3 {
        return false;
    }
    kicad_point_segment_distance(point, *points.last().unwrap(), points[0])
        <= kicad_stroke_hit_tolerance(stroke)
}

/// kicad bezier hits point。
pub(crate) fn kicad_bezier_hits_point(
    points: &[KicadPoint],
    stroke: Option<&KicadStroke>,
    point: KicadPoint,
) -> bool {
    if points.len() != 4 {
        return kicad_polyline_hits_point(points, stroke, point);
    }

    let sampled = kicad_cubic_bezier_samples(points[0], points[1], points[2], points[3]);
    kicad_polyline_hits_point(&sampled, stroke, point)
}

/// kicad cubic bezier samples。
pub(crate) fn kicad_cubic_bezier_samples(
    start: KicadPoint,
    control_1: KicadPoint,
    control_2: KicadPoint,
    end: KicadPoint,
) -> Vec<KicadPoint> {
    const KICAD_BEZIER_HIT_SEGMENTS: usize = 32;
    let mut points = Vec::with_capacity(KICAD_BEZIER_HIT_SEGMENTS + 1);
    for index in 0..=KICAD_BEZIER_HIT_SEGMENTS {
        let t = index as f64 / KICAD_BEZIER_HIT_SEGMENTS as f64;
        let one_minus_t = 1.0 - t;
        let start_weight = one_minus_t * one_minus_t * one_minus_t;
        let control_1_weight = 3.0 * one_minus_t * one_minus_t * t;
        let control_2_weight = 3.0 * one_minus_t * t * t;
        let end_weight = t * t * t;
        points.push(KicadPoint {
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

/// sample kicad arc points。
pub fn sample_kicad_arc_points(
    start: KicadPoint,
    mid: Option<KicadPoint>,
    end: KicadPoint,
) -> Vec<KicadPoint> {
    let Some(mid) = mid else {
        return vec![start, end];
    };
    let Some((center, radius)) = kicad_circle_from_three_points(start, mid, end) else {
        return vec![start, mid, end];
    };

    let start_angle = kicad_angle(center, start);
    let mid_angle = kicad_angle(center, mid);
    let end_angle = kicad_angle(center, end);
    let ccw_sweep = kicad_positive_angle_delta(start_angle, end_angle);
    let mid_on_ccw = kicad_positive_angle_delta(start_angle, mid_angle) <= ccw_sweep;
    let sweep = if mid_on_ccw {
        ccw_sweep
    } else {
        ccw_sweep - std::f64::consts::TAU
    };
    let segments = ((sweep.abs() / (std::f64::consts::PI / 32.0)).ceil() as usize).clamp(8, 96);
    let mut points = Vec::with_capacity(segments + 1);
    for index in 0..=segments {
        let angle = start_angle + sweep * index as f64 / segments as f64;
        points.push(KicadPoint {
            x: center.x + radius * angle.cos(),
            y: center.y + radius * angle.sin(),
        });
    }
    points
}

/// kicad arc hits point。
pub(crate) fn kicad_arc_hits_point(
    start: KicadPoint,
    mid: Option<KicadPoint>,
    end: KicadPoint,
    stroke: Option<&KicadStroke>,
    point: KicadPoint,
) -> bool {
    let sampled = sample_kicad_arc_points(start, mid, end);
    kicad_polyline_hits_point(&sampled, stroke, point)
}

/// kicad circle from three points。
pub(crate) fn kicad_circle_from_three_points(
    first: KicadPoint,
    second: KicadPoint,
    third: KicadPoint,
) -> Option<(KicadPoint, f64)> {
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
    let center = KicadPoint {
        x: (first_len * (second.y - third.y)
            + second_len * (third.y - first.y)
            + third_len * (first.y - second.y))
            / determinant,
        y: (first_len * (third.x - second.x)
            + second_len * (first.x - third.x)
            + third_len * (second.x - first.x))
            / determinant,
    };
    Some((center, kicad_point_distance(center, first)))
}

/// kicad angle。
pub(crate) fn kicad_angle(center: KicadPoint, point: KicadPoint) -> f64 {
    (point.y - center.y).atan2(point.x - center.x)
}

/// kicad positive angle delta。
pub(crate) fn kicad_positive_angle_delta(start: f64, end: f64) -> f64 {
    (end - start).rem_euclid(std::f64::consts::TAU)
}

/// kicad polygon contains point。
pub(crate) fn kicad_polygon_contains_point(points: &[KicadPoint], point: KicadPoint) -> bool {
    if points.len() < 3 {
        return false;
    }
    if kicad_closed_polyline_hits_point(points, None, point) {
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

/// kicad rectangle hits point。
pub(crate) fn kicad_rectangle_hits_point(
    start: KicadPoint,
    end: KicadPoint,
    stroke: Option<&KicadStroke>,
    fill: Option<&KicadFill>,
    point: KicadPoint,
) -> bool {
    let min = KicadPoint {
        x: start.x.min(end.x),
        y: start.y.min(end.y),
    };
    let max = KicadPoint {
        x: start.x.max(end.x),
        y: start.y.max(end.y),
    };
    let shape_bounds = KicadBoundingBox { min, max };
    if kicad_fill_is_solid(fill) && shape_bounds.contains(point) {
        return true;
    }

    let corners = [
        min,
        KicadPoint { x: max.x, y: min.y },
        max,
        KicadPoint { x: min.x, y: max.y },
        min,
    ];
    kicad_polyline_hits_point(&corners, stroke, point)
}

/// kicad circle hits point。
pub(crate) fn kicad_circle_hits_point(
    center: KicadPoint,
    radius: f64,
    stroke: Option<&KicadStroke>,
    fill: Option<&KicadFill>,
    point: KicadPoint,
) -> bool {
    let distance = kicad_point_distance(center, point);
    let radius = radius.abs();
    if kicad_fill_is_solid(fill) && distance <= radius {
        return true;
    }
    (distance - radius).abs() <= kicad_stroke_hit_tolerance(stroke)
}

/// kicad fill is solid。
pub(crate) fn kicad_fill_is_solid(fill: Option<&KicadFill>) -> bool {
    fill.and_then(|fill| fill.fill_type.as_deref())
        .is_some_and(|fill_type| !fill_type.eq_ignore_ascii_case("none"))
}

/// kicad stroke hit tolerance。
pub(crate) fn kicad_stroke_hit_tolerance(stroke: Option<&KicadStroke>) -> f64 {
    let stroke_radius = stroke.and_then(|stroke| stroke.width).unwrap_or(0.0).abs() / 2.0;
    KICAD_CANVAS_LINE_BOUNDS_PADDING.max(stroke_radius)
}

/// kicad point distance。
pub(crate) fn kicad_point_distance(left: KicadPoint, right: KicadPoint) -> f64 {
    let dx = left.x - right.x;
    let dy = left.y - right.y;
    (dx * dx + dy * dy).sqrt()
}

/// kicad point segment distance。
pub(crate) fn kicad_point_segment_distance(
    point: KicadPoint,
    start: KicadPoint,
    end: KicadPoint,
) -> f64 {
    let segment_x = end.x - start.x;
    let segment_y = end.y - start.y;
    let segment_len_sq = segment_x * segment_x + segment_y * segment_y;
    if segment_len_sq <= f64::EPSILON {
        return kicad_point_distance(point, start);
    }

    let projection =
        ((point.x - start.x) * segment_x + (point.y - start.y) * segment_y) / segment_len_sq;
    let projection = projection.clamp(0.0, 1.0);
    let closest = KicadPoint {
        x: start.x + projection * segment_x,
        y: start.y + projection * segment_y,
    };
    kicad_point_distance(point, closest)
}

/// kicad at bounds。
pub(crate) fn kicad_at_bounds(at: Option<KicadAt>, padding: f64) -> Option<KicadBoundingBox> {
    at.map(|at| kicad_point_bounds(at.point(), padding))
}

/// pin body end。
pub(crate) fn pin_body_end(at: KicadAt, length: f64) -> KicadPoint {
    let radians = at.rotation.to_radians();
    KicadPoint {
        x: at.x + length * radians.cos(),
        y: at.y + length * radians.sin(),
    }
}
