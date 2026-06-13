//! Simulation export panel — provides export actions for netlists, CSV,
//! and simulation reports in backend-aware formats.
//!
//! Each export action checks the current backend (ngspice/Xyce) and
//! formats the output accordingly.

use crate::app::NekoSpiceApp;
use super::state::SimulationBackendKind;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Draw the export options panel in the overview workspace.
    ///
    /// Shows export buttons for netlist (.cir), CSV waveform data,
    /// and simulation report, with backend-aware formatting hints.
    pub(crate) fn draw_export_options_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);
        let has_run = self.simulation_panel.last_run.is_some();
        let has_doc = self.document.is_some();

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Export"));
            ui.add_space(4.0);

            // Netlist export
            let netlist_btn = ui.add_enabled(
                has_doc,
                egui::Button::new(
                    egui::RichText::new("Export Netlist (.cir)").color(palette.text),
                )
                .fill(palette.panel_soft)
                .stroke(egui::Stroke::new(1.0, palette.border))
                .min_size(egui::Vec2::new(ui.available_width(), 28.0)),
            );
            if netlist_btn.clicked() {
                self.export_netlist_dialog();
            }
            netlist_btn.on_hover_text(match self.simulation_panel.backend {
                SimulationBackendKind::Ngspice => "Export ngspice-compatible netlist",
                SimulationBackendKind::Xyce => "Export Xyce-compatible netlist with .print directives",
            });

            ui.add_space(4.0);

            // CSV export (only available after a successful run)
            let csv_btn = ui.add_enabled(
                has_run,
                egui::Button::new(
                    egui::RichText::new("Export CSV").color(palette.text),
                )
                .fill(palette.panel_soft)
                .stroke(egui::Stroke::new(1.0, palette.border))
                .min_size(egui::Vec2::new(ui.available_width(), 28.0)),
            );
            csv_btn.on_hover_text("Export waveform data as CSV for external analysis");

            ui.add_space(4.0);

            // Open output directory
            if has_run {
                let dir_btn = ui.add(
                    egui::Button::new(
                        egui::RichText::new("Open Output Directory").color(palette.text),
                    )
                    .fill(palette.panel_soft)
                    .stroke(egui::Stroke::new(1.0, palette.border))
                    .min_size(egui::Vec2::new(ui.available_width(), 28.0)),
                );
                if dir_btn.clicked() {
                    if let Some(run) = &self.simulation_panel.last_run {
                        let path = run.output_dir.display().to_string();
                        ui.ctx().copy_text(path.clone());
                        self.status_message = Some(format!("Output path copied: {}", path));
                    }
                }
            }
        });
    }
}
