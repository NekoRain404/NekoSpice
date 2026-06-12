use crate::app::NekoSpiceApp;
use crate::app::theme::StudioTheme;
use eframe::egui;

impl NekoSpiceApp {
    pub(crate) fn draw_report_preview_mock_page(&self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = StudioTheme::palette(mode);
        let width = ui.available_width().clamp(220.0, 340.0);
        let height = width * 1.28;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, egui::CornerRadius::same(3), palette.canvas);
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(3),
            egui::Stroke::new(1.0, palette.border),
            egui::StrokeKind::Inside,
        );
        painter.text(
            rect.left_top() + egui::vec2(18.0, 18.0),
            egui::Align2::LEFT_TOP,
            "NekoSpice Studio",
            egui::FontId::proportional(15.0),
            palette.background,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(
                rect.left_top() + egui::vec2(18.0, 58.0),
                egui::vec2(width - 36.0, 46.0),
            ),
            egui::CornerRadius::same(4),
            palette.panel,
        );
        painter.text(
            rect.left_top() + egui::vec2(32.0, 72.0),
            egui::Align2::LEFT_TOP,
            "PASS     28     100%",
            egui::FontId::monospace(13.0),
            palette.success,
        );
        for index in 0..5 {
            let y = rect.top() + 132.0 + index as f32 * 24.0;
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 18.0, y),
                    egui::pos2(rect.right() - 18.0, y),
                ],
                egui::Stroke::new(1.0, palette.border),
            );
        }
    }
}

pub(crate) fn draw_grid(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    for index in 1..6 {
        let x = egui::lerp(rect.left()..=rect.right(), index as f32 / 6.0);
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, color),
        );
        let y = egui::lerp(rect.top()..=rect.bottom(), index as f32 / 6.0);
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, color),
        );
    }
}

pub(crate) fn draw_curve(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    phase: f32,
) {
    let points: Vec<_> = (0..80)
        .map(|step| {
            let t = step as f32 / 79.0;
            let x = egui::lerp(rect.left() + 12.0..=rect.right() - 12.0, t);
            let wave = ((t * std::f32::consts::TAU * 1.4) + phase).sin();
            let falloff = t.powf(1.6) * rect.height() * 0.46;
            let y = rect.center().y - wave * 18.0 + falloff - 36.0;
            egui::pos2(x, y)
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(2.0, color)));
}
