use super::NekoSpiceApp;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self, Pos2, Rect, RichText, Sense, Stroke, Vec2};

pub(super) fn two_column(
    app: &mut NekoSpiceApp,
    ui: &mut egui::Ui,
    left_ratio: f32,
    left: impl FnOnce(&mut NekoSpiceApp, &mut egui::Ui),
    right: impl FnOnce(&mut NekoSpiceApp, &mut egui::Ui),
) {
    let spacing = 10.0;
    let width = ui.available_width();
    if width < 520.0 {
        ui.vertical(|ui| {
            left(app, ui);
            ui.add_space(spacing);
            right(app, ui);
        });
        return;
    }

    let left_width = ((width - spacing) * left_ratio).max(220.0);
    let right_width = (width - left_width - spacing).max(220.0);
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(left_width);
            left(app, ui);
        });
        ui.add_space(spacing);
        ui.vertical(|ui| {
            ui.set_width(right_width);
            right(app, ui);
        });
    });
}

pub(super) fn section_header(ui: &mut egui::Ui, mode: StudioThemeMode, title: &str, action: &str) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::section_title_for(mode, title));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, action));
        });
    });
}

/// Section header with a clickable action link. Returns true if clicked.
pub(super) fn section_header_clickable(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    action: &str,
) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(StudioTheme::section_title_for(mode, title));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.selectable_label(false, StudioTheme::accent_for(mode, action)).clicked() {
                clicked = true;
            }
        });
    });
    clicked
}

pub(super) fn project_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    path: &str,
    state: &str,
) {
    let palette = StudioTheme::palette(mode);
    ui.horizontal(|ui| {
        ui.label(StudioTheme::accent_for(mode, "SCH"));
        ui.label(RichText::new(title).strong().color(palette.text));
        ui.label(StudioTheme::muted_for(mode, state));
    });
    ui.label(StudioTheme::muted_for(mode, path));
    ui.separator();
}

pub(super) fn template_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    caption: &str,
    use_text: &str,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let mut clicked = false;
    StudioTheme::panel_frame_for(mode).show(ui, |ui| {
        ui.set_min_height(106.0);
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).strong().size(13.0).color(palette.text));
        ui.label(RichText::new(caption).size(12.0).color(palette.text_muted));
        ui.add_space(6.0);
        draw_template_glyph(ui, mode);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button(format!("+ {}", use_text)).clicked() {
                clicked = true;
            }
        });
    });
    clicked
}

pub(super) fn queue_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    index: &str,
    name: &str,
    detail: &str,
    status: &str,
) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::accent_for(mode, index));
        ui.vertical(|ui| {
            ui.label(name);
            ui.label(StudioTheme::muted_for(mode, detail));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, status));
        });
    });
}

pub(super) fn measurement_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    signal: &str,
    value: &str,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.label(StudioTheme::muted_for(mode, signal));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(StudioTheme::accent_for(mode, value));
            draw_sparkline(ui, mode);
        });
    });
    ui.separator();
}

pub(super) fn recommendation_row(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    title: &str,
    action: &str,
) {
    ui.horizontal(|ui| {
        ui.label(StudioTheme::accent_for(mode, "REC"));
        ui.label(title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let _ = ui.small_button(action);
        });
    });
    ui.separator();
}

fn draw_template_glyph(ui: &mut egui::Ui, mode: StudioThemeMode) {
    let desired = Vec2::new(ui.available_width().max(80.0), 28.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let palette = StudioTheme::palette(mode);
    let painter = ui.painter_at(rect);
    let y = rect.center().y;
    let x0 = rect.left() + 10.0;
    let x1 = rect.right() - 10.0;
    painter.line_segment(
        [Pos2::new(x0, y), Pos2::new(x1, y)],
        Stroke::new(1.2, palette.accent),
    );
    painter.circle_stroke(
        Pos2::new(x0 + rect.width() * 0.25, y),
        5.0,
        Stroke::new(1.2, palette.accent),
    );
    painter.line_segment(
        [
            Pos2::new(x0 + rect.width() * 0.55, rect.top() + 6.0),
            Pos2::new(x0 + rect.width() * 0.55, rect.bottom() - 6.0),
        ],
        Stroke::new(1.2, palette.accent),
    );
}

fn draw_sparkline(ui: &mut egui::Ui, mode: StudioThemeMode) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(58.0, 16.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let palette = StudioTheme::palette(mode);
    let points = sparkline_points(rect);
    painter.add(egui::Shape::line(points, Stroke::new(1.2, palette.success)));
}

fn sparkline_points(rect: Rect) -> Vec<Pos2> {
    let values = [0.25, 0.35, 0.30, 0.58, 0.50, 0.78, 0.70, 0.88];
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let x = rect.left() + rect.width() * (index as f32 / (values.len() - 1) as f32);
            let y = rect.bottom() - rect.height() * value;
            Pos2::new(x, y)
        })
        .collect()
}
