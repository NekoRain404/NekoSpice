//! 原理图文档抽象层。封装 schema 原理图的加载、保存和编辑接口。
//!
use nsp_schema::{
    NspCanvasScene, NspEditSummary, NspSchematic, NspSchematicCheckReport, NspSimulationDirective,
    new_schema_empty, read_schematic_with_libraries, write_schematic,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NspSymbolPlacementResult {
    pub(crate) summary: NspEditSummary,
    pub(crate) reference: String,
    pub(crate) lib_id: String,
}

#[derive(Debug)]
/// schema 原理图文档封装。
///
/// 提供原理图的加载、保存、编辑（移动/旋转/删除元件）
/// 和快照/恢复接口，是原理图编辑的核心数据层。
pub(crate) struct NspGuiDocument {
    path: PathBuf,
    pub(crate) schematic: NspSchematic,
    pub(crate) dirty: bool,
}

impl NspGuiDocument {
    /// load。
    pub(crate) fn load(path: PathBuf) -> Result<Self, String> {
        read_schematic_with_libraries(&path)
            .map(|schematic| Self {
                path,
                schematic,
                dirty: false,
            })
            .map_err(|error| error.to_string())
    }

    /// Create a new empty schematic document.
    ///
    /// The document is created in-memory with a default title block
    /// and must be saved to a path before persisting to disk.
    pub(crate) fn new_empty(name: &str) -> Self {
        let mut schematic = new_schema_empty();
        if let Some(tb) = &mut schematic.title_block {
            tb.title = Some(name.to_string());
        }
        Self {
            path: std::path::PathBuf::from(format!("<unsaved:{name}>")),
            schematic,
            dirty: true,
        }
    }

    /// path。
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// is dirty。
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// scene。
    pub(crate) fn scene(&self) -> NspCanvasScene {
        self.schematic.canvas_scene()
    }

    /// simulation directives。
    pub(crate) fn simulation_directives(&self) -> Vec<NspSimulationDirective> {
        self.schematic.simulation_directives()
    }

    /// check report。
    pub(crate) fn check_report(&self) -> NspSchematicCheckReport {
        self.schematic.check_report()
    }

    /// spice netlist preview。
    pub(crate) fn spice_netlist_preview(&self) -> Result<String, String> {
        self.schematic
            .to_spice_netlist()
            .map_err(|error| error.to_string())
    }

    /// delete item。
    pub(crate) fn save(&mut self) -> Result<(), String> {
        write_schematic(&self.path, &self.schematic)
            .inspect(|_| {
                self.dirty = false;
            })
            .map_err(|error| error.to_string())
    }

    /// Save the schematic to a new path and update the document path.
    pub(crate) fn save_as(&mut self, path: &std::path::Path) -> Result<(), String> {
        write_schematic(path, &self.schematic)
            .map(|_| {
                self.path = path.to_path_buf();
                self.dirty = false;
            })
            .map_err(|error| error.to_string())
    }

    /// Return a deep copy of the current schematic for undo/redo storage.
    pub(crate) fn snapshot(&self) -> NspSchematic {
        self.schematic.clone()
    }

    /// Replace the current schematic with a previously saved snapshot.
    ///
    /// Marks the document dirty so the next save will write the restored state.
    pub(crate) fn restore_snapshot(&mut self, snapshot: NspSchematic) {
        self.schematic = snapshot;
        self.dirty = true;
    }
}

#[cfg(test)]
#[path = "document_tests/mod.rs"]
mod tests;
