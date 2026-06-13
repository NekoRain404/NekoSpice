//! Schematic → simulation panel synchronization.
//!
//! When a schematic is loaded, these methods extract simulation directives
//! (`.tran`, `.ac`, `.dc`, `.step`) and component values (R, C, L, V, I)
//! from the schematic and populate the simulation panel state. This makes
//! the simulation workflow seamless — open a schematic and the panel is
//! pre-configured with the right analysis type and component parameters.

use crate::app::NekoSpiceApp;
use crate::app::simulation::analysis::{AnalysisParams, StepSweep};
use nsp_schema::NspSimulationDirectiveKind;

impl NekoSpiceApp {
    /// Read simulation directives from the loaded schematic and update
    /// the simulation panel state (analysis kind, body, step sweep, etc.).
    ///
    /// Called during `load_schematic()` so the simulation panel always
    /// reflects what's actually in the schematic file.
    pub(super) fn sync_sim_panel_from_schematic(&mut self) {
        let Some(document) = &self.document else {
            return;
        };
        let directives = document.simulation_directives();
        if directives.is_empty() {
            return;
        }

        // Find the first analysis directive
        for directive in &directives {
            match directive.kind {
                NspSimulationDirectiveKind::Tran
                | NspSimulationDirectiveKind::Ac
                | NspSimulationDirectiveKind::Dc
                | NspSimulationDirectiveKind::Op
                | NspSimulationDirectiveKind::Noise
                | NspSimulationDirectiveKind::Disto
                | NspSimulationDirectiveKind::Sens => {
                    self.simulation_panel.directive_kind = directive.kind;
                    let body = directive
                        .text
                        .trim()
                        .strip_prefix(directive.kind.keyword().unwrap_or(""))
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    self.simulation_panel.analysis_params =
                        AnalysisParams::for_kind(directive.kind);
                    self.simulation_panel.analysis_params.parse_body(&body);
                    break;
                }
                _ => {}
            }
        }

        // Look for .step directive
        for directive in &directives {
            if directive.kind == NspSimulationDirectiveKind::Step {
                let body = directive
                    .text
                    .trim()
                    .strip_prefix(".step")
                    .unwrap_or("")
                    .trim();
                self.simulation_panel.step_sweep = StepSweep::from_directive_body(body);
            }
        }

        // Auto-populate component parameters from schematic
        self.auto_populate_component_params();
    }

    /// Auto-populate component parameters from the loaded schematic.
    ///
    /// Scans all symbol instances for passive components (R, C, L) and
    /// independent sources (V, I), extracting their Reference and Value
    /// properties into the simulation profile's component_params table.
    pub(super) fn auto_populate_component_params(&mut self) {
        let Some(document) = &self.document else {
            return;
        };
        let schematic = &document.schematic;
        self.simulation_profile_editor.component_params.clear();

        for symbol in &schematic.symbols {
            let lib_id = &symbol.lib_id;
            let reference = symbol.reference().unwrap_or("");
            let value = symbol.property("Value").unwrap_or("");

            if reference.is_empty() || reference.starts_with('#') {
                continue;
            }

            let kind = classify_spice_component(lib_id);
            if kind.is_empty() {
                continue;
            }

            let unit = match kind {
                "R" => "ohm",
                "C" => "F",
                "L" => "H",
                "V" => "V",
                "I" => "A",
                _ => "",
            };

            self.simulation_profile_editor.component_params.push((
                reference.to_string(),
                value.to_string(),
                unit.to_string(),
            ));
        }
    }
}

/// Classify a schema lib_id into its SPICE component prefix.
fn classify_spice_component(lib_id: &str) -> &'static str {
    let lower = lib_id.to_lowercase();
    if lower.contains(":r") || lower == "device:r" {
        "R"
    } else if lower.contains(":c") || lower == "device:c" {
        "C"
    } else if lower.contains(":l") || lower == "device:l" {
        "L"
    } else if lower.starts_with("power:") {
        ""
    } else if lower.contains(":v") || lower.contains("voltage") {
        "V"
    } else if lower.contains(":i") || lower.contains("current") {
        "I"
    } else if lower.contains(":d") || lower.contains("diode") {
        "D"
    } else if lower.contains(":q") || lower.contains("transistor") {
        "Q"
    } else if lower.contains(":op") || lower.contains("opamp") {
        "U"
    } else {
        let parts: Vec<&str> = lower.split(':').collect();
        if let Some(name) = parts.last() {
            if name.starts_with('r') && name.len() <= 3 {
                "R"
            } else if name.starts_with('c') && name.len() <= 3 {
                "C"
            } else if name.starts_with('l') && name.len() <= 3 {
                "L"
            } else {
                ""
            }
        } else {
            ""
        }
    }
}
