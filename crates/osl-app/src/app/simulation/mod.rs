//! Simulation workspace module.
//!
//! Architecture:
//! - `analysis` — Structured analysis parameters and step sweep config
//! - `state` — SimulationBackendKind, SimulationPanelState (re-exports from analysis)
//! - `panel` — thin sidebar orchestrator composing editor + controller + display
//! - `directive_editor` — structured analysis parameter editing UI
//! - `run_controller` — profile building, run launch, task polling
//! - `status_display` — run results, log viewer, waveform summary
//! - `workspace` — center workspace with overview and profile editor tabs
//! - `profile_editor` — three-column profile editor layout + SimOptions
//! - `profile_editor_options` — thin orchestrator for options right column
//! - `options_environment` — temperature settings
//! - `options_solver` — transient solver + convergence + output control
//! - `options_ic` — initial conditions (.ic / .nodeset)
//! - `options_status` — run status + recent runs
//! - `profile_editor_sections` — analysis setup + component/model params
//! - `profile_editor_widgets` — shared widget helpers
//! - `waveform_panel`, `artifacts_panel`, `report_panel` — result sub-panels
//! - `workspace_widgets` — shared widget helpers

pub(crate) mod analysis;
pub(crate) mod step_sweep;
pub(crate) mod state;
pub(crate) mod panel;
pub(crate) mod panel_sections;
pub(crate) mod directive_editor;
pub(crate) mod run_controller;
pub(crate) mod status_display;
pub(crate) mod profile_editor;
pub(crate) mod profile_editor_options;
pub(crate) mod options_environment;
pub(crate) mod options_solver;
pub(crate) mod options_ic;
pub(crate) mod options_preset;
pub(crate) mod field_validation;
pub(crate) mod quick_start;
pub(crate) mod options_status;
pub(crate) mod profile_editor_sections;
pub(crate) mod profile_editor_widgets;
pub(crate) mod report_panel;
pub(crate) mod waveform_panel;
pub(crate) mod workspace;
pub(crate) mod artifacts_panel;
pub(crate) mod workspace_sections;
pub(crate) mod history;
pub(crate) mod measure_editor;
pub(crate) mod history_panel;
pub(crate) mod step_sweep_editor;
pub(crate) mod workspace_widgets;

pub(crate) use state::SimulationPanelState;
pub(crate) use history::SimulationHistory;
pub(crate) use profile_editor::SimulationProfileEditorState;
