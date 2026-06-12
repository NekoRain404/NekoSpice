//! 用户偏好管理。存储语言、主题、网格显示等界面设置。
//!
use super::NekoSpiceApp;
use super::localization::{StudioLocale, UiText};
use super::theme::{StudioPalette, StudioTheme, StudioThemeMode};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct StudioPreferences {
    pub(super) theme_mode: StudioThemeMode,
    pub(super) locale: StudioLocale,
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
