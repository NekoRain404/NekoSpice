//! Home workspace module.
//!
//! Contains the project home dashboard, command center, insights panel,
//! project context summary, and shared home sections/widgets.
//!
//! Sections are split into focused modules:
//! - `sections` — project list, queue, metrics, recommendations
//! - `templates` — template grid with starter circuits
//! - `quick_actions` — 3x3 action button grid

pub(crate) mod command_center;
pub(crate) mod dashboard;
pub(crate) mod insights_panel;
pub(crate) mod project_context;
pub(crate) mod quick_actions;
pub(crate) mod sections;
pub(crate) mod templates;
pub(crate) mod widgets;
