use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Frame, Margin, RichText, Stroke, TextStyle,
    Vec2,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum StudioThemeMode {
    #[default]
    Midnight,
    Graphite,
    Light,
}

impl StudioThemeMode {
    pub(super) const ALL: [Self; 3] = [Self::Midnight, Self::Graphite, Self::Light];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Midnight => "Midnight",
            Self::Graphite => "Graphite",
            Self::Light => "Light",
        }
    }

    pub(super) fn label_zh(self) -> &'static str {
        match self {
            Self::Midnight => "深夜",
            Self::Graphite => "石墨",
            Self::Light => "浅色",
        }
    }

    pub(super) fn next(self) -> Self {
        match self {
            Self::Midnight => Self::Graphite,
            Self::Graphite => Self::Light,
            Self::Light => Self::Midnight,
        }
    }

    fn uses_light_visuals(self) -> bool {
        matches!(self, Self::Light)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct StudioPalette {
    pub(super) background: Color32,
    pub(super) panel: Color32,
    pub(super) panel_soft: Color32,
    pub(super) panel_hover: Color32,
    pub(super) canvas: Color32,
    pub(super) border: Color32,
    pub(super) border_strong: Color32,
    pub(super) text: Color32,
    pub(super) text_muted: Color32,
    pub(super) accent: Color32,
    pub(super) accent_soft: Color32,
    pub(super) success: Color32,
    pub(super) warning: Color32,
    pub(super) danger: Color32,
    pub(super) strip: Color32,
    pub(super) strip_border: Color32,
}

pub(super) struct StudioTheme;

#[allow(dead_code)]
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

    pub(super) fn palette(mode: StudioThemeMode) -> StudioPalette {
        match mode {
            StudioThemeMode::Midnight => StudioPalette {
                background: Color32::from_rgb(13, 17, 23),     // GitHub dark bg
                panel: Color32::from_rgb(22, 27, 34),          // GitHub dark surface
                panel_soft: Color32::from_rgb(33, 38, 45),     // GitHub dark elevated
                panel_hover: Color32::from_rgb(48, 54, 61),    // GitHub dark hover
                canvas: Color32::from_rgb(236, 240, 244),      // Light canvas (KiCad)
                border: Color32::from_rgb(48, 54, 61),         // GitHub dark border
                border_strong: Color32::from_rgb(88, 96, 105), // GitHub dark border strong
                text: Color32::from_rgb(230, 237, 243),        // Bright text
                text_muted: Color32::from_rgb(139, 148, 158),  // Muted text
                accent: Color32::from_rgb(56, 139, 253),       // GitHub blue
                accent_soft: Color32::from_rgb(24, 45, 77),    // Accent background
                success: Color32::from_rgb(63, 185, 80),       // GitHub green
                warning: Color32::from_rgb(210, 153, 34),      // GitHub yellow
                danger: Color32::from_rgb(248, 81, 73),        // GitHub red
                strip: Color32::from_rgb(13, 17, 23),
                strip_border: Color32::from_rgb(33, 38, 45),
            },
            StudioThemeMode::Graphite => StudioPalette {
                background: Color32::from_rgb(24, 25, 28),
                panel: Color32::from_rgb(32, 33, 38),
                panel_soft: Color32::from_rgb(40, 42, 48),
                panel_hover: Color32::from_rgb(52, 55, 63),
                canvas: Color32::from_rgb(232, 234, 237),
                border: Color32::from_rgb(60, 65, 75),
                border_strong: Color32::from_rgb(90, 98, 112),
                text: Color32::from_rgb(235, 239, 244),
                text_muted: Color32::from_rgb(148, 158, 172),
                accent: Color32::from_rgb(64, 160, 255),
                accent_soft: Color32::from_rgb(26, 58, 92),
                success: Color32::from_rgb(68, 188, 112),
                warning: Color32::from_rgb(218, 162, 52),
                danger: Color32::from_rgb(228, 88, 88),
                strip: Color32::from_rgb(24, 25, 28),
                strip_border: Color32::from_rgb(40, 42, 48),
            },
            StudioThemeMode::Light => StudioPalette {
                background: Color32::from_rgb(236, 240, 245),
                panel: Color32::from_rgb(252, 253, 255),
                panel_soft: Color32::from_rgb(242, 246, 251),
                panel_hover: Color32::from_rgb(228, 238, 250),
                canvas: Color32::from_rgb(250, 251, 253),
                border: Color32::from_rgb(194, 205, 220),
                border_strong: Color32::from_rgb(121, 146, 175),
                text: Color32::from_rgb(24, 34, 48),
                text_muted: Color32::from_rgb(86, 102, 122),
                accent: Color32::from_rgb(0, 106, 220),
                accent_soft: Color32::from_rgb(214, 231, 252),
                success: Color32::from_rgb(30, 142, 82),
                warning: Color32::from_rgb(178, 113, 23),
                danger: Color32::from_rgb(192, 52, 52),
                strip: Color32::from_rgb(247, 250, 253),
                strip_border: Color32::from_rgb(204, 214, 228),
            },
        }
    }

    pub(super) fn apply(ctx: &egui::Context, mode: StudioThemeMode) {
        let palette = Self::palette(mode);
        ctx.set_visuals(if mode.uses_light_visuals() {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        });
        let mut style = (*ctx.global_style()).clone();
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.window_margin = Margin::same(10);
        style.visuals.panel_fill = palette.background;
        style.visuals.window_fill = palette.panel;
        style.visuals.extreme_bg_color = palette.background;
        style.visuals.faint_bg_color = palette.panel_soft;
        style.visuals.widgets.noninteractive.bg_fill = palette.panel_soft;
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
        style.visuals.widgets.inactive.bg_fill = palette.panel_soft;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
        style.visuals.widgets.hovered.bg_fill = palette.panel_hover;
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
        style.visuals.widgets.active.bg_fill = palette.accent_soft;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
        style.visuals.selection.bg_fill = palette.accent_soft;
        style.visuals.selection.stroke = Stroke::new(1.0, palette.accent);
        style.visuals.override_text_color = Some(palette.text);
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

    pub(super) fn panel_frame_for(mode: StudioThemeMode) -> Frame {
        let palette = Self::palette(mode);
        Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(12))
    }

    pub(super) fn section_title_for(mode: StudioThemeMode, text: impl Into<String>) -> RichText {
        RichText::new(text.into())
            .strong()
            .color(Self::palette(mode).text)
    }

    pub(super) fn muted_for(mode: StudioThemeMode, text: impl Into<String>) -> RichText {
        RichText::new(text.into()).color(Self::palette(mode).text_muted)
    }

    pub(super) fn accent_for(mode: StudioThemeMode, text: impl Into<String>) -> RichText {
        RichText::new(text.into())
            .strong()
            .color(Self::palette(mode).accent)
    }

    pub(super) fn status_dot(color: Color32) -> RichText {
        RichText::new("*").color(color)
    }
}

#[cfg(test)]
mod tests {
    use super::{StudioTheme, StudioThemeMode};

    #[test]
    fn studio_theme_uses_distinct_accent_and_background() {
        assert_ne!(StudioTheme::ACCENT, StudioTheme::BACKGROUND);
        assert_ne!(StudioTheme::PANEL, StudioTheme::CANVAS);
    }

    #[test]
    fn studio_theme_modes_have_distinct_palettes() {
        assert_eq!(StudioThemeMode::ALL.len(), 3);
        assert_ne!(
            StudioTheme::palette(StudioThemeMode::Midnight).background,
            StudioTheme::palette(StudioThemeMode::Light).background
        );
    }
}
