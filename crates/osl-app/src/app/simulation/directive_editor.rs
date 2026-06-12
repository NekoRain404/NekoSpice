//! Simulation directive editor — the UI for editing .tran/.ac/.dc/.op
//! directive kind and body text, with a "Set Directive" button that
//! writes the edited value back to the schematic.

use crate::app::NekoSpiceApp;
use eframe::egui;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use osl_kicad::KicadSimulationDirectiveKind;

/// Auto-default directive body when the user switches analysis type.
pub(crate) fn default_directive_body_for_kind(kind: KicadSimulationDirectiveKind) -> String {
    match kind {
        KicadSimulationDirectiveKind::Tran => "1u 1m".to_string(),
        KicadSimulationDirectiveKind::Ac => "dec 10 1 1Meg".to_string(),
        KicadSimulationDirectiveKind::Dc => "V1 0 5 0.1".to_string(),
        KicadSimulationDirectiveKind::Op => String::new(),
        _ => String::new(),
    }
}

impl NekoSpiceApp {
    /// Draw the directive editor: kind selector buttons + body text field + Set Directive.
    pub(in crate::app) fn draw_simulation_directive_editor(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(mode, self.text(UiText::SimulationWorkspace)));
        ui.horizontal(|ui| {
            ui.label(self.text(UiText::Kind));
            for kind in [
                KicadSimulationDirectiveKind::Tran,
                KicadSimulationDirectiveKind::Ac,
                KicadSimulationDirectiveKind::Dc,
                KicadSimulationDirectiveKind::Op,
            ] {
                let label = kind.to_string();
                let active = self.simulation_panel.directive_kind == kind;
                let btn = if active {
                    egui::Button::new(egui::RichText::new(&label).strong())
                        .fill(self.theme_palette().accent_soft)
                } else {
                    egui::Button::new(&label)
                };
                if ui.add(btn).clicked() {
                    if self.simulation_panel.directive_kind != kind {
                        self.simulation_panel.directive_body =
                            default_directive_body_for_kind(kind);
                    }
                    self.simulation_panel.directive_kind = kind;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(self.text(UiText::Body));
            ui.text_edit_singleline(&mut self.simulation_panel.directive_body);
        });
        if ui
            .add_enabled(
                self.document.is_some(),
                egui::Button::new(self.text(UiText::SetDirective)),
            )
            .clicked()
        {
            self.apply_simulation_directive_edit();
        }
    }

    /// Apply the current directive editor state to the loaded document.
    /// Pushes an undo snapshot before modifying the schematic.
    pub(in crate::app) fn apply_simulation_directive_edit(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };
        // Snapshot before edit for undo support
        self.history.push(document.snapshot());
        let kind = self.simulation_panel.directive_kind;
        let body = self.simulation_panel.directive_body.clone();
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
