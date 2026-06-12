//! 用户偏好管理。存储语言、主题、网格显示等界面设置。
//!
use super::NekoSpiceApp;
use super::localization::{StudioLocale, UiText};
use super::theme::{StudioPalette, StudioTheme, StudioThemeMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StudioPreferences {
    pub(super) theme_mode: StudioThemeMode,
    pub(super) locale: StudioLocale,
    /// Path to ngspice executable.
    pub(super) ngspice_path: String,
    /// Path to Xyce executable.
    pub(super) xyce_path: String,
}

impl Default for StudioPreferences {
    fn default() -> Self {
        Self {
            theme_mode: StudioThemeMode::default(),
            locale: StudioLocale::default(),
            ngspice_path: "ngspice".to_string(),
            xyce_path: "xyce".to_string(),
        }
    }
}

impl NekoSpiceApp {
    /// theme mode。
    pub(super) fn theme_mode(&self) -> StudioThemeMode {
        self.preferences.theme_mode
    }

    /// locale。
    pub(super) fn locale(&self) -> StudioLocale {
        self.preferences.locale
    }

    /// text。
    pub(super) fn text(&self, key: UiText) -> &'static str {
        self.locale().text(key)
    }

    /// theme palette。
    pub(super) fn theme_palette(&self) -> StudioPalette {
        StudioTheme::palette(self.theme_mode())
    }

    /// toggle theme mode。
    pub(super) fn toggle_theme_mode(&mut self) {
        self.preferences.theme_mode = self.preferences.theme_mode.next();
    }

    /// toggle locale。
    pub(super) fn toggle_locale(&mut self) {
        self.preferences.locale = self.preferences.locale.next();
    }

    /// theme mode label。
    pub(super) fn theme_mode_label(&self, mode: StudioThemeMode) -> &'static str {
        match self.locale() {
            StudioLocale::English => mode.label(),
            StudioLocale::SimplifiedChinese => mode.label_zh(),
        }
    }
}
