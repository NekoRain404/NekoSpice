//! Waveform viewer module.
//!
//! Contains the waveform workspace, analysis tabs, measurement table,
//! preview canvas, and drawing primitives for waveform visualization.
//!
//! File structure:
//! - `workspace.rs` — workspace state, viewport, analysis tabs
//! - `workspace_sections.rs` — measurement tables, trace chips, plot dispatch
//! - `workspace_sidepanels.rs` — cursor readout, run comparison, export panels
//! - `workspace_widgets.rs` — UI component helpers (tabs, chips, cards)
//! - `preview.rs` — static single/stacked waveform preview
//! - `interactive.rs` — interactive zoom/pan/cursor waveform plot
//! - `preview_primitives.rs` — low-level drawing primitives (grid, buckets)
//! - `freq_domain_primitives.rs` — frequency-domain drawing helpers
//! - `freq_domain_preview.rs` — FFT/Bode/Noise frequency-domain plots
//! - `helpers.rs` — shared helper functions (trace ordering, labels)

pub(crate) mod helpers;
pub(crate) mod preview;
pub(crate) mod preview_primitives;
pub(crate) mod interactive;
pub(crate) mod freq_domain_primitives;
pub(crate) mod freq_domain_preview;
pub(crate) mod workspace;
pub(crate) mod workspace_sections;
pub(crate) mod workspace_measurements;
pub(crate) mod workspace_sidepanels;
pub(crate) mod workspace_widgets;

pub(crate) use workspace::WaveformWorkspaceState;
