//! Schematic workspace module.
//!
//! Contains the schematic editor canvas, toolbar, bottom dock, inspector
//! panel, selection properties, symbol placement controls, and review panel.
//! Sub-modules handle tool state and the property inspector.

pub(crate) mod bottom_dock;
pub(crate) mod toolbar;
pub(crate) mod document_tabs;
pub(crate) mod inspector;
pub(crate) mod review_panel;
pub(crate) mod selection_properties;
pub(crate) mod symbol_placement;
pub(crate) mod tools;
pub(crate) mod workspace;
pub(crate) mod workspace_widgets;

pub(crate) use selection_properties::SelectionPropertyEditorState;
