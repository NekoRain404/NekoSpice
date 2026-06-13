//! Schematic connectivity graph — net detection, bus alias resolution, and net-to-pin mapping.

use crate::transform::transform_symbol_point;
use crate::{NspPoint, NspSchematic, NspSize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct NspNetGraph {
    pub nets: Vec<NspNet>,
    nets_by_point: BTreeMap<PointKey, String>,
}

impl NspNetGraph {
    /// build。
    pub(crate) fn build(schematic: &NspSchematic) -> Self {
        let mut points = BTreeMap::<PointKey, NspPoint>::new();
        for wire in &schematic.wires {
            for point in &wire.points {
                insert_point(&mut points, *point);
            }
        }
        for label in &schematic.labels {
            if let Some(at) = label.at {
                insert_point(&mut points, at.point());
            }
        }
        for junction in &schematic.junctions {
            insert_point(&mut points, junction.at);
        }
        for point in schematic.symbol_pin_points() {
            insert_point(&mut points, point);
        }
        for point in schematic.sheet_pin_points() {
            insert_point(&mut points, point);
        }

        let ordered_keys = points.keys().copied().collect::<Vec<_>>();
        let indexes = ordered_keys
            .iter()
            .enumerate()
            .map(|(index, key)| (*key, index))
            .collect::<BTreeMap<_, _>>();
        let mut graph = DisjointSet::new(ordered_keys.len());

        for wire in &schematic.wires {
            for segment in wire.points.windows(2) {
                let mut segment_indexes = ordered_keys
                    .iter()
                    .filter(|key| {
                        points.get(key).is_some_and(|point| {
                            segment_contains_point(segment[0], segment[1], *point)
                        })
                    })
                    .filter_map(|key| indexes.get(key).copied())
                    .collect::<Vec<_>>();
                segment_indexes.sort_unstable();
                if let Some(first) = segment_indexes.first().copied() {
                    for index in segment_indexes.into_iter().skip(1) {
                        graph.union(first, index);
                    }
                }
            }
        }

        let mut labels_by_name = BTreeMap::<String, Vec<usize>>::new();
        for label in &schematic.labels {
            if let Some(at) = label.at
                && let Some(index) = indexes.get(&PointKey::from(at.point())).copied()
            {
                labels_by_name
                    .entry(normalize_net_name(&label.text))
                    .or_default()
                    .push(index);
            }
        }
        for label_indexes in labels_by_name.values() {
            if let Some(first) = label_indexes.first().copied() {
                for index in label_indexes.iter().copied().skip(1) {
                    graph.union(first, index);
                }
            }
        }

        let mut labels_by_root = BTreeMap::<usize, BTreeSet<String>>::new();
        for label in &schematic.labels {
            if let Some(at) = label.at
                && let Some(index) = indexes.get(&PointKey::from(at.point())).copied()
            {
                labels_by_root
                    .entry(graph.find(index))
                    .or_default()
                    .insert(normalize_net_name(&label.text));
            }
        }
        // Power port symbols define net names (GND → "0", VCC → "VCC", etc.)
        // Each power symbol's pin connects its net name to the point where it's placed.
        for symbol in &schematic.symbols {
            let Some(symbol_at) = symbol.at else { continue };
            // Check if this is a power symbol by looking at its value and reference
            let reference = symbol.reference().unwrap_or_default().trim().to_string();
            if reference.starts_with('#') {
                // Power symbols have references like #PWR01, #PWR02, etc.
                // Their value defines the net name (GND, VCC, VDD, etc.)
                let value = symbol.value().unwrap_or_default().trim().to_string();
                // Skip PWR_FLAG — it's an EDA-specific marker, not a real net
                if value.eq_ignore_ascii_case("PWR_FLAG") {
                    continue;
                }
                if !value.is_empty() {
                    // Get the power pin position
                    if let Some(definition) = schematic.resolved_symbol_definition(&symbol.lib_id) {
                        for pin in definition.scoped_pins(symbol.unit, symbol.body_style) {
                            if let Some(pin_at) = pin.at {
                                let world_point = transform_symbol_point(
                                    pin_at,
                                    symbol_at,
                                    symbol.mirror.as_deref(),
                                );
                                if let Some(index) =
                                    indexes.get(&PointKey::from(world_point)).copied()
                                {
                                    let net_name = normalize_net_name(&value);
                                    labels_by_root
                                        .entry(graph.find(index))
                                        .or_default()
                                        .insert(net_name);
                                }
                            }
                        }
                    }
                }
            }
        }

        let mut names_by_root = BTreeMap::<usize, String>::new();
        let mut generated_index = 1;
        for index in 0..ordered_keys.len() {
            let root = graph.find(index);
            names_by_root.entry(root).or_insert_with(|| {
                preferred_net_label(labels_by_root.get(&root)).unwrap_or_else(|| {
                    let name = format!("n{generated_index:03}");
                    generated_index += 1;
                    name
                })
            });
        }

        let mut nets_by_point = BTreeMap::new();
        let mut points_by_net = BTreeMap::<String, Vec<NspPoint>>::new();
        for (index, key) in ordered_keys.iter().enumerate() {
            let root = graph.find(index);
            let name = names_by_root
                .get(&root)
                .cloned()
                .unwrap_or_else(|| "n000".to_string());
            nets_by_point.insert(*key, name.clone());
            if let Some(point) = points.get(key).copied() {
                points_by_net.entry(name).or_default().push(point);
            }
        }

        let nets = points_by_net
            .into_iter()
            .map(|(name, points)| NspNet { name, points })
            .collect();

        Self {
            nets,
            nets_by_point,
        }
    }

    /// net at。
    pub fn net_at(&self, point: NspPoint) -> Option<&str> {
        self.nets_by_point
            .get(&PointKey::from(point))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NspNet {
    pub name: String,
    pub points: Vec<NspPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PointKey {
    x: i64,
    y: i64,
}

impl From<NspPoint> for PointKey {
    fn from(point: NspPoint) -> Self {
        Self {
            x: coordinate_key(point.x),
            y: coordinate_key(point.y),
        }
    }
}

#[derive(Debug)]
struct DisjointSet {
    parents: Vec<usize>,
}

impl DisjointSet {
    fn new(len: usize) -> Self {
        Self {
            parents: (0..len).collect(),
        }
    }

    fn find(&mut self, item: usize) -> usize {
        let parent = self.parents[item];
        if parent == item {
            item
        } else {
            let root = self.find(parent);
            self.parents[item] = root;
            root
        }
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents[right_root] = left_root;
        }
    }
}

fn insert_point(points: &mut BTreeMap<PointKey, NspPoint>, point: NspPoint) {
    points.entry(PointKey::from(point)).or_insert(point);
}

fn segment_contains_point(start: NspPoint, end: NspPoint, point: NspPoint) -> bool {
    let cross = (point.y - start.y) * (end.x - start.x) - (point.x - start.x) * (end.y - start.y);
    if cross.abs() > 1e-6 {
        return false;
    }

    between_inclusive(point.x, start.x, end.x) && between_inclusive(point.y, start.y, end.y)
}

fn between_inclusive(value: f64, left: f64, right: f64) -> bool {
    let min = left.min(right) - 1e-6;
    let max = left.max(right) + 1e-6;
    value >= min && value <= max
}

/// coordinate key。
pub(crate) fn coordinate_key(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

/// same point。
pub(crate) fn same_point(left: NspPoint, right: NspPoint) -> bool {
    coordinate_key(left.x) == coordinate_key(right.x)
        && coordinate_key(left.y) == coordinate_key(right.y)
}

/// same size。
pub(crate) fn same_size(left: NspSize, right: NspSize) -> bool {
    coordinate_key(left.width) == coordinate_key(right.width)
        && coordinate_key(left.height) == coordinate_key(right.height)
}

/// normalize net name。
pub(crate) fn normalize_net_name(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "gnd" | "agnd" | "dgnd" | "earth" => "0".to_string(),
        _ => name.trim().to_string(),
    }
}

fn preferred_net_label(labels: Option<&BTreeSet<String>>) -> Option<String> {
    let labels = labels?;
    labels
        .iter()
        .find(|label| label.as_str() == "0")
        .cloned()
        .or_else(|| labels.iter().find(|label| !label.is_empty()).cloned())
}
