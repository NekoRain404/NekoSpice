//! Schematic inspector sub-module.
//!
//! Provides the property inspector panel, section renderers,
//! simulator settings, and reusable inspector widgets.

pub(crate) mod panel;
pub(crate) mod sections;
pub(crate) mod simulator;
pub(crate) mod widgets;

pub(crate) use panel::SchematicInspectorPanelState;
