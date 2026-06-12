use crate::app::NekoSpiceApp;
use super::dashboard::SECTION_GAP;
use crate::app::localization::UiText;
use crate::app::navigation::StudioWorkspace;
use crate::app::review::widgets::REVIEW_FINDINGS;
use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText, Vec2};

impl NekoSpiceApp {
    pub(crate) fn draw_home_command_center(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::CommandCenter),
            ));
            ui.add_space(6.0);
            self.draw_home_command_metrics(ui);
            ui.add_space(SECTION_GAP);
            self.draw_home_next_steps(ui);
        });
    }

    fn draw_home_command_metrics(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let columns = if ui.available_width() >= 760.0 { 4 } else { 2 };
        let spacing = 8.0;
        let width =
            ((ui.available_width() - spacing * (columns - 1) as f32) / columns as f32).max(120.0);

        egui::Grid::new("home_command_metrics")
            .num_columns(columns)
            .spacing(Vec2::new(spacing, spacing))
            .show(ui, |ui| {
                for (index, card) in self.home_command_metric_cards().into_iter().enumerate() {
                    command_metric_card(ui, mode, width, card);
                    if (index + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });

        ui.label(StudioTheme::muted_for(
            mode,
            format!(
                "{}: {} / {}: {}",
                self.text(UiText::Renderer),
                "wgpu",
                self.text(UiText::Solver),
                "ngspice"
            ),
        ));
        ui.colored_label(palette.success, self.text(UiText::SystemOperational));
    }

    fn draw_home_next_steps(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::NextSteps),
        ));
        let steps = [
            HomeNextStep {
                label: self.text(UiText::OpenSchematic),
                workspace: StudioWorkspace::Schematic,
            },
            HomeNextStep {
                label: self.text(UiText::RunSimulation),
                workspace: StudioWorkspace::Simulation,
            },
            HomeNextStep {
                label: self.text(UiText::DesignReview),
                workspace: StudioWorkspace::Review,
            },
            HomeNextStep {
                label: self.text(UiText::ModelLibrary),
                workspace: StudioWorkspace::Library,
            },
        ];

        ui.horizontal_wrapped(|ui| {
            for step in steps {
                if next_step_button(ui, mode, step.label) {
                    self.active_workspace = step.workspace;
                }
            }
        });
    }

    fn home_command_metric_cards(&self) -> [HomeMetricCard; 4] {
        let open_issues = REVIEW_FINDINGS.len().to_string();
        let symbol_coverage = self.symbol_coverage_text();
        let run_state = self
            .simulation_panel
            .last_run
            .as_ref()
            .map(|run| run.metadata.status.as_str().to_string())
            .unwrap_or_else(|| self.text(UiText::Queued).to_string());

        [
            HomeMetricCard {
                title: self.text(UiText::OpenIssues),
                value: open_issues,
                caption: self.text(UiText::DesignReview).to_string(),
                color: StudioTheme::palette(self.theme_mode()).warning,
            },
            HomeMetricCard {
                title: self.text(UiText::ReviewScore),
                value: "72".to_string(),
                caption: self.text(UiText::HealthReady).to_string(),
                color: StudioTheme::palette(self.theme_mode()).success,
            },
            HomeMetricCard {
                title: self.text(UiText::SymbolCoverage),
                value: symbol_coverage,
                caption: self.text(UiText::LibraryStatus).to_string(),
                color: StudioTheme::palette(self.theme_mode()).accent,
            },
            HomeMetricCard {
                title: self.text(UiText::LastRun),
                value: run_state,
                caption: self.text(UiText::SimulationQueue).to_string(),
                color: StudioTheme::palette(self.theme_mode()).success,
            },
        ]
    }

    fn symbol_coverage_text(&self) -> String {
        let schematic_symbols = self
            .scene
            .as_ref()
            .map(|scene| scene.symbols.len())
            .unwrap_or_default();
        let library_symbols = self
            .library
            .as_ref()
            .map(|library| library.index().symbols.len())
            .unwrap_or_default();
        if schematic_symbols == 0 || library_symbols == 0 {
            return self.text(UiText::Missing).to_string();
        }
        format!("{schematic_symbols} / {library_symbols}")
    }
}

#[derive(Debug)]
struct HomeMetricCard {
    title: &'static str,
    value: String,
    caption: String,
    color: Color32,
}

#[derive(Debug, Clone, Copy)]
struct HomeNextStep {
    label: &'static str,
    workspace: StudioWorkspace,
}

fn command_metric_card(ui: &mut egui::Ui, mode: StudioThemeMode, width: f32, card: HomeMetricCard) {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
        .fill(palette.panel_soft)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_min_height(72.0);
            ui.label(StudioTheme::muted_for(mode, card.title));
            ui.label(
                RichText::new(card.value)
                    .size(18.0)
                    .strong()
                    .color(card.color),
            );
            ui.label(StudioTheme::muted_for(mode, card.caption));
        });
}

fn next_step_button(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str) -> bool {
    ui.add(
        egui::Button::new(RichText::new(label).strong())
            .fill(StudioTheme::palette(mode).panel_soft)
            .stroke(egui::Stroke::new(1.0, StudioTheme::palette(mode).border))
            .corner_radius(4),
    )
    .clicked()
}
