
use crate::{ImportDiagnostic, ImportSeverity, is_ground_node};
use osl_core::read_text;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::path::{Path, PathBuf};

pub(crate) fn is_ltspice_schematic(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("asc"))
}

#[derive(Debug)]
pub(crate) struct LtspiceSchematicImport {
    pub(crate) netlist: String,
    pub(crate) diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug, Default)]
struct LtspiceSchematic {
    wires: Vec<LtspiceWire>,
    flags: Vec<LtspiceFlag>,
    symbols: Vec<LtspiceSymbol>,
    directives: Vec<LtspiceDirective>,
    diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AscPoint {
    x: i32,
    y: i32,
}

impl AscPoint {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug)]
struct LtspiceWire {
    start: AscPoint,
    end: AscPoint,
}

#[derive(Debug)]
struct LtspiceFlag {
    point: AscPoint,
    name: String,
}

#[derive(Debug)]
struct LtspiceSymbol {
    line: usize,
    name: String,
    origin: AscPoint,
    rotation: String,
    attrs: BTreeMap<String, String>,
}

#[derive(Debug)]
struct LtspiceDirective {
    text: String,
}

#[derive(Debug, Clone)]
struct LtspiceSymbolSpec {
    prefix: String,
    pins: Vec<AscPoint>,
    source: LtspiceSymbolSpecSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LtspiceSymbolSpecSource {
    Builtin,
    AsyFile,
}

#[derive(Debug)]
struct LtspiceSymbolLibrary {
    search_dirs: Vec<PathBuf>,
    cache: BTreeMap<String, Option<LtspiceSymbolSpec>>,
    diagnostics: Vec<ImportDiagnostic>,
}

#[derive(Debug)]
struct AscNetGraph {
    names: BTreeMap<AscPoint, String>,
    has_ground: bool,
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
