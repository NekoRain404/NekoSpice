#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum StudioWorkspace {
    #[default]
    Schematic,
    Library,
    Simulation,
    Reports,
}

impl StudioWorkspace {
    pub(super) const ALL: [Self; 4] = [
        Self::Schematic,
        Self::Library,
        Self::Simulation,
        Self::Reports,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Schematic => "Schematic",
            Self::Library => "Library",
            Self::Simulation => "Simulation",
            Self::Reports => "Reports",
        }
    }

    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::Schematic => "SCH",
            Self::Library => "LIB",
            Self::Simulation => "SIM",
            Self::Reports => "RPT",
        }
    }

    pub(super) fn caption(self) -> &'static str {
        match self {
            Self::Schematic => "Edit KiCad-compatible sheets",
            Self::Library => "Browse symbols and placement scope",
            Self::Simulation => "Run ngspice and inspect outputs",
            Self::Reports => "Review artifacts, waveforms, reports",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StudioWorkspace;

    #[test]
    fn studio_workspaces_have_stable_labels() {
        assert_eq!(StudioWorkspace::ALL.len(), 4);
        assert_eq!(StudioWorkspace::Schematic.label(), "Schematic");
        assert!(!StudioWorkspace::Simulation.caption().is_empty());
    }
}
