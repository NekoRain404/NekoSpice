//! Reports workspace module.
//!
//! Contains the reports workspace, measurement summaries,
//! report preview, report sections, report state, and shared widgets.

pub(crate) mod measurements;
pub(crate) mod preview;
pub(crate) mod sections;
pub(crate) mod state;
pub(crate) mod widgets;
pub(crate) mod workspace;
pub(crate) mod export;

pub(crate) use state::ReportsWorkspaceState;
