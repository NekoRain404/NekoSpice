//! Shared widget helpers for the simulation workspace.
//!
//! Provides reusable UI components: analysis mode buttons, metric cards,
//! code preview lines, profile summary rows, and status indicators.

use crate::app::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, RichText};

/// Interactive analysis mode button card with active state highlighting.
pub(crate) fn analysis_mode_button(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    caption: &str,
    active: bool,
) -> bool {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
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
        .corner_radius(5)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(132.0);
            ui.set_min_height(54.0);
            ui.label(RichText::new(title).strong().color(palette.text));
            ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
        })
        .response
        .interact(egui::Sense::click())
        .clicked()
}

/// Solver metric card displaying a label, large value, and caption.
pub(crate) fn solver_metric_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    caption: &str,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(72.0);
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).strong().size(18.0).color(palette.text));
        ui.label(RichText::new(caption).size(11.0).color(palette.text_muted));
    });
}

/// Status indicator card with a colored dot, label, and value.
#[allow(dead_code)]
pub(crate) fn status_indicator_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    color: Color32,
    label: &str,
    value: &str,
) {
    let palette = StudioTheme::palette(mode);
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(56.0);
        ui.horizontal(|ui| {
            ui.label(StudioTheme::status_dot(color));
            ui.vertical(|ui| {
                ui.label(StudioTheme::muted_for(mode, label));
                ui.label(RichText::new(value).strong().color(palette.text));
            });
        });
    });
}

/// Code preview line with line number gutter and syntax-aware coloring.
///
/// Colors:
/// - Comments (`*`) — muted grey
/// - Directives (`.tran`, `.ac`, `.options`, etc.) — purple
/// - Components/other — default text color
pub(crate) fn code_preview_line(ui: &mut egui::Ui, line_number: usize, text: &str) {
    let palette = StudioTheme::palette(StudioThemeMode::Midnight);
    let trimmed = text.trim_start();
    let color = if trimmed.starts_with('*') {
        // Comment line
        Color32::from_rgb(108, 122, 137)
    } else if trimmed.starts_with('.') {
        // SPICE directive
        Color32::from_rgb(180, 140, 255)
    } else {
        palette.text
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{line_number:>2}"))
                .monospace()
                .color(Color32::from_rgb(80, 90, 100)),
        );
        ui.label(RichText::new(text).monospace().color(color));
    });
}

/// Profile summary row showing a label, monospace value, and status tag.
pub(crate) fn profile_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    status: &str,
) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::muted_for(mode, label));
        ui.label(RichText::new(value).monospace().color(palette.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, status));
        });
    });
}

/// Analysis modes available in the overview and profile editor.
pub(crate) fn analysis_modes() -> [(
    osl_kicad::KicadSimulationDirectiveKind,
    &'static str,
    &'static str,
); 7] {
    [
        (
            osl_kicad::KicadSimulationDirectiveKind::Op,
            ".op",
            "operating point",
        ),
        (osl_kicad::KicadSimulationDirectiveKind::Dc, ".dc", "sweep"),
        (
            osl_kicad::KicadSimulationDirectiveKind::Tran,
            ".tran",
            "time domain",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Ac,
            ".ac",
            "small signal",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Noise,
            ".noise",
            "noise analysis",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Disto,
            ".disto",
            "distortion",
        ),
        (
            osl_kicad::KicadSimulationDirectiveKind::Sens,
            ".sens",
            "sensitivity",
        ),
    ]
}
