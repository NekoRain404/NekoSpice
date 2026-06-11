use super::NekoSpiceApp;
use super::localization::UiText;
use super::schematic_inspector_widgets::{property_row, status_pill};
use super::theme::StudioTheme;
use eframe::egui::{self, RichText};

impl NekoSpiceApp {
    pub(super) fn draw_schematic_review_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("schematic_review_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.draw_review_score_card(ui);
                ui.add_space(8.0);
                self.draw_review_issue_summary(ui);
                ui.add_space(8.0);
                self.draw_review_recommendations(ui);
                ui.add_space(8.0);
                self.draw_review_waveform_insight(ui);
                ui.add_space(8.0);
                self.draw_review_actions(ui);
            });
    }

    fn draw_review_score_card(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::DesignReview),
            ));
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(74.0, 74.0), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                painter.circle_stroke(rect.center(), 30.0, egui::Stroke::new(8.0, palette.border));
                painter.circle_stroke(rect.center(), 30.0, egui::Stroke::new(8.0, palette.success));
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "72",
                    egui::FontId::proportional(20.0),
                    palette.text,
                );
                ui.vertical(|ui| {
                    property_row(ui, mode, self.text(UiText::OverallScore), "Good");
                    property_row(ui, mode, self.text(UiText::Critical), "3");
                    property_row(ui, mode, self.text(UiText::Major), "5");
                    property_row(ui, mode, self.text(UiText::Minor), "8");
                    property_row(ui, mode, self.text(UiText::Suggestions), "2");
                });
            });
        });
    }

    fn draw_review_issue_summary(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Issues),
            ));
            ui.label(StudioTheme::muted_for(mode, self.text(UiText::Severity)));
            review_issue_row(
                ui,
                mode,
                self.text(UiText::Critical),
                "U2A Output Saturation",
                "High load clipping",
                palette.danger,
            );
            review_issue_row(
                ui,
                mode,
                self.text(UiText::Critical),
                "Power Supply Headroom",
                "Low PSRR at high frequencies",
                palette.danger,
            );
            review_issue_row(
                ui,
                mode,
                self.text(UiText::Major),
                "Stability Margin Low",
                "Phase margin below target",
                palette.warning,
            );
            review_issue_row(
                ui,
                mode,
                self.text(UiText::Minor),
                "Input Bias Current High",
                "Review op-amp selection",
                palette.accent,
            );
        });
    }

    fn draw_review_recommendations(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::TopRecommendations),
            ));
            ui.label(StudioTheme::muted_for(
                mode,
                self.text(UiText::Recommendations),
            ));
            recommendation_row(
                ui,
                mode,
                "Improve Stability Margin",
                "Increase compensation or lower gain",
                self.text(UiText::Impact),
                "High",
                palette.danger,
            );
            recommendation_row(
                ui,
                mode,
                "Increase Power Supply Decoupling",
                "Add local 100 nF and bulk capacitors",
                self.text(UiText::Impact),
                "High",
                palette.danger,
            );
            recommendation_row(
                ui,
                mode,
                "Add Output Isolation",
                "Place small series resistor before capacitive load",
                self.text(UiText::Impact),
                "Medium",
                palette.warning,
            );
        });
    }

    fn draw_review_waveform_insight(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::WaveformInsight),
            ));
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 80.0), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_stroke(
                rect,
                egui::CornerRadius::same(4),
                egui::Stroke::new(1.0, palette.border),
                egui::StrokeKind::Inside,
            );
            let points: Vec<_> = (0..80)
                .map(|step| {
                    let t = step as f32 / 79.0;
                    let x = egui::lerp(rect.left() + 8.0..=rect.right() - 8.0, t);
                    let y = rect.center().y + (t * std::f32::consts::TAU * 5.0).sin() * 10.0;
                    egui::pos2(x, y)
                })
                .collect();
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(2.0, palette.accent),
            ));
            ui.label(StudioTheme::muted_for(
                mode,
                "Output approaches rail during the final transient window.",
            ));
        });
    }

    fn draw_review_actions(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::AiAssistant),
            ));
            status_pill(ui, mode, self.text(UiText::CanvasLinked), palette.success);
            ui.add_space(6.0);
            if ui.button(self.text(UiText::ExplainWaveform)).clicked() {
                self.status_message = Some(self.text(UiText::ExplainWaveform).to_string());
            }
            if ui.button(self.text(UiText::FindOptimization)).clicked() {
                self.active_workspace = super::navigation::StudioWorkspace::Optimization;
            }
            if ui.button(self.text(UiText::VerifyStability)).clicked() {
                self.active_workspace = super::navigation::StudioWorkspace::Simulation;
            }
        });
    }
}

fn review_issue_row(
    ui: &mut egui::Ui,
    mode: super::theme::StudioThemeMode,
    severity: &str,
    title: &str,
    detail: &str,
    color: egui::Color32,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(severity).strong().color(color));
        ui.vertical(|ui| {
            ui.label(title);
            ui.label(StudioTheme::muted_for(mode, detail));
        });
    });
}

fn recommendation_row(
    ui: &mut egui::Ui,
    mode: super::theme::StudioThemeMode,
    title: &str,
    detail: &str,
    impact_label: &str,
    impact: &str,
    color: egui::Color32,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(title);
            ui.label(StudioTheme::muted_for(mode, detail));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(impact).strong().color(color));
            ui.label(StudioTheme::muted_for(mode, impact_label));
        });
    });
}
