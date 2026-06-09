use super::NekoSpiceApp;
use super::localization::{StudioLocale, UiText};
use super::theme::{StudioPalette, StudioTheme, StudioThemeMode};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct StudioPreferences {
    pub(super) theme_mode: StudioThemeMode,
    pub(super) locale: StudioLocale,
}

impl NekoSpiceApp {
    pub(super) fn theme_mode(&self) -> StudioThemeMode {
        self.preferences.theme_mode
    }

    pub(super) fn locale(&self) -> StudioLocale {
        self.preferences.locale
    }

    pub(super) fn text(&self, key: UiText) -> &'static str {
        self.locale().text(key)
    }

    pub(super) fn theme_palette(&self) -> StudioPalette {
        StudioTheme::palette(self.theme_mode())
    }

    pub(super) fn toggle_theme_mode(&mut self) {
        self.preferences.theme_mode = self.preferences.theme_mode.next();
    }

    pub(super) fn toggle_locale(&mut self) {
        self.preferences.locale = self.preferences.locale.next();
    }

    pub(super) fn theme_mode_label(&self, mode: StudioThemeMode) -> &'static str {
        match self.locale() {
            StudioLocale::English => mode.label(),
            StudioLocale::SimplifiedChinese => mode.label_zh(),
        }
    }
}
