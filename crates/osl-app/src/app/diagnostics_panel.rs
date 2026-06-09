use super::status_strip::severity_color;
use super::{NekoSpiceApp, theme::StudioTheme};
use eframe::egui;
use osl_kicad::KicadDiagnosticSeverity;

impl NekoSpiceApp {
    pub(super) fn draw_document_diagnostics_panel(&self, ui: &mut egui::Ui, max_height: f32) {
        StudioTheme::panel_frame().show(ui, |ui| {
            ui.label(StudioTheme::section_title("Diagnostics"));
            let Some(document) = &self.document else {
                ui.label(StudioTheme::muted("No editable schematic loaded"));
                return;
            };
            let report = document.check_report();
            ui.horizontal(|ui| {
                ui.colored_label(
                    severity_color(KicadDiagnosticSeverity::Error),
                    format!("{} errors", report.error_count()),
                );
                ui.colored_label(
                    severity_color(KicadDiagnosticSeverity::Warning),
                    format!("{} warnings", report.warning_count()),
                );
                ui.label(StudioTheme::muted(format!("{} info", report.info_count())));
            });
            egui::ScrollArea::vertical()
                .id_salt("studio_document_diagnostics")
                .max_height(max_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if report.diagnostics.is_empty() {
                        ui.label(StudioTheme::muted("No diagnostics"));
                    }
                    for diagnostic in &report.diagnostics {
                        ui.colored_label(
                            severity_color(diagnostic.severity),
                            format!("{}: {}", diagnostic.code, diagnostic.message),
                        );
                    }
                });
        });
    }
}
