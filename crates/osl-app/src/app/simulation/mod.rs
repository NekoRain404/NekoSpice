//! Simulation workspace module.
//!
//! Contains the simulation panel (backend selector, node list, netlist),
//! profile editor (analysis types, options, sections, widgets),
//! workspace view, artifacts panel, report panel, and waveform panel.

pub(crate) mod artifacts_panel;
pub(crate) mod panel;
pub(crate) mod profile_editor;
pub(crate) mod profile_editor_options;
pub(crate) mod profile_editor_sections;
pub(crate) mod profile_editor_widgets;
pub(crate) mod report_panel;
pub(crate) mod waveform_panel;
pub(crate) mod workspace;
pub(crate) mod workspace_sections;
pub(crate) mod workspace_widgets;

pub(crate) use panel::SimulationPanelState;
pub(crate) use profile_editor::SimulationProfileEditorState;
