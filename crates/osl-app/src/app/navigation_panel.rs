use super::NekoSpiceApp;
use super::navigation::StudioWorkspace;
use super::theme::StudioTheme;
use super::widgets::metric_row;
use eframe::egui::{self, RichText};

impl NekoSpiceApp {
    pub(super) fn draw_workspace_navigation(&mut self, ui: &mut egui::Ui) {
        ui.heading("NekoSpice");
        ui.label(StudioTheme::muted(
            "Rust-native KiCad schematic and ngspice studio",
        ));
        ui.add_space(12.0);

        for workspace in StudioWorkspace::ALL {
            let selected = self.active_workspace == workspace;
            let label = format!("{}  {}", workspace.icon(), workspace.label());
            let response = ui
                .add_sized(
                    [ui.available_width(), 34.0],
                    egui::Button::new(RichText::new(label).strong())
                        .fill(if selected {
                            StudioTheme::ACCENT_SOFT
                        } else {
                            StudioTheme::PANEL_SOFT
                        })
                        .stroke(egui::Stroke::new(
                            1.0,
                            if selected {
                                StudioTheme::ACCENT
                            } else {
                                StudioTheme::BORDER
                            },
                        ))
                        .corner_radius(6),
                )
                .on_hover_text(workspace.caption());
            if response.clicked() {
                self.active_workspace = workspace;
            }
            ui.add_space(4.0);
        }

        ui.add_space(10.0);
        StudioTheme::panel_frame().show(ui, |ui| {
            ui.label(StudioTheme::section_title("System"));
            metric_row(ui, "Renderer", "wgpu");
            metric_row(ui, "Solver", "ngspice");
            metric_row(
                ui,
                "Document",
                self.document
                    .as_ref()
                    .map(|document| {
                        if document.is_dirty() {
                            "dirty"
                        } else {
                            "clean"
                        }
                    })
                    .unwrap_or("missing"),
            );
        });
    }
}
