use super::NekoSpiceApp;
use super::localization::UiText;
use super::schematic_workspace_widgets::{
    bottom_console_line, bottom_tab, canvas_toolbar_button, document_tab, signal_row,
};
use super::theme::StudioTheme;
use eframe::egui::{self, Vec2};

impl NekoSpiceApp {
    pub(super) fn draw_schematic_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_schematic_workspace_toolbar(ui);
            ui.add_space(4.0);
            self.draw_schematic_document_tabs(ui);
            ui.add_space(4.0);
            let canvas_height = (ui.available_height() - 220.0).max(280.0);
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), canvas_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| self.draw_canvas(ui),
            );
            ui.add_space(6.0);
            self.draw_schematic_bottom_dock(ui);
        });
    }

    fn draw_schematic_workspace_toolbar(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal_wrapped(|ui| {
            canvas_toolbar_button(ui, mode, self.text(UiText::Save), self.document.is_some());
            if canvas_toolbar_button(ui, mode, self.text(UiText::Fit), true).clicked() {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }
            if canvas_toolbar_button(ui, mode, self.text(UiText::Run), self.document.is_some())
                .clicked()
            {
                self.run_simulation_from_panel();
            }
            ui.separator();
            canvas_toolbar_button(ui, mode, self.text(UiText::Wires), true);
            canvas_toolbar_button(ui, mode, self.text(UiText::Labels), true);
            canvas_toolbar_button(ui, mode, self.text(UiText::Buses), true);
            canvas_toolbar_button(ui, mode, self.text(UiText::Sheets), true);
            ui.separator();
            ui.label(StudioTheme::muted_for(mode, self.text(UiText::Zoom)));
            ui.label(StudioTheme::accent_for(
                mode,
                format!("{:.0}%", self.viewport.zoom * 10.0),
            ));
            ui.separator();
            ui.label(StudioTheme::muted_for(mode, self.text(UiText::Drc)));
            ui.colored_label(palette.success, self.text(UiText::Ready));
        });
    }

    fn draw_schematic_document_tabs(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_wrapped(|ui| {
            ui.label(StudioTheme::muted_for(
                mode,
                self.text(UiText::SchematicTabs),
            ));
            let active_name = self
                .document
                .as_ref()
                .and_then(|document| document.path().file_name())
                .and_then(|name| name.to_str())
                .unwrap_or(self.text(UiText::NoDocument));
            document_tab(ui, mode, active_name, true);
            document_tab(ui, mode, "bias_network.kicad_sch", false);
            document_tab(ui, mode, "protections.kicad_sch", false);
            if ui.small_button("+").clicked() {
                self.status_message = Some(self.text(UiText::NewSchematic).to_string());
            }
        });
    }

    /// Bottom dock panel: signal list, console output, and tab bar.
    /// Dynamically shows signals from the loaded schematic's labels and
    /// status messages from the most recent simulation run.
    fn draw_schematic_bottom_dock(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        egui::Frame::new()
            .fill(palette.panel_soft)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                // Tab bar for bottom panels
                ui.horizontal_wrapped(|ui| {
                    bottom_tab(ui, mode, self.text(UiText::Waveforms), true);
                    bottom_tab(ui, mode, "FFT", false);
                    bottom_tab(ui, mode, "Bode", false);
                    bottom_tab(ui, mode, self.text(UiText::StatusConsole), false);
                    bottom_tab(ui, mode, self.text(UiText::Netlist), false);
                    bottom_tab(ui, mode, "ERC", false);
                    bottom_tab(ui, mode, self.text(UiText::Inspector), false);
                });
                ui.separator();

                // Two-column layout: signal list (left) + console (right)
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() * 0.30).max(180.0));
                        // Show dynamic signals from loaded schematic labels
                        if let Some(document) = &self.document {
                            let scene = document.scene();
                            let mut signal_count = 0;
                            for symbol in &scene.symbols {
                                if !symbol.reference.is_empty() {
                                    signal_count += 1;
                                    let label = format!("V({})", symbol.reference);
                                    let color = match signal_count % 4 {
                                        0 => palette.accent,
                                        1 => palette.success,
                                        2 => palette.warning,
                                        _ => palette.danger,
                                    };
                                    signal_row(ui, mode, &label, "auto", color);
                                }
                            }
                            if signal_count == 0 {
                                ui.label(StudioTheme::muted_for(mode, "No signals detected"));
                            }
                        } else {
                            // Fallback: show placeholder signals
                            signal_row(ui, mode, "V(IN)", "1.000 V/div", palette.accent);
                            signal_row(ui, mode, "V(OUT)", "1.000 V/div", palette.success);
                        }
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        // Show actual status messages and simulation results
                        if let Some(msg) = &self.status_message {
                            bottom_console_line(ui, mode, msg, palette.success);
                        }
                        if let Some(error) = &self.simulation_panel.last_error {
                            bottom_console_line(ui, mode, error, palette.danger);
                        }
                        if let Some(run) = &self.simulation_panel.last_run {
                            bottom_console_line(
                                ui,
                                mode,
                                &format!(
                                    "Last run: {} — {} ms",
                                    run.metadata.status.as_str(),
                                    run.metadata.duration_ms
                                ),
                                palette.text_muted,
                            );
                        }
                        // Show netlist directive count from loaded schematic
                        if let Some(document) = &self.document {
                            let dir_count = document.simulation_directives().len();
                            bottom_console_line(
                                ui,
                                mode,
                                &format!("Directives: {dir_count}"),
                                palette.text_muted,
                            );
                        }
                    });
                });
            });
    }
}
