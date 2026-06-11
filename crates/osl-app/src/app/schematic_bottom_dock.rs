/// Bottom dock tabs for the schematic workspace.
///
/// Provides Waveforms, FFT, Bode, Console, Netlist, ERC, and Inspector views.
/// Each tab renders its content based on the current simulation state and loaded document.
use super::NekoSpiceApp;
use super::schematic_workspace_widgets::bottom_console_line;
use super::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    /// Waveforms tab: signal list and scale overview.
    /// Waveform tab: signal list + stacked waveform preview.
    pub(super) fn draw_bottom_waveforms_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        if let Some(run) = &self.simulation_panel.last_run {
            match &run.waveform {
                crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
                    // Signal list header
                    ui.label(StudioTheme::section_title_for(
                        mode,
                        format!("Signals ({})", summary.variable_count),
                    ));
                    ui.add_space(2.0);

                    // Stacked waveform preview (actual chart)
                    let preview_height = 100.0;
                    super::waveform_preview::draw_stacked_waveform_preview(
                        ui,
                        mode,
                        summary,
                        None,
                        preview_height,
                    );
                    ui.add_space(4.0);

                    // Variable summary table
                    for variable in &summary.variables {
                        super::schematic_workspace_widgets::signal_row(
                            ui,
                            mode,
                            &variable.name,
                            &format!(
                                "{}: min={:.3} max={:.3}",
                                variable.unit, variable.min, variable.max,
                            ),
                            palette.accent,
                        );
                    }
                    if summary.omitted_variable_count > 0 {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            format!("+{} more variables", summary.omitted_variable_count),
                        ));
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

    /// FFT tab: frequency-domain analysis preview.
    pub(super) fn draw_bottom_fft_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        if let Some(run) = &self.simulation_panel.last_run {
            match &run.waveform {
                crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
                    ui.label(StudioTheme::section_title_for(
                        mode,
                        format!("FFT ({})", summary.plot_name),
                    ));
                    ui.add_space(2.0);
                    // Show frequency variables if available
                    let freq_vars: Vec<_> = summary.variables.iter()
                        .filter(|v| v.name.to_lowercase().contains("freq") || v.unit.to_lowercase().contains("hz"))
                        .collect();
                    if freq_vars.is_empty() {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            "No frequency-domain signals detected in this simulation",
                        ));
                    } else {
                        for variable in freq_vars {
                            super::schematic_workspace_widgets::signal_row(
                                ui,
                                mode,
                                &variable.name,
                                &variable.unit,
                                palette.warning,
                            );
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(StudioTheme::muted_for(
                        mode,
                        "FFT requires transient data with uniform time steps",
                    ));
                }
                _ => {
                    ui.label(StudioTheme::muted_for(mode, "Run a transient simulation for FFT analysis"));
                }
            }
        } else {
            ui.label(StudioTheme::muted_for(mode, "Run a simulation for FFT analysis"));
        }
    }

    /// Bode plot tab: AC analysis magnitude/phase display.
    pub(super) fn draw_bottom_bode_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        if let Some(run) = &self.simulation_panel.last_run {
            match &run.waveform {
                crate::waveform_summary::GuiWaveformSummaryState::Ready(summary) => {
                    ui.label(StudioTheme::section_title_for(
                        mode,
                        format!("Bode ({})", summary.plot_name),
                    ));
                    ui.add_space(2.0);
                    // Check if this is an AC analysis
                    let is_ac = summary.plot_name.to_lowercase().contains("ac")
                        || summary.title.to_lowercase().contains("ac");
                    if is_ac {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            "AC analysis detected — magnitude and phase plots available",
                        ));
                    } else {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            "Bode plot requires AC analysis data",
                        ));
                    }
                    ui.add_space(4.0);
                    // Show available variables
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
                _ => {
                    ui.label(StudioTheme::muted_for(mode, "Run an AC analysis for Bode plot"));
                }
            }
        } else {
            ui.label(StudioTheme::muted_for(mode, "Run an AC analysis for Bode plot"));
        }
    }

    /// Console output tab.
    pub(super) fn draw_bottom_console_tab(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn draw_bottom_netlist_tab(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn draw_bottom_erc_tab(&mut self, ui: &mut egui::Ui) {
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
    pub(super) fn draw_bottom_inspector_tab(&mut self, ui: &mut egui::Ui) {
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
