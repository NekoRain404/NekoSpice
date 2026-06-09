use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Frame, Margin, RichText, Stroke, TextStyle,
    Vec2,
};

pub(super) struct StudioTheme;

impl StudioTheme {
    pub(super) const BACKGROUND: Color32 = Color32::from_rgb(6, 16, 28);
    pub(super) const PANEL: Color32 = Color32::from_rgb(10, 25, 42);
    pub(super) const PANEL_SOFT: Color32 = Color32::from_rgb(14, 32, 52);
    pub(super) const PANEL_HOVER: Color32 = Color32::from_rgb(18, 44, 72);
    pub(super) const CANVAS: Color32 = Color32::from_rgb(236, 240, 244);
    pub(super) const BORDER: Color32 = Color32::from_rgb(37, 64, 94);
    pub(super) const BORDER_STRONG: Color32 = Color32::from_rgb(58, 101, 145);
    pub(super) const TEXT: Color32 = Color32::from_rgb(232, 239, 247);
    pub(super) const TEXT_MUTED: Color32 = Color32::from_rgb(142, 162, 184);
    pub(super) const ACCENT: Color32 = Color32::from_rgb(38, 137, 255);
    pub(super) const ACCENT_SOFT: Color32 = Color32::from_rgb(15, 63, 116);
    pub(super) const SUCCESS: Color32 = Color32::from_rgb(76, 202, 118);
    pub(super) const WARNING: Color32 = Color32::from_rgb(235, 174, 64);
    pub(super) const DANGER: Color32 = Color32::from_rgb(238, 91, 91);

    pub(super) fn apply(ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::dark());
        let mut style = (*ctx.global_style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.window_margin = Margin::same(10);
        style.visuals.panel_fill = Self::BACKGROUND;
        style.visuals.window_fill = Self::PANEL;
        style.visuals.extreme_bg_color = Self::BACKGROUND;
        style.visuals.faint_bg_color = Self::PANEL_SOFT;
        style.visuals.widgets.noninteractive.bg_fill = Self::PANEL_SOFT;
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        style.visuals.widgets.inactive.bg_fill = Self::PANEL_SOFT;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Self::BORDER);
        style.visuals.widgets.hovered.bg_fill = Self::PANEL_HOVER;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Self::BORDER_STRONG);
        style.visuals.widgets.active.bg_fill = Self::ACCENT_SOFT;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, Self::ACCENT);
        style.visuals.selection.bg_fill = Self::ACCENT_SOFT;
        style.visuals.selection.stroke = Stroke::new(1.0, Self::ACCENT);
        style.visuals.override_text_color = Some(Self::TEXT);
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        );
        ctx.set_global_style(style);
    }

    pub(super) fn page_frame() -> Frame {
        Frame::new().fill(Self::BACKGROUND).inner_margin(0)
    }

    pub(super) fn panel_frame() -> Frame {
        Frame::new()
            .fill(Self::PANEL)
            .stroke(Stroke::new(1.0, Self::BORDER))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(12))
    }

    pub(super) fn strip_frame() -> Frame {
        Frame::new()
            .fill(Color32::from_rgb(7, 19, 34))
            .stroke(Stroke::new(1.0, Color32::from_rgb(18, 42, 68)))
            .inner_margin(Margin::symmetric(12, 8))
    }

    pub(super) fn section_title(text: &str) -> RichText {
        RichText::new(text).strong().color(Self::TEXT)
    }

    pub(super) fn muted(text: impl Into<String>) -> RichText {
        RichText::new(text.into()).color(Self::TEXT_MUTED)
    }

    pub(super) fn accent(text: impl Into<String>) -> RichText {
        RichText::new(text.into()).strong().color(Self::ACCENT)
    }

    pub(super) fn status_dot(color: Color32) -> RichText {
        RichText::new("*").color(color)
    }
}

#[cfg(test)]
mod tests {
    use super::StudioTheme;

    #[test]
    fn studio_theme_uses_distinct_accent_and_background() {
        assert_ne!(StudioTheme::ACCENT, StudioTheme::BACKGROUND);
        assert_ne!(StudioTheme::PANEL, StudioTheme::CANVAS);
    }
}
