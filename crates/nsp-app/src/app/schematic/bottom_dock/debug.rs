//! Bottom dock debug tabs: Console, Netlist, ERC, and Inspector.
//!
//! These tabs provide diagnostic output, SPICE netlist preview,
//! electrical rules check results, and selected-item inspection.

use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// 控制台标签页：状态消息、错误和仿真输出日志。
    pub(crate) fn draw_bottom_console_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        if let Some(msg) = &self.status_message {
            super::super::workspace_widgets::bottom_console_line(ui, mode, msg, palette.success);
        }
        if let Some(error) = &self.simulation_panel.last_error {
            super::super::workspace_widgets::bottom_console_line(ui, mode, error, palette.danger);
        }
        // 显示完整的 ngspice/xyce 仿真日志
        if let Some(run) = &self.simulation_panel.last_run {
            let log_path = run.output_dir.join("ngspice.log");
            let fallback = run.output_dir.join("xyce.log");
            let actual = if log_path.is_file() {
                log_path
            } else {
                fallback
            };
            if actual.is_file()
                && let Ok(content) = std::fs::read_to_string(&actual)
            {
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        ui.monospace(&content);
                    });
            }
        }
        if let Some(document) = &self.document {
            let dir_count = document.simulation_directives().len();
            super::super::workspace_widgets::bottom_console_line(
                ui,
                mode,
                &format!("Directives: {dir_count}"),
                palette.text_muted,
            );
        }
        if self.status_message.is_none()
            && self.simulation_panel.last_error.is_none()
            && self.simulation_panel.last_run.is_none()
        {
            ui.label(StudioTheme::muted_for(mode, "No output yet"));
        }
    }

    /// 网表标签页：从加载的原理图生成 SPICE 网表预览。
    pub(crate) fn draw_bottom_netlist_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let Some(document) = &self.document else {
            ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
            return;
        };
        let profile = self.build_simulation_profile();
        match document
            .spice_netlist_preview()
            .map(|netlist| nsp_sim::inject_profile_directives(&netlist, &profile))
        {
            Ok(netlist) => {
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.monospace(netlist);
                    });
            }
            Err(error) => {
                let palette = self.theme_palette();
                super::super::workspace_widgets::bottom_console_line(
                    ui,
                    mode,
                    &format!("Netlist generation failed: {error}"),
                    palette.danger,
                );
            }
        }
    }

    /// ERC 标签页：电气规则检查结果。
    pub(crate) fn draw_bottom_erc_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let Some(document) = &self.document else {
            ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
            return;
        };
        let report = document.check_report();
        let (error_count, warning_count) = (report.error_count(), report.warning_count());
        super::super::workspace_widgets::bottom_console_line(
            ui,
            mode,
            &format!("ERC: {error_count} errors, {warning_count} warnings"),
            if error_count > 0 {
                palette.danger
            } else if warning_count > 0 {
                palette.warning
            } else {
                palette.success
            },
        );
        egui::ScrollArea::vertical()
            .max_height(120.0)
            .show(ui, |ui| {
                for diag in &report.diagnostics {
                    use nsp_schema::NspDiagnosticSeverity;
                    let (prefix, color) = match diag.severity {
                        NspDiagnosticSeverity::Error => ("ERROR", palette.danger),
                        NspDiagnosticSeverity::Warning => ("WARNING", palette.warning),
                        NspDiagnosticSeverity::Info => ("INFO", palette.text_muted),
                    };
                    super::super::workspace_widgets::bottom_console_line(
                        ui,
                        mode,
                        &format!("{prefix}: {}", diag.message),
                        color,
                    );
                }
            });
    }

    /// 检查器标签页：当前选中项的属性显示。
    pub(crate) fn draw_bottom_inspector_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let Some(hit) = &self.selected_hit else {
            ui.label(StudioTheme::muted_for(mode, "Click an item to inspect"));
            return;
        };
        let palette = self.theme_palette();
        ui.label(StudioTheme::section_title_for(
            mode,
            format!("Selected: {}", hit.kind),
        ));
        if let Some(ref uuid) = hit.uuid {
            ui.label(StudioTheme::muted_for(mode, format!("UUID: {uuid}")));
        }
        super::super::workspace_widgets::bottom_console_line(
            ui,
            mode,
            &format!(
                "Bounds: ({:.1}, {:.1}) — ({:.1}, {:.1})",
                hit.bounds.min.x, hit.bounds.min.y, hit.bounds.max.x, hit.bounds.max.y
            ),
            palette.text_muted,
        );
    }
}
