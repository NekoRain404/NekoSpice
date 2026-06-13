//! Simulation directive editor — structured UI for editing analysis parameters.
//!
//! Each analysis type shows its own set of labeled fields:
//! - `.tran`: tstep, tstop, tstart, tmax, UIC checkbox
//! - `.ac`: sweep type (dec/lin/oct), npoints, fstart, fstop
//! - `.dc`: source name, vstart, vstop, vincr
//! - `.op`: no parameters
//!
//! The structured fields are converted to SPICE directive body text when
//! the user clicks "Set Directive" or runs the simulation.

use super::field_validation::{FieldValidity, validate_spice_value};
use super::profile_editor_widgets::labeled_edit;
use super::state::AnalysisParams;
use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;
use nsp_schema::NspSimulationDirectiveKind;

impl NekoSpiceApp {
    /// Draw the structured directive editor in the panel sidebar.
    ///
    /// Shows analysis type buttons and type-specific parameter fields.
    pub(in crate::app) fn draw_simulation_directive_editor(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(mode, "Analysis Type"));
            ui.add_space(4.0);

            // Analysis type selector buttons
            ui.horizontal_wrapped(|ui| {
                for kind in [
                    NspSimulationDirectiveKind::Tran,
                    NspSimulationDirectiveKind::Ac,
                    NspSimulationDirectiveKind::Dc,
                    NspSimulationDirectiveKind::Op,
                    NspSimulationDirectiveKind::Noise,
                    NspSimulationDirectiveKind::Disto,
                    NspSimulationDirectiveKind::Sens,
                ] {
                    let label = kind.to_string();
                    let active = self.simulation_panel.directive_kind == kind;
                    let btn = if active {
                        egui::Button::new(egui::RichText::new(&label).strong().color(palette.text))
                            .fill(palette.accent_soft)
                            .stroke(egui::Stroke::new(1.0, palette.accent))
                    } else {
                        egui::Button::new(egui::RichText::new(&label).color(palette.text_muted))
                            .fill(palette.panel_soft)
                            .stroke(egui::Stroke::new(1.0, palette.border))
                    };
                    if ui.add(btn).clicked() && self.simulation_panel.directive_kind != kind {
                        self.simulation_panel.directive_kind = kind;
                        self.simulation_panel.analysis_params = AnalysisParams::for_kind(kind);
                    }
                }
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Structured parameter fields for the selected analysis type
            self.draw_analysis_params_fields(ui);

            // Action buttons: Set Directive + Reset to Defaults
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.document.is_some(),
                        egui::Button::new("Set Directive").fill(palette.accent_soft),
                    )
                    .clicked()
                {
                    self.apply_simulation_directive_edit();
                }
                if ui.button("Reset Defaults").clicked() {
                    self.simulation_panel.analysis_params =
                        AnalysisParams::for_kind(self.simulation_panel.directive_kind);
                }
            });
        });
    }

    /// Draw type-specific parameter fields based on the current analysis kind.
    fn draw_analysis_params_fields(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();

        match &mut self.simulation_panel.analysis_params {
            AnalysisParams::Tran {
                tstep,
                tstop,
                tstart,
                tmax,
                uic,
            } => {
                egui::Grid::new("tran_params_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        labeled_edit(ui, mode, "Tstep", tstep, "1u")
                            .on_hover_text("Printing/plotting increment");
                        labeled_edit(ui, mode, "Tstop", tstop, "1m")
                            .on_hover_text("Simulation stop time");
                        labeled_edit(ui, mode, "Tstart", tstart, "0")
                            .on_hover_text("Start time for plotting (0 = beginning)");
                        labeled_edit(ui, mode, "Tmax", tmax, "auto")
                            .on_hover_text("Maximum internal timestep (0 = auto)");
                        // Validation feedback for key fields
                        ui.label(StudioTheme::muted_for(mode, "UIC"));
                        ui.checkbox(uic, "Use Initial Conditions")
                    });
                // Validation indicators
                ui.add_space(4.0);
                let tstep_valid = validate_spice_value(tstep);
                let tstop_valid = validate_spice_value(tstop);
                if tstep_valid != FieldValidity::Ok || tstop_valid != FieldValidity::Ok {
                    ui.horizontal(|ui| {
                        if tstep_valid != FieldValidity::Ok {
                            ui.colored_label(
                                tstep_valid.color(&palette),
                                format!("Tstep: {}", tstep_valid.tooltip()),
                            );
                        }
                        if tstop_valid != FieldValidity::Ok {
                            ui.colored_label(
                                tstop_valid.color(&palette),
                                format!("Tstop: {}", tstop_valid.tooltip()),
                            );
                        }
                    });
                }
                egui::Grid::new("tran_params_grid_extra")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(StudioTheme::muted_for(mode, "UIC"));
                        ui.checkbox(uic, "Use Initial Conditions")
                            .on_hover_text("Skip initial operating point calculation");
                        ui.end_row();
                    });
            }
            AnalysisParams::Ac {
                sweep_type,
                npoints,
                fstart,
                fstop,
            } => {
                // Sweep type selector
                ui.label(StudioTheme::muted_for(mode, "Sweep Type"));
                ui.horizontal(|ui| {
                    for (st, tip) in [
                        ("dec", "Points per decade"),
                        ("lin", "Total linear points"),
                        ("oct", "Points per octave"),
                    ] {
                        let active = sweep_type.as_str() == st;
                        let btn = if active {
                            egui::Button::new(egui::RichText::new(st).strong())
                                .fill(palette.accent_soft)
                        } else {
                            egui::Button::new(st)
                        };
                        if ui.add(btn).on_hover_text(tip).clicked() {
                            *sweep_type = st.to_string();
                        }
                    }
                });
                ui.add_space(4.0);
                egui::Grid::new("ac_params_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        labeled_edit(ui, mode, "Points", npoints, "10")
                            .on_hover_text("Number of points per decade/octave or total");
                        labeled_edit(ui, mode, "Fstart", fstart, "1")
                            .on_hover_text("Start frequency (Hz)");
                        labeled_edit(ui, mode, "Fstop", fstop, "1Meg")
                            .on_hover_text("Stop frequency (Hz)");
                    });
            }
            AnalysisParams::Dc {
                source,
                vstart,
                vstop,
                vincr,
            } => {
                egui::Grid::new("dc_params_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        labeled_edit(ui, mode, "Source", source, "V1")
                            .on_hover_text("Independent source to sweep");
                        labeled_edit(ui, mode, "Vstart", vstart, "0").on_hover_text("Start value");
                        labeled_edit(ui, mode, "Vstop", vstop, "5").on_hover_text("Stop value");
                        labeled_edit(ui, mode, "Vincr", vincr, "0.1")
                            .on_hover_text("Increment step");
                    });
            }
            AnalysisParams::Op => {
                ui.label(StudioTheme::muted_for(
                    mode,
                    "Operating point analysis — calculates DC bias conditions.",
                ));
            }
            AnalysisParams::Disto {
                fstart,
                fstop,
                fstep,
                maxharmonic,
            } => {
                egui::Grid::new("disto_params_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        labeled_edit(ui, mode, "Fstart", fstart, "1")
                            .on_hover_text("Start frequency (Hz)");
                        labeled_edit(ui, mode, "Fstop", fstop, "100k")
                            .on_hover_text("Stop frequency (Hz)");
                        labeled_edit(ui, mode, "Fstep", fstep, "auto")
                            .on_hover_text("Frequency step (0 = automatic)");
                        labeled_edit(ui, mode, "MaxHarm", maxharmonic, "3")
                            .on_hover_text("Maximum harmonic order");
                    });
            }
            AnalysisParams::Sens { output } => {
                egui::Grid::new("sens_params_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        labeled_edit(ui, mode, "Output", output, "V(out)")
                            .on_hover_text("Output variable for sensitivity analysis");
                    });
            }
            AnalysisParams::Noise {
                output,
                input_source,
                sweep_type,
                npoints,
                fstart,
                fstop,
            } => {
                egui::Grid::new("noise_params_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        labeled_edit(ui, mode, "Output", output, "V(out)")
                            .on_hover_text("Noise output variable");
                        labeled_edit(ui, mode, "Input Source", input_source, "V(src)")
                            .on_hover_text("Input noise source");
                    });
                // Sweep type selector
                ui.label(StudioTheme::muted_for(mode, "Sweep Type"));
                ui.horizontal(|ui| {
                    for (st, tip) in [
                        ("dec", "Points per decade"),
                        ("lin", "Total linear points"),
                        ("oct", "Points per octave"),
                    ] {
                        let active = sweep_type.as_str() == st;
                        let btn = if active {
                            egui::Button::new(egui::RichText::new(st).strong())
                                .fill(palette.accent_soft)
                        } else {
                            egui::Button::new(st)
                        };
                        if ui.add(btn).on_hover_text(tip).clicked() {
                            *sweep_type = st.to_string();
                        }
                    }
                });
                ui.add_space(4.0);
                egui::Grid::new("noise_sweep_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        labeled_edit(ui, mode, "Points", npoints, "10")
                            .on_hover_text("Number of points per decade/octave or total");
                        labeled_edit(ui, mode, "Fstart", fstart, "1")
                            .on_hover_text("Start frequency (Hz)");
                        labeled_edit(ui, mode, "Fstop", fstop, "100Meg")
                            .on_hover_text("Stop frequency (Hz)");
                    });
                // AC validation
                let fstart_valid = validate_spice_value(fstart);
                let fstop_valid = validate_spice_value(fstop);
                if fstart_valid != FieldValidity::Ok || fstop_valid != FieldValidity::Ok {
                    ui.horizontal(|ui| {
                        if fstart_valid != FieldValidity::Ok {
                            ui.colored_label(
                                fstart_valid.color(&palette),
                                format!("Fstart: {}", fstart_valid.tooltip()),
                            );
                        }
                        if fstop_valid != FieldValidity::Ok {
                            ui.colored_label(
                                fstop_valid.color(&palette),
                                format!("Fstop: {}", fstop_valid.tooltip()),
                            );
                        }
                    });
                }
            }
        }

        // Draw analysis-specific range presets after grids to avoid borrow conflicts
        match self.simulation_panel.directive_kind {
            nsp_schema::NspSimulationDirectiveKind::Tran => self.draw_tran_range_presets(ui, mode),
            nsp_schema::NspSimulationDirectiveKind::Ac => self.draw_ac_range_presets(ui, mode),
            nsp_schema::NspSimulationDirectiveKind::Dc => self.draw_dc_range_presets(ui, mode),
            _ => {}
        }
    }

    /// Apply the current structured directive to the loaded document.
    pub(in crate::app) fn apply_simulation_directive_edit(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        self.history.push(document.snapshot());
        let kind = self.simulation_panel.directive_kind;
        let body = self.simulation_panel.analysis_params.to_body();
        match document.set_simulation_directive(kind, body, None) {
            Ok(summary) => {
                self.scene = Some(document.scene());
                self.load_error = None;
                self.history.clear_redo();
                self.status_message =
                    Some(format!("Edited {} {}", summary.operation, summary.target));
            }
            Err(error) => {
                self.status_message = Some(error);
            }
        }
    }
}
