//! Home project context — loaded schematic info, library status, and file paths.

use super::dashboard::SECTION_GAP;
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use crate::app::widgets::metric_row;
use eframe::egui;

impl NekoSpiceApp {
    /// draw home project context。
    pub(crate) fn draw_home_project_context(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let snapshot = self.studio_status_snapshot();

        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::ProjectOverview),
        ));
        ui.add_space(4.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            metric_row(ui, mode, self.text(UiText::Project), &snapshot.project_name);
            metric_row(
                ui,
                mode,
                self.text(UiText::Document),
                &snapshot.document_state,
            );
            metric_row(ui, mode, self.text(UiText::Solver), &snapshot.solver_status);
            metric_row(
                ui,
                mode,
                self.text(UiText::Workspace),
                &snapshot.waveform_status,
            );
        });

        ui.add_space(SECTION_GAP);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::SchematicHealth),
            ));
            if let Some(scene) = &self.scene {
                metric_row(
                    ui,
                    mode,
                    self.text(UiText::Symbols),
                    &scene.symbols.len().to_string(),
                );
                metric_row(
                    ui,
                    mode,
                    self.text(UiText::Wires),
                    &scene.wires.len().to_string(),
                );
                metric_row(
                    ui,
                    mode,
                    self.text(UiText::Labels),
                    &scene.labels.len().to_string(),
                );
                metric_row(
                    ui,
                    mode,
                    self.text(UiText::Sheets),
                    &scene.sheets.len().to_string(),
                );
            } else {
                ui.label(StudioTheme::muted_for(
                    mode,
                    self.text(UiText::NoSchematicLoaded),
                ));
            }
        });

        ui.add_space(SECTION_GAP);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::LibraryScope),
            ));
            if let Some(library) = &self.library {
                metric_row(
                    ui,
                    mode,
                    self.text(UiText::Symbols),
                    &library.index().symbols.len().to_string(),
                );
                metric_row(
                    ui,
                    mode,
                    self.text(UiText::Project),
                    &library.index().libraries.len().to_string(),
                );
                metric_row(
                    ui,
                    mode,
                    self.text(UiText::Diagnostics),
                    &library.index().diagnostics.len().to_string(),
                );
            } else {
                ui.label(StudioTheme::muted_for(mode, self.text(UiText::Missing)));
            }
        });
    }
}
