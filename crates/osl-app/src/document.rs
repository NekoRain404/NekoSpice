//! 原理图文档抽象层。封装 KiCad 原理图的加载、保存和编辑接口。
//!
use osl_kicad::{KicadEditSummary,
    KicadCanvasScene, KicadSchematic, KicadSchematicCheckReport, KicadSimulationDirective,
    read_kicad_schematic_with_libraries, write_kicad_schematic,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct KicadSymbolPlacementResult {
    pub(crate) summary: KicadEditSummary,
    pub(crate) reference: String,
    pub(crate) lib_id: String,
}

#[derive(Debug)]
/// KiCad 原理图文档封装。
///
/// 提供原理图的加载、保存、编辑（移动/旋转/删除元件）
/// 和快照/恢复接口，是原理图编辑的核心数据层。
pub(crate) struct KicadGuiDocument {
    path: PathBuf,
    pub(crate) schematic: KicadSchematic,
    pub(crate) dirty: bool,
}

impl KicadGuiDocument {
    /// load。
    pub(crate) fn load(path: PathBuf) -> Result<Self, String> {
        read_kicad_schematic_with_libraries(&path)
            .map(|schematic| Self {
                path,
                schematic,
                dirty: false,
            })
            .map_err(|error| error.to_string())
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
    pub(crate) fn scene(&self) -> KicadCanvasScene {
        self.schematic.canvas_scene()
    }

    /// simulation directives。
    pub(crate) fn simulation_directives(&self) -> Vec<KicadSimulationDirective> {
        self.schematic.simulation_directives()
    }

    /// check report。
    pub(crate) fn check_report(&self) -> KicadSchematicCheckReport {
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
        write_kicad_schematic(&self.path, &self.schematic)
            .inspect(|_| {
                self.dirty = false;
            })
            .map_err(|error| error.to_string())
    }

    /// Save the schematic to a new path and update the document path.
    pub(crate) fn save_as(&mut self, path: &std::path::Path) -> Result<(), String> {
        write_kicad_schematic(path, &self.schematic).map(|_| {
            self.path = path.to_path_buf();
            self.dirty = false;
        }).map_err(|error| error.to_string())
    }

    /// Return a deep copy of the current schematic for undo/redo storage.
    pub(crate) fn snapshot(&self) -> KicadSchematic {
        self.schematic.clone()
    }

    /// Replace the current schematic with a previously saved snapshot.
    ///
    /// Marks the document dirty so the next save will write the restored state.
    pub(crate) fn restore_snapshot(&mut self, snapshot: KicadSchematic) {
        self.schematic = snapshot;
        self.dirty = true;
    }
}


#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
