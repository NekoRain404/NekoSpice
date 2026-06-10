use super::localization::UiText;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum OptimizationTab {
    Targets,
    Sweep,
    #[default]
    MonteCarlo,
}

impl OptimizationTab {
    pub(super) const ALL: [Self; 3] = [Self::Targets, Self::Sweep, Self::MonteCarlo];

    pub(super) fn text_key(self) -> UiText {
        match self {
            Self::Targets => UiText::Optimization,
            Self::Sweep => UiText::ParametricSweep,
            Self::MonteCarlo => UiText::MonteCarlo,
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct OptimizationWorkspaceState {
    pub(super) active_tab: OptimizationTab,
}
