use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText, Stroke};

pub(crate) fn metric_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    caption: &str,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).strong().size(18.0).color(palette.text));
        ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
    });
}

pub(crate) fn parameter_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    status: &str,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(StudioTheme::muted_for(mode, value));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, status));
        });
    });
}

pub(crate) fn result_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    rank: &str,
    values: &str,
    score: &str,
) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::accent_for(mode, rank));
        ui.monospace(values);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, score));
        });
    });
}

pub(crate) fn sweep_row(
    ui: &mut egui::Ui,
    parameter: &str,
    range: &str,
    samples: &str,
    status: &str,
) {
    ui.label(parameter);
    ui.monospace(range);
    ui.monospace(samples);
    ui.label(status);
    ui.end_row();
}

pub(crate) fn definition_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    parameter: &str,
    nominal: &str,
    distribution: &str,
    tolerance: &str,
    sensitivity: &str,
) {
    let palette = StudioTheme::palette(mode);
    let sensitivity_color = match sensitivity {
        "High" => palette.danger,
        "Med" => palette.warning,
        _ => palette.text_muted,
    };
    ui.label(parameter);
    ui.monospace(nominal);
    ui.label(distribution);
    ui.monospace(tolerance);
    ui.label(RichText::new(sensitivity).strong().color(sensitivity_color));
    ui.end_row();
}

pub(crate) fn measurement_row(
    ui: &mut egui::Ui,
    measurement: &str,
    kind: &str,
    limit: &str,
    goal: &str,
) {
    ui.label(measurement);
    ui.label(kind);
    ui.monospace(limit);
    ui.label(goal);
    ui.end_row();
}

pub(crate) fn progress_bar(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: f32) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(label);
        let bar = egui::ProgressBar::new(value)
            .desired_width(ui.available_width().max(80.0))
            .fill(palette.accent)
            .show_percentage();
        ui.add(bar);
    });
}

pub(crate) fn mini_donut(ui: &mut egui::Ui, mode: StudioThemeMode, pass_ratio: f32) {
    let palette = StudioTheme::palette(mode);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(76.0, 76.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = 32.0;
    painter.circle_stroke(center, radius, Stroke::new(9.0, palette.border));
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(9.0 * pass_ratio.clamp(0.0, 1.0), palette.success),
    );
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        "93.3%",
        egui::FontId::proportional(14.0),
        palette.text,
    );
}

pub(crate) fn status_chip(ui: &mut egui::Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).strong().color(color));
}
