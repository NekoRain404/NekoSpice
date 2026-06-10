use super::localization::UiText;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ReportsTab {
    Overview,
    #[default]
    Measurements,
    Plots,
    Builder,
    Templates,
    ExportHistory,
}

impl ReportsTab {
    pub(super) const ALL: [Self; 6] = [
        Self::Overview,
        Self::Measurements,
        Self::Plots,
        Self::Builder,
        Self::Templates,
        Self::ExportHistory,
    ];

    pub(super) fn text_key(self) -> UiText {
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
pub(super) struct ReportsWorkspaceState {
    pub(super) active_tab: ReportsTab,
}
