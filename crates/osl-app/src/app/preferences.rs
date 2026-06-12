//! 用户偏好管理。存储语言、主题、网格显示等界面设置。
//!
//! 偏好设置持久化到 `~/.config/nekospice/settings.json`。
//! 启动时自动加载，修改时自动保存。

use super::NekoSpiceApp;
use super::localization::{StudioLocale, UiText};
use super::theme::{StudioPalette, StudioTheme, StudioThemeMode};
use std::fs;
use std::path::PathBuf;

/// 持久化到磁盘的偏好设置 JSON 结构。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SettingsFile {
    theme_mode: String,
    locale: String,
    ngspice_path: String,
    xyce_path: String,
}

impl Default for SettingsFile {
    fn default() -> Self {
        Self {
            theme_mode: "Dark".to_string(),
            locale: "en".to_string(),
            ngspice_path: "ngspice".to_string(),
            xyce_path: "xyce".to_string(),
        }
    }
}

/// 获取设置文件路径：`~/.config/nekospice/settings.json`
fn settings_path() -> PathBuf {
    dirs_or_fallback()
        .join("nekospice")
        .join("settings.json")
}

/// 优先使用 XDG_CONFIG_HOME，回退到 HOME/.config
fn dirs_or_fallback() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config"))
                .unwrap_or_else(|| PathBuf::from("."))
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StudioPreferences {
    pub(super) theme_mode: StudioThemeMode,
    pub(super) locale: StudioLocale,
    /// ngspice 可执行文件路径
    pub(super) ngspice_path: String,
    /// Xyce 可执行文件路径
    pub(super) xyce_path: String,
}

impl Default for StudioPreferences {
    fn default() -> Self {
        // 尝试从磁盘加载，失败则使用硬编码默认值
        Self::load_from_disk().unwrap_or_else(|| Self {
            theme_mode: StudioThemeMode::default(),
            locale: StudioLocale::default(),
            ngspice_path: "ngspice".to_string(),
            xyce_path: "xyce".to_string(),
        })
    }
}

impl StudioPreferences {
    /// 从磁盘加载偏好设置。
    fn load_from_disk() -> Option<Self> {
        let path = settings_path();
        let data = fs::read_to_string(&path).ok()?;
        let file: SettingsFile = serde_json::from_str(&data).ok()?;
        Some(Self {
            theme_mode: StudioThemeMode::from_str(&file.theme_mode),
            locale: StudioLocale::from_str(&file.locale),
            ngspice_path: file.ngspice_path,
            xyce_path: file.xyce_path,
        })
    }

    /// 保存当前偏好设置到磁盘。
    pub(super) fn save_to_disk(&self) {
        let file = SettingsFile {
            theme_mode: self.theme_mode.as_str().to_string(),
            locale: self.locale.as_str().to_string(),
            ngspice_path: self.ngspice_path.clone(),
            xyce_path: self.xyce_path.clone(),
        };
        let path = settings_path();
        // 创建目录（忽略已存在错误）
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&file) {
            let _ = fs::write(&path, json);
        }
    }
}

impl NekoSpiceApp {
    /// 当前主题模式
    pub(super) fn theme_mode(&self) -> StudioThemeMode {
        self.preferences.theme_mode
    }

    /// 当前语言
    pub(super) fn locale(&self) -> StudioLocale {
        self.preferences.locale
    }

    /// 根据当前语言获取 UI 文本
    pub(super) fn text(&self, key: UiText) -> &'static str {
        self.locale().text(key)
    }

    /// 获取当前主题调色板
    pub(super) fn theme_palette(&self) -> StudioPalette {
        StudioTheme::palette(self.theme_mode())
    }

    /// 切换主题模式并保存
    pub(super) fn toggle_theme_mode(&mut self) {
        self.preferences.theme_mode = self.preferences.theme_mode.next();
        self.preferences.save_to_disk();
    }

    /// 切换语言并保存
    pub(super) fn toggle_locale(&mut self) {
        self.preferences.locale = self.preferences.locale.next();
        self.preferences.save_to_disk();
    }

    /// 主题模式显示文本
    pub(super) fn theme_mode_label(&self, mode: StudioThemeMode) -> &'static str {
        match self.locale() {
            StudioLocale::English => mode.label(),
            StudioLocale::SimplifiedChinese => mode.label_zh(),
        }
    }
}
