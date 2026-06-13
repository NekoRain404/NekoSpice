//! Reports workspace state — active report selection and filter state.

use crate::app::localization::UiText;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ReportsTab {
    Overview,
    #[default]
    Measurements,
    Plots,
    Builder,
    Templates,
    ExportHistory,
}

impl ReportsTab {
    pub(crate) const ALL: [Self; 6] = [
        Self::Overview,
        Self::Measurements,
        Self::Plots,
        Self::Builder,
        Self::Templates,
        Self::ExportHistory,
    ];

    /// text key。
    pub(crate) fn text_key(self) -> UiText {
        match self {
            Self::Overview => UiText::Overview,
            Self::Measurements => UiText::Measurements,
            Self::Plots => UiText::Plots,
            Self::Builder => UiText::ReportBuilder,
            Self::Templates => UiText::Templates,
            Self::ExportHistory => UiText::ExportHistory,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct ReportsWorkspaceState {
    pub(crate) active_tab: ReportsTab,
}
