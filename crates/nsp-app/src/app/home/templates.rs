//! Home workspace template cards.
//!
//! Provides the template grid on the home dashboard. Each template
//! represents a starting-point circuit (OpAmp, DC-DC, LDO, etc.).

use super::widgets::{section_header_clickable, template_card};
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use eframe::egui::{self, Vec2};

/// Template metadata for the home grid.
#[derive(Debug, Clone, Copy)]
pub(super) struct HomeTemplate {
    pub(super) name: UiText,
    pub(super) caption: &'static str,
}

/// Available home templates.
pub(super) fn home_templates() -> [HomeTemplate; 5] {
    [
        HomeTemplate {
            name: UiText::TemplateOpAmp,
            caption: "Single / Dual OpAmp",
        },
        HomeTemplate {
            name: UiText::TemplateDcDc,
            caption: "Buck / Boost",
        },
        HomeTemplate {
            name: UiText::TemplateLdo,
            caption: "Low Dropout",
        },
        HomeTemplate {
            name: UiText::TemplateDifferentialPair,
            caption: "Analog Front End",
        },
        HomeTemplate {
            name: UiText::TemplatePowerSupply,
            caption: "SMPS / Flyback",
        },
    ]
}

impl NekoSpiceApp {
    /// Draw template row with responsive grid layout.
    pub(crate) fn draw_template_row(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        if section_header_clickable(
            ui,
            mode,
            self.text(UiText::StartTemplate),
            self.text(UiText::ViewAll),
        ) {
            self.active_workspace = StudioWorkspace::Schematic;
        }
        ui.add_space(4.0);

        let spacing = 10.0;
        let available_width = ui.available_width();
        let columns: usize = if available_width >= 850.0 {
            5
        } else if available_width >= 620.0 {
            3
        } else if available_width >= 390.0 {
            2
        } else {
            1
        };
        let card_width = ((available_width - spacing * (columns.saturating_sub(1) as f32))
            / columns as f32)
            .max(150.0);

        egui::Grid::new("home_template_grid")
            .num_columns(columns)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                for (index, template) in home_templates().into_iter().enumerate() {
                    ui.allocate_ui_with_layout(
                        Vec2::new(card_width, 126.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            template_card(
                                ui,
                                mode,
                                self.text(template.name),
                                template.caption,
                                self.text(UiText::Use),
                            );
                        },
                    );
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
    }
}
