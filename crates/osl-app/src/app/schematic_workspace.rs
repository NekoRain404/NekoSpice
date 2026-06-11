/// Schematic workspace: canvas, toolbar, document tabs, and bottom dock.
///
/// The bottom dock switches between Waveforms, FFT, Bode, Console, Netlist,
/// ERC, and Inspector views based on the active tab.
use super::NekoSpiceApp;
use super::SchematicBottomTab;
use super::localization::UiText;
use super::schematic_workspace_widgets::{
    bottom_console_line, canvas_toolbar_button, document_tab, toolbar_icon_button,
};
use super::theme::StudioTheme;
use eframe::egui::{self, CornerRadius, Stroke, Vec2};

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
            let inspector_width = 280.0;
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), canvas_height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    // Vertical tool palette on the left
                    let _palette_width = self.draw_tool_palette(ui);
                    // Main canvas area (occupies remaining width minus inspector)
                    let canvas_width = (ui.available_width() - inspector_width - 8.0).max(200.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(canvas_width, canvas_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.draw_canvas(ui),
                    );
                    // Right-side inspector panel
                    ui.add_space(4.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(inspector_width, canvas_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(canvas_height)
                                .show(ui, |ui| {
                                    self.draw_schematic_inspector_panel(ui);
                                });
                        },
                    );
                },
            );
            ui.add_space(6.0);
            self.draw_schematic_bottom_dock(ui);
        });
    }

    /// Toolbar row: file ops, drawing tools, zoom, DRC status.
    fn draw_schematic_workspace_toolbar(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();

        ui.horizontal(|ui| {
            // File operations
            canvas_toolbar_button(ui, mode, "Save", self.document.is_some());
            ui.add_space(2.0);
            if canvas_toolbar_button(ui, mode, "Fit", true).clicked() {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }
            if canvas_toolbar_button(ui, mode, "Run", self.document.is_some())
                .clicked()
            {
                self.run_simulation_from_panel();
            }

            ui.separator();

            // Drawing tools
            toolbar_icon_button(ui, mode, "\u{250C}", "Wire Tool", true);
            toolbar_icon_button(ui, mode, "\u{2190}", "Net Label", true);
            toolbar_icon_button(ui, mode, "\u{2550}", "Bus Tool", true);
            toolbar_icon_button(ui, mode, "\u{25A3}", "Sheet Symbol", true);

            ui.separator();

            // Zoom display
            ui.label(StudioTheme::muted_for(mode, "Zoom"));
            ui.label(StudioTheme::accent_for(
                mode,
                format!("{:.0}%", self.viewport.zoom * 10.0),
            ));

            ui.separator();

            // DRC status
            ui.label(StudioTheme::muted_for(mode, "DRC"));
            let report = self
                .document
                .as_ref()
                .map(|doc| doc.check_report());
            let (dot_color, drc_text) = match report {
                Some(r) if r.error_count() > 0 => (palette.danger, format!("{} errors", r.error_count())),
                Some(r) if r.warning_count() > 0 => (palette.warning, format!("{} warnings", r.warning_count())),
                Some(_) => (palette.success, "Clean".to_string()),
                None => (palette.text_muted, "No doc".to_string()),
            };
            ui.label(
                egui::RichText::new(format!("\u{25CF} {drc_text}"))
                    .color(dot_color)
                    .size(12.0),
            );
        });
    }

    /// Document tab bar: shows loaded schematic and placeholder sub-sheets.
    fn draw_schematic_document_tabs(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_wrapped(|ui| {
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
        let palette = self.theme_palette();
        let current_tab = self.schematic_bottom_tab;

        egui::Frame::new()
            .fill(palette.panel_soft)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                // Tab bar
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
                        let fill = if is_active {
                            palette.accent_soft
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let text_color = if is_active {
                            palette.accent
                        } else {
                            palette.text_muted
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(label)
                                        .size(12.0)
                                        .color(text_color),
                                )
                                .fill(fill)
                                .stroke(if is_active {
                                    Stroke::new(1.0, palette.accent)
                                } else {
                                    Stroke::NONE
                                })
                                .corner_radius(CornerRadius::same(4)),
                            )
                            .clicked()
                        {
                            self.schematic_bottom_tab = tab;
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                match current_tab {
                    SchematicBottomTab::Waveforms => self.draw_bottom_waveforms_tab(ui),
                    SchematicBottomTab::Fft => self.draw_bottom_fft_tab(ui),
                    SchematicBottomTab::Bode => self.draw_bottom_bode_tab(ui),
                    SchematicBottomTab::Console => self.draw_bottom_console_tab(ui),
                    SchematicBottomTab::Netlist => self.draw_bottom_netlist_tab(ui),
                    SchematicBottomTab::Erc => self.draw_bottom_erc_tab(ui),
                    SchematicBottomTab::Inspector => self.draw_bottom_inspector_tab(ui),
                }
            });
    }

    /// Waveforms tab: signal list and scale overview.
    fn draw_bottom_waveforms_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        if let Some(run) = &self.simulation_panel.last_run {
            match &run.waveform {
                crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
                    for variable in &summary.variables {
                        super::schematic_workspace_widgets::signal_row(
                            ui,
                            mode,
                            &variable.name,
                            &variable.unit,
                            palette.accent,
                        );
                    }
                }
                crate::waveform_summary::GuiWaveformSummaryState::Missing { .. } => {
                    ui.label(StudioTheme::muted_for(mode, "No waveform data loaded"));
                }
                crate::waveform_summary::GuiWaveformSummaryState::Error { message, .. } => {
                    bottom_console_line(
                        ui,
                        mode,
                        &format!("Waveform error: {message}"),
                        palette.danger,
                    );
                }
            }
        } else {
            ui.label(StudioTheme::muted_for(mode, "Run a simulation to view waveforms"));
        }
    }

    /// FFT tab placeholder.
    fn draw_bottom_fft_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::muted_for(mode, "FFT analysis \u{2014} run a simulation first"));
    }

    /// Bode plot tab placeholder.
    fn draw_bottom_bode_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::muted_for(mode, "Bode plot \u{2014} run an AC analysis first"));
    }

    /// Console output tab.
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
                    "Bounds: ({:.1}, {:.1}) \u{2014} ({:.1}, {:.1})",
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
