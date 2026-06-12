//! Left and center column sections of the simulation profile editor:
//! - Analysis setup panel (analysis type selection)
//! - Component parameters table
//! - Model parameters table
//! - Parameter definitions editor (center)

use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use super::state::AnalysisParams;
use super::profile_editor_widgets::{param_table, section_header};
use super::workspace_widgets::analysis_mode_button;
use crate::app::theme::StudioTheme;
use eframe::egui;
use osl_kicad::KicadSimulationDirectiveKind;

/// Draw the analysis setup panel (left column, top).
pub(crate) fn draw_analysis_setup_panel(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::AnalysisSetup));
        ui.add_space(4.0);
        // Two-column grid of analysis mode buttons
        for row in profile_analysis_modes().chunks(2) {
            ui.columns(2, |columns| {
                for (col, (kind, title, caption)) in row.iter().enumerate() {
                    let active = app.simulation_panel.directive_kind == *kind;
                    if analysis_mode_button(&mut columns[col], mode, title, caption, active) {
                        if app.simulation_panel.directive_kind != *kind {
                            app.simulation_panel.analysis_params = AnalysisParams::for_kind(*kind);
                        }
                        app.simulation_panel.directive_kind = *kind;
                    }
                }
            });
            ui.add_space(4.0);
        }
        ui.separator();
        // Show current directive preview
        let body = app.simulation_panel.analysis_params.to_body();
        let directive = format!("{} {}", app.simulation_panel.directive_kind, body.trim());
        ui.label(StudioTheme::muted_for(mode, "Current directive:"));
        ui.monospace(directive);
    });
}

/// Draw the component parameters table (left column, middle).
pub(crate) fn draw_component_params(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::ComponentParameters));
        ui.add_space(4.0);
        let count = app.simulation_profile_editor.component_params.len();
        ui.label(StudioTheme::muted_for(
            mode,
            format!("{count} component(s) defined"),
        ));
        ui.add_space(4.0);
        param_table(
            ui,
            mode,
            &mut app.simulation_profile_editor.component_params,
        );
        ui.add_space(4.0);
        if ui.small_button("+ Add Component").clicked() {
            app.simulation_profile_editor
                .component_params
                .push((String::new(), String::new(), String::new()));
        }
    });
}

/// Draw the model parameters table (left column, bottom).
pub(crate) fn draw_model_params(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::ModelParameters));
        ui.add_space(4.0);
        let count = app.simulation_profile_editor.model_params.len();
        ui.label(StudioTheme::muted_for(
            mode,
            format!("{count} model(s) defined"),
        ));
        ui.add_space(4.0);
        param_table(
            ui,
            mode,
            &mut app.simulation_profile_editor.model_params,
        );
        ui.add_space(4.0);
        if ui.small_button("+ Add Model").clicked() {
            app.simulation_profile_editor
                .model_params
                .push((String::new(), String::new(), String::new()));
        }
    });
}

/// Draw the parameter definitions editor (center column).
pub(crate) fn draw_parameter_definitions(app: &mut NekoSpiceApp, ui: &mut egui::Ui) {
    let mode = app.theme_mode();
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        section_header(ui, mode, app.text(UiText::ParameterDefinitions));
        ui.add_space(4.0);
        ui.label(StudioTheme::muted_for(
            mode,
            "Define simulation parameter names, values, and units.",
        ));
        ui.add_space(6.0);

        // Column headers
        egui::Grid::new("param_def_headers")
            .num_columns(3)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label(StudioTheme::muted_for(mode, "Name"));
                ui.label(StudioTheme::muted_for(mode, "Value"));
                ui.label(StudioTheme::muted_for(mode, "Unit"));
                ui.end_row();
            });

        // Editable parameter definition rows
        if app.simulation_profile_editor.component_params.is_empty()
            && app.simulation_profile_editor.model_params.is_empty()
        {
            ui.add_space(12.0);
            ui.label(StudioTheme::muted_for(
                mode,
                "Add component or model parameters to define them here.",
            ));
        } else {
            egui::ScrollArea::vertical()
                .id_salt("param_definitions_scroll")
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("param_definitions_grid")
                        .num_columns(3)
                        .spacing([8.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            for row in app
                                .simulation_profile_editor
                                .component_params
                                .iter_mut()
                            {
                                ui.text_edit_singleline(&mut row.0);
                                ui.text_edit_singleline(&mut row.1);
                                ui.text_edit_singleline(&mut row.2);
                                ui.end_row();
                            }
                            for row in
                                app.simulation_profile_editor.model_params.iter_mut()
                            {
                                ui.text_edit_singleline(&mut row.0);
                                ui.text_edit_singleline(&mut row.1);
                                ui.text_edit_singleline(&mut row.2);
                                ui.end_row();
                            }
                        });
                });
        }

        ui.add_space(6.0);
        // Quick template buttons
        ui.horizontal(|ui| {
            if ui.small_button("Load RC Template").clicked() {
                load_rc_template(&mut app.simulation_profile_editor.component_params);
            }
            if ui.small_button("Load Op-Amp Template").clicked() {
                load_opamp_template(&mut app.simulation_profile_editor.component_params);
            }
            if ui.small_button("Clear All").clicked() {
                app.simulation_profile_editor.component_params.clear();
                app.simulation_profile_editor.model_params.clear();
            }
        });
    });
}

/// Analysis modes available in the profile editor.
fn profile_analysis_modes() -> [(
    KicadSimulationDirectiveKind,
    &'static str,
    &'static str,
); 7] {
    [
        (
            KicadSimulationDirectiveKind::Tran,
            ".tran",
            "time domain",
        ),
        (
            KicadSimulationDirectiveKind::Ac,
            ".ac",
            "small signal",
        ),
        (
            KicadSimulationDirectiveKind::Dc,
            ".dc",
            "sweep",
        ),
        (
            KicadSimulationDirectiveKind::Op,
            ".op",
            "operating point",
        ),
        (
            KicadSimulationDirectiveKind::Noise,
            ".noise",
            "noise",
        ),
        (
            KicadSimulationDirectiveKind::Disto,
            ".disto",
            "distortion",
        ),
        (
            KicadSimulationDirectiveKind::Sens,
            ".sens",
            "sensitivity",
        ),
    ]
}

/// Pre-fill a basic RC low-pass filter template.
fn load_rc_template(params: &mut Vec<(String, String, String)>) {
    params.clear();
    params.push(("R1".into(), "10k".into(), "ohm".into()));
    params.push(("C1".into(), "100n".into(), "F".into()));
    params.push(("V1".into(), "1".into(), "V".into()));
    params.push(("Fcut".into(), "159".into(), "Hz".into()));
}

/// Pre-fill a basic op-amp template.
fn load_opamp_template(params: &mut Vec<(String, String, String)>) {
    params.clear();
    params.push(("Rf".into(), "100k".into(), "ohm".into()));
    params.push(("Rin".into(), "10k".into(), "ohm".into()));
    params.push(("R1".into(), "10k".into(), "ohm".into()));
    params.push(("R2".into(), "10k".into(), "ohm".into()));
    params.push(("C1".into(), "10p".into(), "F".into()));
    params.push(("Vcc".into(), "15".into(), "V".into()));
    params.push(("Vee".into(), "-15".into(), "V".into()));
}
