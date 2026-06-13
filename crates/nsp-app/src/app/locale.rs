//! 语言区域定义。管理界面语言选择和语言切换逻辑。

use super::localization::UiText;

/// 支持的界面语言。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum StudioLocale {
    #[default]
    English,
    SimplifiedChinese,
}

impl StudioLocale {
    /// 从字符串解析语言标识（"zh" / "Chinese" → 简体中文，其余 → 英文）。
    pub(super) fn from_str(s: &str) -> Self {
        match s {
            "zh" | "Chinese" => Self::SimplifiedChinese,
            _ => Self::English,
        }
    }

    /// 返回 ISO 语言代码。
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh",
        }
    }

    /// 所有支持的语言列表。
    pub(super) const ALL: [Self; 2] = [Self::English, Self::SimplifiedChinese];

    /// 语言的本地化名称。
    pub(super) fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
        }
    }

    /// 用于 UI 按钮的短标签。
    pub(super) fn short_code(self) -> &'static str {
        match self {
            Self::English => "EN",
            Self::SimplifiedChinese => "中",
        }
    }

    /// 切换到下一个语言（循环）。
    pub(super) fn next(self) -> Self {
        match self {
            Self::English => Self::SimplifiedChinese,
            Self::SimplifiedChinese => Self::English,
        }
    }

    /// 获取指定文本键在当前语言下的翻译。
    pub(super) fn text(self, key: UiText) -> &'static str {
        match self {
            Self::English => key.en(),
            Self::SimplifiedChinese => key.zh_hans(),
        }
    }
}
