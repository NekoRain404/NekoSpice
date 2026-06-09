use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, RichText};

pub(super) fn library_filter_tab(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    count: usize,
    active: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let text = if active {
        RichText::new(format!("{label}  {count}"))
            .strong()
            .color(palette.text)
    } else {
        StudioTheme::muted_for(mode, format!("{label}  {count}"))
    };
    ui.add(
        egui::Button::new(text)
            .fill(if active {
                palette.accent_soft
            } else {
                palette.panel_soft
            })
            .stroke(egui::Stroke::new(
                1.0,
                if active {
                    palette.accent
                } else {
                    palette.border
                },
            ))
            .corner_radius(4),
    )
    .clicked()
}

pub(super) fn symbol_list_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    id: &str,
    detail: &str,
    stats: &str,
    active: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let fill = if active {
        palette.accent_soft
    } else {
        palette.panel
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(
            1.0,
            if active {
                palette.accent
            } else {
                palette.border
            },
        ))
        .corner_radius(5)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(id).strong().color(palette.text));
                    ui.label(StudioTheme::muted_for(mode, detail));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(StudioTheme::accent_for(mode, stats));
                });
            });
        })
        .response
        .interact(egui::Sense::click())
        .clicked()
}

pub(super) fn code_line(ui: &mut egui::Ui, line_number: usize, text: &str) {
    ui.horizontal(|ui| {
        ui.monospace(format!("{line_number:>2}"));
        ui.monospace(text);
    });
}

pub(super) fn metadata_row(ui: &mut egui::Ui, mode: StudioThemeMode, label: &str, value: &str) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).color(palette.text));
        });
    });
}
