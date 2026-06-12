//! Review workspace module.
//!
//! Contains the design review workspace, checklist management,
//! review state (severity filters, findings), and shared widgets.

pub(crate) mod checklist;
pub(crate) mod state;
pub(crate) mod widgets;
pub(crate) mod workspace;

pub(crate) use state::ReviewWorkspaceState;
