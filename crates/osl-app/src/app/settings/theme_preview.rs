use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::{StudioPalette, StudioTheme, StudioThemeMode};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

impl NekoSpiceApp {
    /// draw settings theme gallery。
    pub(crate) fn draw_settings_theme_gallery(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::Appearance),
            ));
            ui.label(StudioTheme::muted_for(
                mode,
                format!(
                    "{}: {}",
                    self.text(UiText::CurrentTheme),
                    self.theme_mode_label(mode)
                ),
            ));
            ui.add_space(8.0);

            let spacing = 8.0;
            let columns = if ui.available_width() >= 720.0 { 3 } else { 1 };
            let card_width = ((ui.available_width() - spacing * (columns - 1) as f32)
                / columns as f32)
                .max(180.0);

            egui::Grid::new("settings_theme_gallery")
                .num_columns(columns)
                .spacing(Vec2::new(spacing, spacing))
                .show(ui, |ui| {
                    for (index, candidate) in StudioThemeMode::ALL.into_iter().enumerate() {
                        let selected = self.preferences.theme_mode == candidate;
                        let label = self.theme_mode_label(candidate);
                        if theme_preview_card(ui, candidate, selected, label, card_width) {
                            self.preferences.theme_mode = candidate;
                            self.status_message =
                                Some(format!("{}: {}", self.text(UiText::Theme), label));
                        }
                        if (index + 1) % columns == 0 {
                            ui.end_row();
                        }
                    }
                });

            ui.add_space(8.0);
            self.draw_settings_visual_readiness(ui);
        });
    }

    fn draw_settings_visual_readiness(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal_wrapped(|ui| {
            readiness_pill(
                ui,
                mode,
                self.text(UiText::Renderer),
                "wgpu",
                palette.success,
            );
            readiness_pill(
                ui,
                mode,
                self.text(UiText::Language),
                self.locale().native_name(),
                palette.accent,
            );
            readiness_pill(
                ui,
                mode,
                self.text(UiText::CanvasLinked),
                "KiCad",
                palette.success,
            );
            readiness_pill(
                ui,
                mode,
                self.text(UiText::Solver),
                "ngspice",
                palette.success,
            );
        });
    }
}

fn theme_preview_card(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    selected: bool,
    label: &str,
    width: f32,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let stroke = if selected {
        Stroke::new(1.5, palette.accent)
    } else {
        Stroke::new(1.0, palette.border)
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 132.0), Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, palette.panel);
    painter.rect_stroke(rect, 6.0, stroke, StrokeKind::Inside);
    painter.text(
        Pos2::new(rect.left() + 14.0, rect.top() + 16.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(14.0),
        palette.text,
    );
    if selected {
        painter.text(
            Pos2::new(rect.right() - 16.0, rect.top() + 16.0),
            egui::Align2::RIGHT_TOP,
            "*",
            egui::FontId::proportional(14.0),
            palette.success,
        );
    }
    draw_palette_swatches(&painter, palette, rect);
    draw_theme_canvas_preview(&painter, palette, rect);
    response.clicked()
}

fn draw_palette_swatches(painter: &egui::Painter, palette: StudioPalette, rect: Rect) {
    for (index, color) in [
        palette.background,
        palette.panel,
        palette.panel_soft,
        palette.accent,
        palette.success,
        palette.warning,
        palette.danger,
    ]
    .into_iter()
    .enumerate()
    {
        let left = rect.left() + 14.0 + index as f32 * 24.0;
        let swatch = Rect::from_min_size(Pos2::new(left, rect.top() + 48.0), Vec2::splat(16.0));
        painter.rect_filled(swatch, 3.0, color);
        painter.rect_stroke(
            swatch,
            3.0,
            Stroke::new(1.0, palette.border_strong),
            StrokeKind::Inside,
        );
    }
}

fn draw_theme_canvas_preview(painter: &egui::Painter, palette: StudioPalette, rect: Rect) {
    let preview = Rect::from_min_max(
        Pos2::new(rect.left() + 14.0, rect.top() + 78.0),
        Pos2::new(rect.right() - 14.0, rect.bottom() - 12.0),
    );
    painter.rect_filled(preview, 4.0, palette.canvas);
    painter.rect_stroke(
        preview,
        4.0,
        Stroke::new(1.0, palette.border),
        StrokeKind::Inside,
    );

    let grid = palette.border.linear_multiply(0.35);
    for step in 1..4 {
        let x = preview.left() + preview.width() * step as f32 / 4.0;
        painter.line_segment(
            [Pos2::new(x, preview.top()), Pos2::new(x, preview.bottom())],
            Stroke::new(1.0, grid),
        );
    }
    let y = preview.center().y;
    painter.line_segment(
        [
            Pos2::new(preview.left() + 8.0, y),
            Pos2::new(preview.right() - 8.0, y),
        ],
        Stroke::new(1.5, palette.accent),
    );
    let symbol = Rect::from_center_size(preview.center(), Vec2::new(30.0, 18.0));
    painter.rect_stroke(
        symbol,
        2.0,
        Stroke::new(1.2, Color32::from_black_alpha(180)),
        StrokeKind::Inside,
    );
}

fn readiness_pill(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    label: &str,
    value: &str,
    color: Color32,
) {
    let palette = StudioTheme::palette(mode);
    egui::Frame::new()
        .fill(palette.panel_soft)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(4)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(label).color(palette.text_muted));
                ui.label(egui::RichText::new(value).strong().color(color));
            });
        });
}
