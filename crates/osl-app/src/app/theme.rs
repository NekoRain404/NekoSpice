//! 主题系统。提供 Midnight/Graphite/Light 三种主题的颜色和样式定义。
//!
//! 颜色方案参考现代 IDE 设计（VS Code / JetBrains Fleet），强调层次感和可读性。
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

/// 主题模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StudioThemeMode {
    Midnight,
    Graphite,
    Light,
}

impl Default for StudioThemeMode {
    fn default() -> Self {
        Self::Midnight
    }
}

impl StudioThemeMode {
    pub(super) const ALL: [Self; 3] = [Self::Midnight, Self::Graphite, Self::Light];

    /// label。
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Midnight => "Midnight",
            Self::Graphite => "Graphite",
            Self::Light => "Light",
        }
    }

    /// label zh。
    pub(super) fn label_zh(self) -> &'static str {
        match self {
            Self::Midnight => "深夜",
            Self::Graphite => "石墨",
            Self::Light => "浅色",
        }
    }

    /// next。
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

/// 主题调色板。现代 IDE 风格，强调层次感。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StudioPalette {
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

/// 主题工具集
pub(crate) struct StudioTheme;

impl StudioTheme {
    /// 根据主题模式返回调色板
    pub(crate) fn palette(mode: StudioThemeMode) -> StudioPalette {
        match mode {
            StudioThemeMode::Midnight => StudioPalette {
                // 深邃但不死黑的背景，参考 VS Code Dark+
                background: Color32::from_rgb(18, 18, 24),
                panel: Color32::from_rgb(24, 25, 32),
                panel_soft: Color32::from_rgb(30, 32, 40),
                panel_hover: Color32::from_rgb(38, 42, 54),
                canvas: Color32::from_rgb(14, 14, 18),
                border: Color32::from_rgb(42, 44, 56),
                border_strong: Color32::from_rgb(56, 60, 76),
                text: Color32::from_rgb(224, 228, 237),
                text_muted: Color32::from_rgb(130, 136, 150),
                accent: Color32::from_rgb(88, 166, 255),
                accent_soft: Color32::from_rgb(28, 48, 76),
                success: Color32::from_rgb(88, 200, 130),
                warning: Color32::from_rgb(255, 190, 60),
                danger: Color32::from_rgb(255, 100, 100),
                strip: Color32::from_rgb(14, 14, 18),
                strip_border: Color32::from_rgb(32, 34, 44),
            },
            StudioThemeMode::Graphite => StudioPalette {
                background: Color32::from_rgb(30, 30, 34),
                panel: Color32::from_rgb(38, 38, 44),
                panel_soft: Color32::from_rgb(46, 46, 54),
                panel_hover: Color32::from_rgb(56, 56, 66),
                canvas: Color32::from_rgb(240, 242, 245),
                border: Color32::from_rgb(56, 58, 68),
                border_strong: Color32::from_rgb(76, 80, 94),
                text: Color32::from_rgb(228, 232, 238),
                text_muted: Color32::from_rgb(142, 148, 162),
                accent: Color32::from_rgb(80, 160, 255),
                accent_soft: Color32::from_rgb(32, 54, 86),
                success: Color32::from_rgb(80, 192, 120),
                warning: Color32::from_rgb(245, 182, 55),
                danger: Color32::from_rgb(245, 95, 95),
                strip: Color32::from_rgb(28, 28, 32),
                strip_border: Color32::from_rgb(44, 46, 54),
            },
            StudioThemeMode::Light => StudioPalette {
                background: Color32::from_rgb(242, 244, 248),
                panel: Color32::from_rgb(252, 253, 255),
                panel_soft: Color32::from_rgb(248, 249, 252),
                panel_hover: Color32::from_rgb(238, 240, 246),
                canvas: Color32::from_rgb(248, 250, 254),
                border: Color32::from_rgb(210, 216, 226),
                border_strong: Color32::from_rgb(180, 188, 202),
                text: Color32::from_rgb(30, 34, 42),
                text_muted: Color32::from_rgb(110, 118, 134),
                accent: Color32::from_rgb(40, 120, 220),
                accent_soft: Color32::from_rgb(220, 236, 255),
                success: Color32::from_rgb(40, 168, 96),
                warning: Color32::from_rgb(200, 148, 24),
                danger: Color32::from_rgb(220, 60, 60),
                strip: Color32::from_rgb(252, 253, 255),
                strip_border: Color32::from_rgb(218, 222, 230),
            },
        }
    }

    /// 应用主题到 egui 上下文
    pub(crate) fn apply(ctx: &egui::Context, mode: StudioThemeMode) {
        let palette = Self::palette(mode);
        let mut style = (*ctx.style()).clone();

        // 全局字体
        style.visuals.override_text_color = Some(palette.text);

        // 面板样式
        style.spacing.item_spacing = Vec2::new(6.0, 4.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.spacing.window_margin = egui::Margin::same(12);

        // 圆角

        // Widget 样式
        if !mode.uses_light_visuals() {
            // 深色主题
            style.visuals.widgets.noninteractive.bg_fill = palette.panel;
            style.visuals.widgets.noninteractive.bg_stroke = Stroke::NONE;
            style.visuals.widgets.inactive.bg_fill = palette.panel_soft;
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
            style.visuals.widgets.hovered.bg_fill = palette.panel_hover;
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
            style.visuals.widgets.active.bg_fill = palette.accent_soft;
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
            // 窗口和弹出面板
            style.visuals.window_fill = palette.panel;
            style.visuals.window_stroke = Stroke::new(1.0, palette.border);
            style.visuals.window_corner_radius = CornerRadius::same(6);
            // 菜单
            style.visuals.widgets.open.bg_fill = palette.panel_hover;
            // 分割线
            style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.border);
        } else {
            // 浅色主题
            style.visuals.widgets.noninteractive.bg_fill = palette.panel;
            style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
            style.visuals.widgets.inactive.bg_fill = Color32::WHITE;
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
            style.visuals.widgets.hovered.bg_fill = palette.panel_hover;
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_strong);
            style.visuals.widgets.active.bg_fill = palette.accent_soft;
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
            style.visuals.window_fill = palette.panel;
            style.visuals.window_stroke = Stroke::new(1.0, palette.border);
            style.visuals.window_corner_radius = CornerRadius::same(6);
            style.visuals.widgets.open.bg_fill = palette.panel_hover;
            style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.border);
        }

        // 选择框样式
        style.visuals.selection.bg_fill = palette.accent;
        style.visuals.selection.stroke = Stroke::new(1.0, palette.accent);

        ctx.set_style(style);
    }

    /// 创建面板帧样式
    pub(crate) fn panel_frame_for(mode: StudioThemeMode) -> egui::Frame {
        let palette = Self::palette(mode);
        egui::Frame::none()
            .fill(palette.panel)
            .inner_margin(egui::Margin::same(12))
            .rounding(CornerRadius::same(6))
            .stroke(Stroke::new(1.0, palette.border))
    }

    /// 创建卡片帧样式
    pub(crate) fn card_frame_for(mode: StudioThemeMode) -> egui::Frame {
        let palette = Self::palette(mode);
        egui::Frame::none()
            .fill(palette.panel_soft)
            .inner_margin(egui::Margin::same(10))
            .rounding(CornerRadius::same(4))
            .stroke(Stroke::new(1.0, palette.border))
    }

    /// 区段标题样式
    pub(crate) fn section_title_for(mode: StudioThemeMode, text: impl Into<String>) -> RichText {
        let palette = Self::palette(mode);
        RichText::new(text.into())
            .color(palette.text)
            .strong()
            .size(13.0)
    }

    /// 静音文字样式
    pub(crate) fn muted_for(mode: StudioThemeMode, text: impl Into<String>) -> RichText {
        let palette = Self::palette(mode);
        RichText::new(text.into()).color(palette.text_muted).size(12.0)
    }

    /// 强调文字样式
    pub(crate) fn accent_for(mode: StudioThemeMode, text: impl Into<String>) -> RichText {
        let palette = Self::palette(mode);
        RichText::new(text.into()).color(palette.accent).size(12.0)
    }

    /// 状态圆点
    pub(crate) fn status_dot(color: Color32) -> RichText {
        RichText::new("\u{25CF}").color(color).size(8.0)
    }

    /// 工具栏按钮样式
    pub(crate) fn toolbar_button_style(mode: StudioThemeMode) -> egui::Style {
        let palette = Self::palette(mode);
        let mut style = egui::Style::default();
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
        style.visuals.widgets.hovered.bg_fill = palette.panel_hover;
        style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        style.visuals.override_text_color = Some(palette.text);
        style.spacing.button_padding = Vec2::new(8.0, 4.0);
        style
    }
}

/// 诊断计数摘要（用于状态栏）
#[allow(dead_code)]
pub(crate) struct DiagnosticCounts {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
}

#[allow(dead_code)]
impl DiagnosticCounts {
    pub fn total(&self) -> usize {
        self.errors + self.warnings + self.info
    }
    pub fn is_clean(&self) -> bool {
        self.total() == 0
    }
}
