//! Schematic inspector panel — component properties editor.

use super::widgets::inspector_tab;
use crate::app::NekoSpiceApp;
use crate::app::localization::UiText;
use crate::app::theme::StudioTheme;
use eframe::egui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SchematicInspectorPanelState {
    active_tab: SchematicInspectorTab,
}

impl Default for SchematicInspectorPanelState {
    fn default() -> Self {
        Self {
            active_tab: initial_schematic_inspector_tab(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum SchematicInspectorTab {
    #[default]
    Properties,
    Inspector,
    Libraries,
    Simulator,
    Review,
}

impl SchematicInspectorTab {
    const ALL: [Self; 5] = [
        Self::Properties,
        Self::Inspector,
        Self::Libraries,
        Self::Simulator,
        Self::Review,
    ];

    fn label_key(self) -> UiText {
        match self {
            Self::Properties => UiText::Properties,
            Self::Inspector => UiText::SchemaInspector,
            Self::Libraries => UiText::Libraries,
            Self::Simulator => UiText::Simulator,
            Self::Review => UiText::DesignReview,
        }
    }

    fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "properties" => Some(Self::Properties),
            "inspector" | "schema-inspector" => Some(Self::Inspector),
            "libraries" | "library" => Some(Self::Libraries),
            "simulator" | "simulation" => Some(Self::Simulator),
            "review" | "design-review" => Some(Self::Review),
            _ => None,
        }
    }
}

fn initial_schematic_inspector_tab() -> SchematicInspectorTab {
    std::env::var("NEKOSPICE_INITIAL_SCHEMATIC_INSPECTOR")
        .ok()
        .and_then(|value| SchematicInspectorTab::from_slug(&value))
        .unwrap_or_default()
}

impl NekoSpiceApp {
    /// draw schematic inspector panel。
    pub(crate) fn draw_schematic_inspector_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.label(StudioTheme::section_title_for(
            mode,
            self.text(UiText::Inspector),
        ));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::SchematicInspectorCaption),
        ));
        ui.add_space(8.0);
        self.draw_schematic_inspector_tabs(ui);
        ui.add_space(8.0);

        match self.schematic_inspector.active_tab {
            SchematicInspectorTab::Properties => self.draw_schematic_properties_tab(ui),
            SchematicInspectorTab::Inspector => self.draw_schematic_schema_inspector_tab(ui),
            SchematicInspectorTab::Libraries => self.draw_schematic_libraries_tab(ui),
            SchematicInspectorTab::Simulator => self.draw_schematic_simulator_tab(ui),
            SchematicInspectorTab::Review => self.draw_schematic_review_tab(ui),
        }
    }

    fn draw_schematic_inspector_tabs(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.horizontal_wrapped(|ui| {
            for tab in SchematicInspectorTab::ALL {
                let active = self.schematic_inspector.active_tab == tab;
                if inspector_tab(ui, mode, self.text(tab.label_key()), active) {
                    self.schematic_inspector.active_tab = tab;
                }
            }
        });
    }
}
