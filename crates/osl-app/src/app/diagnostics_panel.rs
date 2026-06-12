//! 诊断信息面板。显示 ERC 错误、警告和仿真诊断结果。
//!
use super::localization::UiText;
use super::status_strip::severity_color;
use super::{NekoSpiceApp, theme::StudioTheme};
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;

impl NekoSpiceApp {
    /// draw document diagnostics panel。
    pub(super) fn draw_document_diagnostics_panel(&self, ui: &mut egui::Ui, max_height: f32) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Diagnostics),
            ));
            let Some(document) = &self.document else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoEditableSchematicLoaded),
                ));
                return;
            };
            let report = document.check_report();
            ui.horizontal(|ui| {
                ui.colored_label(
                    severity_color(mode, KicadDiagnosticSeverity::Error),
                    format!("{} errors", report.error_count()),
                );
                ui.colored_label(
                    severity_color(mode, KicadDiagnosticSeverity::Warning),
                    format!("{} warnings", report.warning_count()),
                );
                ui.label(StudioTheme::muted_for(
                    mode,
                    format!("{} {}", report.info_count(), self.text(UiText::Info)),
                ));
            });
            egui::ScrollArea::vertical()
                .id_salt("studio_document_diagnostics")
                .max_height(max_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if report.diagnostics.is_empty() {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            self.text(UiText::NoDiagnostics),
                        ));
                    }
                    for diagnostic in &report.diagnostics {
                        ui.colored_label(
                            severity_color(mode, diagnostic.severity),
                            format!("{}: {}", diagnostic.code, diagnostic.message),
                        );
                    }
                });
        });
    }
}
