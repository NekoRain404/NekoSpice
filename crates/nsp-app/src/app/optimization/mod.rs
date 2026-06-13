//! Optimization workspace module.
//!
//! Contains the parameter optimization workspace, optimization sections,
//! optimization state (tabs, parameter sweeps), and shared widgets.

pub(crate) mod sections;
pub(crate) mod state;
pub(crate) mod widgets;
pub(crate) mod workspace;

pub(crate) use state::OptimizationWorkspaceState;
