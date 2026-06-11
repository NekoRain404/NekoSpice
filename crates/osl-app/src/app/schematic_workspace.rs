// Schematic workspace: canvas, toolbar, document tabs, and bottom dock.
// The bottom dock switches between Waveforms, FFT, Bode, Console, Netlist,
// ERC, and Inspector views based on the active tab.

use super::NekoSpiceApp;
use super::SchematicBottomTab;
use super::localization::UiText;
use super::schematic_workspace_widgets::{
    bottom_console_line, canvas_toolbar_button, document_tab, signal_row,
};
use super::theme::StudioTheme;
use eframe::egui::{self, Vec2};

impl NekoSpiceApp {
    /// Main entry point for the schematic center workspace.
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

    /// Toolbar row: save, fit, run, drawing tools, zoom, DRC status.
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

    /// Document tab bar: shows loaded schematic and placeholder sub-sheets.
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

    /// Bottom dock panel with tab switching between views.
    fn draw_schematic_bottom_dock(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let current_tab = self.schematic_bottom_tab;

        egui::Frame::new()
            .fill(palette.panel_soft)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                // Tab bar: clicking a tab switches the active view
                ui.horizontal_wrapped(|ui| {
                    let tab_defs: &[(SchematicBottomTab, &str)] = &[
                        (SchematicBottomTab::Waveforms, self.text(UiText::Waveforms)),
                        (SchematicBottomTab::Fft, "FFT"),
                        (SchematicBottomTab::Bode, "Bode"),
                        (SchematicBottomTab::Console, self.text(UiText::StatusConsole)),
                        (SchematicBottomTab::Netlist, self.text(UiText::Netlist)),
                        (SchematicBottomTab::Erc, "ERC"),
                        (SchematicBottomTab::Inspector, self.text(UiText::Inspector)),
                    ];
                    for &(tab, label) in tab_defs {
                        let is_active = current_tab == tab;
                        let btn = egui::Button::new(if is_active {
                            StudioTheme::accent_for(mode, label)
                        } else {
                            StudioTheme::muted_for(mode, label)
                        })
                        .fill(if is_active {
                            palette.accent_soft
                        } else {
                            palette.panel
                        })
                        .stroke(egui::Stroke::new(1.0, palette.border))
                        .corner_radius(4);
                        if ui.add(btn).clicked() {
                            self.schematic_bottom_tab = tab;
                        }
                    }
                });
                ui.separator();

                // Content area based on active tab
                match self.schematic_bottom_tab {
                    SchematicBottomTab::Waveforms => {
                        self.draw_bottom_waveforms_tab(ui);
                    }
                    SchematicBottomTab::Fft => {
                        self.draw_bottom_fft_tab(ui);
                    }
                    SchematicBottomTab::Bode => {
                        self.draw_bottom_bode_tab(ui);
                    }
                    SchematicBottomTab::Console => {
                        self.draw_bottom_console_tab(ui);
                    }
                    SchematicBottomTab::Netlist => {
                        self.draw_bottom_netlist_tab(ui);
                    }
                    SchematicBottomTab::Erc => {
                        self.draw_bottom_erc_tab(ui);
                    }
                    SchematicBottomTab::Inspector => {
                        self.draw_bottom_inspector_tab(ui);
                    }
                }
            });
    }

    // -- Bottom dock tab content views --

    /// Waveforms tab: signal list from loaded schematic.
    fn draw_bottom_waveforms_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() * 0.30).max(180.0));
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
                    signal_row(ui, mode, "V(IN)", "1.000 V/div", palette.accent);
                    signal_row(ui, mode, "V(OUT)", "1.000 V/div", palette.success);
                }
            });
            ui.separator();
            ui.vertical(|ui| {
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
            });
        });
    }

    /// FFT tab: placeholder for future FFT visualization.
    fn draw_bottom_fft_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::muted_for(
            mode,
            "FFT analysis — run a transient simulation first",
        ));
    }

    /// Bode tab: placeholder for future Bode plot.
    fn draw_bottom_bode_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::muted_for(
            mode,
            "Bode plot — run an AC simulation first",
        ));
    }

    /// Console tab: simulation status, errors, and log output.
    fn draw_bottom_console_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
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
                    "Simulation completed: {} ({} ms)",
                    run.metadata.status.as_str(),
                    run.metadata.duration_ms
                ),
                palette.text_muted,
            );
        }
        if let Some(document) = &self.document {
            let dir_count = document.simulation_directives().len();
            bottom_console_line(
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

    /// Netlist tab: SPICE netlist preview from loaded schematic.
    fn draw_bottom_netlist_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        if let Some(document) = &self.document {
            match document.spice_netlist_preview() {
                Ok(netlist) => {
                    egui::ScrollArea::vertical()
                        .max_height(140.0)
                        .show(ui, |ui| {
                            ui.monospace(netlist);
                        });
                }
                Err(error) => {
                    let palette = self.theme_palette();
                    bottom_console_line(
                        ui,
                        mode,
                        &format!("Netlist generation failed: {error}"),
                        palette.danger,
                    );
                }
            }
        } else {
            ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
        }
    }

    /// ERC tab: Electrical Rules Check results.
    fn draw_bottom_erc_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        if let Some(document) = &self.document {
            let report = document.check_report();
            let error_count = report.error_count();
            let warning_count = report.warning_count();
            bottom_console_line(
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
                        use osl_kicad::KicadDiagnosticSeverity;
                        let (prefix, color) = match diag.severity {
                            KicadDiagnosticSeverity::Error => ("ERROR", palette.danger),
                            KicadDiagnosticSeverity::Warning => ("WARNING", palette.warning),
                            KicadDiagnosticSeverity::Info => ("INFO", palette.text_muted),
                        };
                        bottom_console_line(ui, mode, &format!("{prefix}: {}", diag.message), color);
                    }
                });
        } else {
            ui.label(StudioTheme::muted_for(mode, "No schematic loaded"));
        }
    }

    /// Inspector tab: selected item properties.
    fn draw_bottom_inspector_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        if let Some(hit) = &self.selected_hit {
            let palette = self.theme_palette();
            ui.label(StudioTheme::section_title_for(
                mode,
                format!("Selected: {}", hit.kind),
            ));
            if let Some(ref uuid) = hit.uuid {
                ui.label(StudioTheme::muted_for(mode, format!("UUID: {uuid}")));
            }
            bottom_console_line(
                ui,
                mode,
                &format!(
                    "Bounds: ({:.1}, {:.1}) — ({:.1}, {:.1})",
                    hit.bounds.min.x, hit.bounds.min.y, hit.bounds.max.x, hit.bounds.max.y
                ),
                palette.text_muted,
            );
        } else {
            ui.label(StudioTheme::muted_for(
                mode,
                "Click an item to inspect",
            ));
        }
    }
}
