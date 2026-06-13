//! Optimization workspace state — active tab, editable targets, sweep
//! parameters, Monte Carlo configuration, and distribution results.
//!
//! All editable fields are stored as strings so the user can type
//! SPICE-style values directly. The actual numeric parsing happens
//! at run time when the user triggers a sweep or Monte Carlo analysis.

use crate::app::localization::UiText;

/// Active sub-tab in the optimization workspace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum OptimizationTab {
    /// Define optimization targets (minimize/maximize measurements).
    Targets,
    /// Configure parametric sweep ranges.
    Sweep,
    /// Monte Carlo analysis with tolerance distributions.
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

/// An editable optimization target (minimize or maximize a measurement).
#[derive(Debug, Clone)]
pub(crate) struct OptimizationTarget {
    pub name: String,
    pub goal: String,
    pub constraint: String,
}

impl Default for OptimizationTarget {
    fn default() -> Self {
        Self {
            name: String::new(),
            goal: "minimize".to_string(),
            constraint: String::new(),
        }
    }
}

/// An editable parametric sweep parameter.
#[derive(Debug, Clone)]
pub(crate) struct SweepParam {
    pub name: String,
    pub start: String,
    pub stop: String,
    pub count: String,
}

impl Default for SweepParam {
    fn default() -> Self {
        Self {
            name: String::new(),
            start: String::new(),
            stop: String::new(),
            count: "10".to_string(),
        }
    }
}

/// A Monte Carlo parameter with tolerance distribution.
#[derive(Debug, Clone)]
pub(crate) struct MonteCarloParam {
    pub name: String,
    pub nominal: String,
    pub distribution: String,
    pub tolerance: String,
}

impl Default for MonteCarloParam {
    fn default() -> Self {
        Self {
            name: String::new(),
            nominal: String::new(),
            distribution: "Gaussian".to_string(),
            tolerance: "1%".to_string(),
        }
    }
}

/// A Monte Carlo response measurement with pass/fail spec.
#[derive(Debug, Clone, Default)]
pub(crate) struct MonteCarloMeasurement {
    pub name: String,
    pub kind: String,
    pub spec: String,
}

/// Full state for the optimization workspace.
#[derive(Debug, Default)]
pub(crate) struct OptimizationWorkspaceState {
    /// Currently active sub-tab.
    pub(crate) active_tab: OptimizationTab,

    // ── Targets ────────────────────────────────────────
    /// Editable optimization targets.
    pub(crate) targets: Vec<OptimizationTarget>,

    // ── Sweep ──────────────────────────────────────────
    /// Editable sweep parameters.
    pub(crate) sweep_params: Vec<SweepParam>,

    // ── Monte Carlo ────────────────────────────────────
    /// Editable MC parameter definitions.
    pub(crate) mc_params: Vec<MonteCarloParam>,
    /// Editable MC response measurements.
    pub(crate) mc_measurements: Vec<MonteCarloMeasurement>,
    /// MC sample count (editable).
    pub(crate) mc_sample_count: String,

    // ── Results (populated after runs) ─────────────────
    /// Number of MC runs completed.
    pub(crate) mc_completed: usize,
    /// Number of MC runs that passed.
    pub(crate) mc_passed: usize,
}
