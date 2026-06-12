//! Waveform viewer module.
//!
//! Contains the waveform workspace, analysis tabs, measurement table,
//! preview canvas, and drawing primitives for waveform visualization.

pub(crate) mod preview;
pub(crate) mod preview_primitives;
pub(crate) mod workspace;
pub(crate) mod workspace_sections;
pub(crate) mod workspace_widgets;

pub(crate) use workspace::WaveformWorkspaceState;
