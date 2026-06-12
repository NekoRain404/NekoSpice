/// Bottom dock tabs for the schematic workspace.
///
/// Provides Waveforms, FFT, Bode, Console, Netlist, ERC, and Inspector views.
/// Each tab renders its content based on the current simulation state and loaded document.
use crate::app::NekoSpiceApp;
use super::workspace_widgets::bottom_console_line;
use crate::app::theme::StudioTheme;
use eframe::egui::{self, Pos2, Stroke, Vec2};

impl NekoSpiceApp {
    /// Waveforms tab: signal list and scale overview.
    /// Waveform tab: signal list + stacked waveform preview.
    pub(crate) fn draw_bottom_waveforms_tab(&mut self, ui: &mut egui::Ui) {
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
                    crate::app::waveform::preview::draw_stacked_waveform_preview(
                        ui,
                        mode,
                        summary,
                        None,
                        preview_height,
                    );
                    ui.add_space(4.0);

                    // Variable summary table
                    for variable in &summary.variables {
                        super::workspace_widgets::signal_row(
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
    ///
    /// Shows signal variables that could be analyzed in the frequency domain,
    /// along with a mini frequency-axis preview chart.
    pub(crate) fn draw_bottom_fft_tab(&mut self, ui: &mut egui::Ui) {
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

                    // Signal list for frequency-domain analysis
                    let freq_vars: Vec<_> = summary.variables.iter()
                        .filter(|v| {
                            let name = v.name.to_lowercase();
                            name.starts_with("v(") || name.starts_with("i(")
                                || name.contains("freq") || v.unit.to_lowercase().contains("hz")
                        })
                        .collect();
                    if freq_vars.is_empty() {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            "Select voltage/current signals for FFT analysis",
                        ));
                    } else {
                        for variable in freq_vars.iter().take(4) {
                            super::workspace_widgets::signal_row(
                                ui,
                                mode,
                                &variable.name,
                                &format!("{}: {} pts", variable.unit, summary.point_count),
                                palette.warning,
                            );
                        }
                    }

                    // Mini frequency-domain chart placeholder
                    ui.add_space(4.0);
                    let chart_height = 50.0;
                    let desired_size = Vec2::new(ui.available_width().max(120.0), chart_height);
                    let (rect, _) = ui.allocate_exact_size(desired_size, eframe::egui::Sense::hover());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 2.0, crate::app::waveform::preview_primitives::plot_fill(mode));
                    painter.rect_stroke(
                        rect,
                        2.0,
                        eframe::egui::Stroke::new(1.0, palette.border),
                        eframe::egui::StrokeKind::Inside,
                    );
                    // Draw frequency axis labels
                    let plot_rect = rect.shrink(6.0);
                    painter.text(
                        plot_rect.left_bottom() + Vec2::new(4.0, -2.0),
                        egui::Align2::LEFT_BOTTOM,
                        "0 Hz",
                        egui::FontId::monospace(9.0),
                        palette.text_muted,
                    );
                    painter.text(
                        plot_rect.right_bottom() + Vec2::new(-4.0, -2.0),
                        egui::Align2::RIGHT_BOTTOM,
                        "Fs/2",
                        egui::FontId::monospace(9.0),
                        palette.text_muted,
                    );
                    // Placeholder message
                    painter.text(
                        plot_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "FFT visualization (run transient sim)",
                        egui::FontId::proportional(10.0),
                        palette.text_muted,
                    );
                }
                _ => {
                    ui.label(StudioTheme::muted_for(mode, "Run a transient simulation for FFT analysis"));
                }
            }
        } else {
            ui.label(StudioTheme::muted_for(mode, "Run a simulation to enable FFT"));
        }
    }

    /// Bode plot tab: AC analysis magnitude/phase display.
    ///
    /// Shows magnitude (dB) and phase (deg) when AC analysis data is available,
    /// along with a mini dual-axis chart placeholder.
    pub(crate) fn draw_bottom_bode_tab(&mut self, ui: &mut egui::Ui) {
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

                    // Check analysis type
                    let is_ac = summary.plot_name.to_lowercase().contains("ac")
                        || summary.title.to_lowercase().contains("ac");

                    if is_ac {
                        // Magnitude plot header
                        ui.label(StudioTheme::muted_for(mode, "|H(f)| [dB]"));
                        let mag_height = 35.0;
                        let desired = Vec2::new(ui.available_width().max(120.0), mag_height);
                        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 2.0, crate::app::waveform::preview_primitives::plot_fill(mode));
                        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);
                        let plot_rect = rect.shrink(4.0);
                        // Draw placeholder magnitude curve
                        let points: Vec<Pos2> = (0..40).map(|i| {
                            let x = plot_rect.left() + (i as f32 / 39.0) * plot_rect.width();
                            let y_norm = (-(i as f32 / 39.0 - 0.3).powi(2) * 3.0 + 1.0).clamp(0.1, 0.9);
                            Pos2::new(x, plot_rect.top() + y_norm * plot_rect.height())
                        }).collect();
                        for w in points.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(1.5, palette.accent));
                        }

                        ui.add_space(4.0);
                        // Phase plot header
                        ui.label(StudioTheme::muted_for(mode, "arg(H(f)) [deg]"));
                        let phase_height = 35.0;
                        let desired = Vec2::new(ui.available_width().max(120.0), phase_height);
                        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 2.0, crate::app::waveform::preview_primitives::plot_fill(mode));
                        painter.rect_stroke(rect, 2.0, Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);
                        let plot_rect = rect.shrink(4.0);
                        // Draw placeholder phase curve (0 to -180 deg)
                        let points: Vec<Pos2> = (0..40).map(|i| {
                            let x = plot_rect.left() + (i as f32 / 39.0) * plot_rect.width();
                            let t = i as f32 / 39.0;
                            let y_norm = (t * 0.8 + 0.1).clamp(0.05, 0.95);
                            Pos2::new(x, plot_rect.top() + y_norm * plot_rect.height())
                        }).collect();
                        for w in points.windows(2) {
                            painter.line_segment([w[0], w[1]], Stroke::new(1.5, palette.warning));
                        }
                    } else {
                        ui.label(StudioTheme::muted_for(
                            mode,
                            "Bode plot requires AC analysis -- run .ac simulation",
                        ));
                    }

                    ui.add_space(4.0);
                    for variable in &summary.variables {
                        super::workspace_widgets::signal_row(
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
            ui.label(StudioTheme::muted_for(mode, "Run a simulation to enable Bode plot"));
        }
    }

    /// Console tab: status messages, errors, and simulation output.
    pub(crate) fn draw_bottom_console_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        if let Some(msg) = &self.status_message {
            bottom_console_line(ui, mode, msg, palette.success);
        }
        if let Some(error) = &self.simulation_panel.last_error {
            bottom_console_line(ui, mode, error, palette.danger);
        }
        // Show full ngspice/xyce log in console tab
        if let Some(run) = &self.simulation_panel.last_run {
            let log_path = run.output_dir.join("ngspice.log");
            let fallback = run.output_dir.join("xyce.log");
            let actual = if log_path.is_file() { log_path } else { fallback };
            if actual.is_file() {
                if let Ok(content) = std::fs::read_to_string(&actual) {
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| {
                            ui.monospace(&content);
                        });
                }
            }
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
    pub(crate) fn draw_bottom_netlist_tab(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        if let Some(document) = &self.document {
            let profile = self.build_simulation_profile();
            let result = document.spice_netlist_preview().map(|netlist| {
                osl_sim::inject_profile_directives(&netlist, &profile)
            });
            match result {
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
    pub(crate) fn draw_bottom_erc_tab(&mut self, ui: &mut egui::Ui) {
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
    pub(crate) fn draw_bottom_inspector_tab(&mut self, ui: &mut egui::Ui) {
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
