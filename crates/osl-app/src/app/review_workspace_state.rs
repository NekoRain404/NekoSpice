use super::localization::StudioLocale;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ReviewWorkspaceState {
    pub(super) severity_filter: ReviewSeverityFilter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ReviewSeverityFilter {
    #[default]
    All,
    Critical,
    Major,
    Minor,
}

impl ReviewSeverityFilter {
    pub(super) const ALL: [Self; 4] = [Self::All, Self::Critical, Self::Major, Self::Minor];

    pub(super) fn label(self, locale: StudioLocale) -> &'static str {
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

    pub(super) fn matches(self, severity: ReviewSeverity) -> bool {
        match self {
            Self::All => true,
            Self::Critical => severity == ReviewSeverity::Critical,
            Self::Major => severity == ReviewSeverity::Major,
            Self::Minor => severity == ReviewSeverity::Minor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReviewSeverity {
    Critical,
    Major,
    Minor,
}

impl ReviewSeverity {
    pub(super) fn label(self, locale: StudioLocale) -> &'static str {
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
