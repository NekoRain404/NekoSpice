use super::state::{ReviewSeverity, ReviewSeverityFilter};
use crate::app::theme::{StudioPalette, StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ReviewFinding {
    pub(crate) severity: ReviewSeverity,
    pub(crate) title: &'static str,
    pub(crate) detail: &'static str,
}

pub(crate) const REVIEW_FINDINGS: [ReviewFinding; 4] = [
    ReviewFinding {
        severity: ReviewSeverity::Critical,
        title: "U2A Output Saturation",
        detail: "High load clipping",
    },
    ReviewFinding {
        severity: ReviewSeverity::Critical,
        title: "Power Supply Headroom",
        detail: "Low PSRR at high frequencies",
    },
    ReviewFinding {
        severity: ReviewSeverity::Major,
        title: "Stability Margin Low",
        detail: "Phase margin below target",
    },
    ReviewFinding {
        severity: ReviewSeverity::Minor,
        title: "Input Bias Current High",
        detail: "Review op-amp selection",
    },
];

pub(crate) fn severity_color(palette: StudioPalette, severity: ReviewSeverity) -> Color32 {
    match severity {
        ReviewSeverity::Critical => palette.danger,
        ReviewSeverity::Major => palette.warning,
        ReviewSeverity::Minor => palette.accent,
    }
}

pub(crate) fn review_metric_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(StudioTheme::palette(mode).text));
        });
    });
}

pub(crate) fn review_filter_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    active_filter: &mut ReviewSeverityFilter,
    filter: ReviewSeverityFilter,
    label: &str,
    value: &str,
    color: Color32,
) {
    ui.horizontal(|ui| {
        let selected = *active_filter == filter;
        if ui
            .selectable_label(selected, RichText::new(label).color(color).strong())
            .clicked()
        {
            *active_filter = filter;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(StudioTheme::palette(mode).text));
        });
    });
}

pub(crate) fn review_stat_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    color: Color32,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(color).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(StudioTheme::palette(mode).text));
        });
    });
}

pub(crate) fn review_issue_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    severity: &str,
    title: &str,
    detail: &str,
    color: Color32,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(severity).strong().color(color));
        ui.vertical(|ui| {
            ui.label(title);
            ui.label(StudioTheme::muted_for(mode, detail));
        });
    });
}

pub(crate) fn review_recommendation_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    detail: &str,
    impact: &str,
    color: Color32,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(title);
            ui.label(StudioTheme::muted_for(mode, detail));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(impact).strong().color(color));
            ui.label(StudioTheme::muted_for(mode, "Impact"));
        });
    });
}
