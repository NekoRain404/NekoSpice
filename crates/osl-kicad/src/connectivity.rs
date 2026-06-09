use crate::{KicadPoint, KicadSchematic, KicadSize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNetGraph {
    pub nets: Vec<KicadNet>,
    nets_by_point: BTreeMap<PointKey, String>,
}

impl KicadNetGraph {
    pub(crate) fn build(schematic: &KicadSchematic) -> Self {
        let mut points = BTreeMap::<PointKey, KicadPoint>::new();
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
        let mut points_by_net = BTreeMap::<String, Vec<KicadPoint>>::new();
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
            .map(|(name, points)| KicadNet { name, points })
            .collect();

        Self {
            nets,
            nets_by_point,
        }
    }

    pub fn net_at(&self, point: KicadPoint) -> Option<&str> {
        self.nets_by_point
            .get(&PointKey::from(point))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KicadNet {
    pub name: String,
    pub points: Vec<KicadPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PointKey {
    x: i64,
    y: i64,
}

impl From<KicadPoint> for PointKey {
    fn from(point: KicadPoint) -> Self {
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

fn insert_point(points: &mut BTreeMap<PointKey, KicadPoint>, point: KicadPoint) {
    points.entry(PointKey::from(point)).or_insert(point);
}

fn segment_contains_point(start: KicadPoint, end: KicadPoint, point: KicadPoint) -> bool {
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

pub(crate) fn coordinate_key(value: f64) -> i64 {
    (value * 1_000_000.0).round() as i64
}

pub(crate) fn same_point(left: KicadPoint, right: KicadPoint) -> bool {
    coordinate_key(left.x) == coordinate_key(right.x)
        && coordinate_key(left.y) == coordinate_key(right.y)
}

pub(crate) fn same_size(left: KicadSize, right: KicadSize) -> bool {
    coordinate_key(left.width) == coordinate_key(right.width)
        && coordinate_key(left.height) == coordinate_key(right.height)
}

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
