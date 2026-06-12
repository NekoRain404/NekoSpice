use crate::app::localization::UiText;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum OptimizationTab {
    Targets,
    Sweep,
    #[default]
    MonteCarlo,
}

impl OptimizationTab {
    pub(crate) const ALL: [Self; 3] = [Self::Targets, Self::Sweep, Self::MonteCarlo];

    pub(crate) fn text_key(self) -> UiText {
        match self {
            Self::Targets => UiText::Optimization,
            Self::Sweep => UiText::ParametricSweep,
            Self::MonteCarlo => UiText::MonteCarlo,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct OptimizationWorkspaceState {
    pub(crate) active_tab: OptimizationTab,
}
