use crate::app::localization::StudioLocale;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ReviewWorkspaceState {
    pub(crate) severity_filter: ReviewSeverityFilter,
    pub(crate) checklist_tab: ReviewChecklistTab,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ReviewChecklistTab {
    #[default]
    Readiness,
    Electrical,
    Models,
}

impl ReviewChecklistTab {
    pub(crate) const ALL: [Self; 3] = [Self::Readiness, Self::Electrical, Self::Models];

    /// label。
    pub(crate) fn label(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => match self {
                Self::Readiness => "Readiness",
                Self::Electrical => "Electrical",
                Self::Models => "Models",
            },
            StudioLocale::SimplifiedChinese => match self {
                Self::Readiness => "就绪度",
                Self::Electrical => "电气",
                Self::Models => "模型",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ReviewSeverityFilter {
    #[default]
    All,
    Critical,
    Major,
    Minor,
}

impl ReviewSeverityFilter {
    pub(crate) const ALL: [Self; 4] = [Self::All, Self::Critical, Self::Major, Self::Minor];

    /// label。
    pub(crate) fn label(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => match self {
                Self::All => "All",
                Self::Critical => "Critical",
                Self::Major => "Major",
                Self::Minor => "Minor",
            },
            StudioLocale::SimplifiedChinese => match self {
                Self::All => "全部",
                Self::Critical => "严重",
                Self::Major => "主要",
                Self::Minor => "轻微",
            },
        }
    }

    /// matches。
    pub(crate) fn matches(self, severity: ReviewSeverity) -> bool {
        match self {
            Self::All => true,
            Self::Critical => severity == ReviewSeverity::Critical,
            Self::Major => severity == ReviewSeverity::Major,
            Self::Minor => severity == ReviewSeverity::Minor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewSeverity {
    Critical,
    Major,
    Minor,
}

impl ReviewSeverity {
    /// label。
    pub(crate) fn label(self, locale: StudioLocale) -> &'static str {
        match locale {
            StudioLocale::English => match self {
                Self::Critical => "Critical",
                Self::Major => "Major",
                Self::Minor => "Minor",
            },
            StudioLocale::SimplifiedChinese => match self {
                Self::Critical => "严重",
                Self::Major => "主要",
                Self::Minor => "轻微",
            },
        }
    }
}
