//! 编辑操作与文档加载。将 [`NekoSpiceApp`] 的文件 I/O 和结构编辑方法分离到此模块，
//! 使 `app.rs` 仅保留核心结构体定义与模块声明。
//!
//! 包含：加载原理图/符号库、删除/移动/旋转选中项、撤销/重做、保存文档。

use crate::document::KicadGuiDocument;
use crate::library::KicadGuiLibrary;
use crate::placement_config::SymbolPlacementConfig;
use crate::DEFAULT_GUI_LIBRARY_TABLE;
use osl_kicad::KicadPoint;
use std::path::PathBuf;

use super::NekoSpiceApp;

impl NekoSpiceApp {
    // ── 加载操作 ────────────────────────────────────────────────────────

    /// 加载初始仿真结果（由环境变量 `NEKOSPICE_INITIAL_RUN_DIR` 控制）。
    fn load_initial_simulation_run(&mut self) {
        let Some(path) = std::env::var_os("NEKOSPICE_INITIAL_RUN_DIR") else {
            return;
        };
        match crate::simulation::GuiSimulationRun::from_output_dir(PathBuf::from(path)) {
            Ok(run) => {
                self.sync_selected_waveform_signal(&run.waveform);
                self.simulation_panel.last_run = Some(run);
            }
            Err(error) => {
                self.simulation_panel.last_error = Some(error.clone());
                self.status_message = Some(error);
            }
        }
    }

    /// 从指定路径加载 KiCad 原理图文件，重置视口和编辑历史。
    pub(super) fn load_schematic(&mut self, path: PathBuf) {
        match KicadGuiDocument::load(path.clone()) {
            Ok(document) => {
                let scene = document.scene();
                self.schematic_path = path.display().to_string();
                self.viewport.fit_scene(scene.bounds);
                self.document = Some(document);
                self.scene = Some(scene);
                self.selected_hit = None;
                self.clear_property_editor();
                self.schematic_tools.clear_pending();
                self.load_error = None;
                self.history.clear();
                self.status_message = Some("Loaded schematic".to_string());
            }
            Err(error) => {
                self.load_error = Some(error.to_string());
                self.status_message = None;
            }
        }
    }

    /// 从指定路径加载 KiCad 符号库。
    pub(super) fn load_symbol_library(&mut self, path: PathBuf) {
        match KicadGuiLibrary::load(path.clone()) {
            Ok(library) => {
                self.library_table_path = path.display().to_string();
                self.library = Some(library);
                self.selected_symbol_id = self
                    .library
                    .as_ref()
                    .and_then(|library| library.index().symbols.first())
                    .map(|symbol| symbol.id.clone());
                self.selected_symbol_placement = SymbolPlacementConfig::default();
                self.placement = None;
                self.library_error = None;
                self.status_message = Some("Loaded symbol library".to_string());
            }
            Err(error) => {
                self.library = None;
                self.selected_symbol_id = None;
                self.selected_symbol_placement = SymbolPlacementConfig::default();
                self.placement = None;
                self.library_error = Some(error.to_string());
            }
        }
    }

    /// 公共入口：Default impl 调用，按顺序加载初始原理图、符号库和仿真结果。
    pub(super) fn load_initial_resources(&mut self) {
        self.load_schematic(PathBuf::from(crate::DEFAULT_GUI_SCHEMATIC));
        self.load_symbol_library(PathBuf::from(DEFAULT_GUI_LIBRARY_TABLE));
        self.load_initial_simulation_run();
    }

    // ── 编辑操作 ────────────────────────────────────────────────────────

    /// 删除当前选中的原理图元素，自动创建撤销快照。
    pub(super) fn delete_selected(&mut self) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        // 编辑前保存快照以支持撤销
        self.history.push(document.snapshot());

        match document.delete_item(&uuid) {
            Ok(summary) => {
                let scene = document.scene();
                self.viewport.fit_scene(scene.bounds);
                self.scene = Some(scene);
                self.selected_hit = None;
                self.clear_property_editor();
                self.load_error = None;
                self.status_message =
                    Some(format!("Deleted {} {}", summary.operation, summary.target));
                self.history.clear_redo();
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    /// 将选中元素移动指定偏移量（内部方法）。
    fn move_selected(&mut self, delta: KicadPoint) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        self.history.push(document.snapshot());

        match document.move_item(&uuid, delta) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit = scene.item_hit_by_uuid(&uuid);
                self.scene = Some(scene);
                self.sync_property_editor_from_selection();
                self.load_error = None;
                self.status_message =
                    Some(format!("Moved {} {}", summary.operation, summary.target));
                self.history.clear_redo();
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    /// 微调选中元素位置（方向键触发，步长 2.54mm）。
    pub(super) fn nudge_selected(&mut self, direction: super::EditNudgeDirection) {
        self.move_selected(direction.delta());
    }

    /// 将选中元素旋转 90°，自动创建撤销快照。
    pub(super) fn rotate_selected(&mut self) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        self.history.push(document.snapshot());

        match document.rotate_item(&uuid, 90.0) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit = scene.item_hit_by_uuid(&uuid);
                self.scene = Some(scene);
                self.sync_property_editor_from_selection();
                self.load_error = None;
                self.status_message =
                    Some(format!("Rotated {} {}", summary.operation, summary.target));
                self.history.clear_redo();
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    // ── 撤销/重做 ────────────────────────────────────────────────────────

    /// 撤销上一步编辑操作，恢复到之前的快照。
    pub(super) fn undo(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let current = document.snapshot();
        let Some(previous) = self.history.undo(current) else {
            self.status_message = Some("Nothing to undo".to_string());
            return;
        };
        document.restore_snapshot(previous);
        let scene = document.scene();
        self.viewport.fit_scene(scene.bounds);
        self.scene = Some(scene);
        self.selected_hit = None;
        self.clear_property_editor();
        self.status_message = Some("Undo".to_string());
    }

    /// 重做上一步被撤销的编辑操作。
    pub(super) fn redo(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        let current = document.snapshot();
        let Some(next) = self.history.redo(current) else {
            self.status_message = Some("Nothing to redo".to_string());
            return;
        };
        document.restore_snapshot(next);
        let scene = document.scene();
        self.viewport.fit_scene(scene.bounds);
        self.scene = Some(scene);
        self.selected_hit = None;
        self.clear_property_editor();
        self.status_message = Some("Redo".to_string());
    }

    // ── 持久化 ──────────────────────────────────────────────────────────

    /// 保存当前文档到磁盘。
    pub(super) fn save_document(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        match document.save() {
            Ok(()) => {
                self.status_message = Some(format!("Saved {}", document.path().display()));
                self.load_error = None;
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }
}
