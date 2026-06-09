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

    fn draw_schematic_bottom_dock(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        egui::Frame::new()
            .fill(palette.panel_soft)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
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
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() * 0.30).max(180.0));
                        signal_row(ui, mode, "V(IN)", "1.000 V/div", palette.accent);
                        signal_row(ui, mode, "V(OUT)", "1.000 V/div", palette.success);
                        signal_row(ui, mode, "V(+15V)", "5.00 V/div", palette.warning);
                        signal_row(ui, mode, "V(-15V)", "5.00 V/div", palette.danger);
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        bottom_console_line(
                            ui,
                            mode,
                            "[ngspice] transient analysis ready",
                            palette.success,
                        );
                        bottom_console_line(
                            ui,
                            mode,
                            "Samples: 5001 | Step: 2.00 us | Stop: 10.00 ms",
                            palette.text_muted,
                        );
                        bottom_console_line(
                            ui,
                            mode,
                            "Warning: C10 capacitance may affect stability",
                            palette.warning,
                        );
                    });
                });
            });
    }
}
