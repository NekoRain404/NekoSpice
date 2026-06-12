//! Simulation workspace module.
//!
//! Architecture:
//! - `state` — SimulationBackendKind, SimulationPanelState
//! - `panel` — thin sidebar orchestrator composing editor + controller + display
//! - `directive_editor` — directive kind/body editing UI
//! - `run_controller` — profile building, run launch, task polling
//! - `status_display` — run results, log viewer, waveform summary
//! - `workspace` — center workspace with overview and profile editor tabs
//! - `profile_editor*` — three-column profile editor layout
//! - `waveform_panel`, `artifacts_panel`, `report_panel` — result sub-panels
//! - `workspace_widgets` — shared widget helpers

pub(crate) mod state;
pub(crate) mod panel;
pub(crate) mod directive_editor;
pub(crate) mod run_controller;
pub(crate) mod status_display;
pub(crate) mod profile_editor;
pub(crate) mod profile_editor_options;
pub(crate) mod profile_editor_sections;
pub(crate) mod profile_editor_widgets;
pub(crate) mod report_panel;
pub(crate) mod waveform_panel;
pub(crate) mod workspace;
pub(crate) mod artifacts_panel;
pub(crate) mod workspace_sections;
pub(crate) mod workspace_widgets;

pub(crate) use state::SimulationPanelState;
pub(crate) use profile_editor::SimulationProfileEditorState;
