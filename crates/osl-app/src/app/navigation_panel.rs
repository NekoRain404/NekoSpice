use super::NekoSpiceApp;
use super::localization::UiText;
use super::navigation::StudioWorkspace;
use super::theme::StudioTheme;
use super::widgets::metric_row;
use eframe::egui::{self, RichText};

impl NekoSpiceApp {
    pub(super) fn draw_workspace_navigation(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let locale = self.locale();

        ui.heading(self.text(UiText::StudioTitle));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::StudioSubtitle),
        ));
        ui.add_space(12.0);

        for workspace in StudioWorkspace::ALL {
            let selected = self.active_workspace == workspace;
            let label = format!(
                "{}  {}",
                workspace.icon(),
                workspace.localized_label(locale)
            );
            let response = ui
                .add_sized(
                    [ui.available_width(), 34.0],
                    egui::Button::new(RichText::new(label).strong())
                        .fill(if selected {
                            palette.accent_soft
                        } else {
                            palette.panel_soft
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if selected {
                                palette.accent
                            } else {
                                palette.border
                            },
                        ))
                        .corner_radius(6),
                )
                .on_hover_text(workspace.localized_caption(locale));
            if response.clicked() {
                self.active_workspace = workspace;
            }
            ui.add_space(4.0);
        }

        ui.add_space(10.0);
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::System),
            ));
            metric_row(ui, mode, self.text(UiText::Renderer), "wgpu");
            metric_row(ui, mode, self.text(UiText::Solver), "ngspice");
            let dirty = self.text(UiText::Dirty);
            let clean = self.text(UiText::Clean);
            let missing = self.text(UiText::Missing);
            metric_row(
                ui,
                mode,
                self.text(UiText::Document),
                self.document
                    .as_ref()
                    .map(|document| if document.is_dirty() { dirty } else { clean })
                    .unwrap_or(missing),
            );
            ui.separator();
            metric_row(
                ui,
                mode,
                self.text(UiText::Theme),
                self.theme_mode_label(self.theme_mode()),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Language),
                self.locale().native_name(),
            );
        });
    }
}
