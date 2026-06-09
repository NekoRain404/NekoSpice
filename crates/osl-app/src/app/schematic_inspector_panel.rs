use super::NekoSpiceApp;
use super::localization::UiText;
use super::schematic_inspector_widgets::inspector_tab;
use super::theme::StudioTheme;
use eframe::egui;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct SchematicInspectorPanelState {
    active_tab: SchematicInspectorTab,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum SchematicInspectorTab {
    #[default]
    Properties,
    Inspector,
    Libraries,
    Simulator,
}

impl SchematicInspectorTab {
    const ALL: [Self; 4] = [
        Self::Properties,
        Self::Inspector,
        Self::Libraries,
        Self::Simulator,
    ];

    fn label_key(self) -> UiText {
        match self {
            Self::Properties => UiText::Properties,
            Self::Inspector => UiText::KiCadInspector,
            Self::Libraries => UiText::Libraries,
            Self::Simulator => UiText::Simulator,
        }
    }
}

impl NekoSpiceApp {
    pub(super) fn draw_schematic_inspector_panel(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        ui.heading(self.text(UiText::Inspector));
        ui.label(StudioTheme::muted_for(
            mode,
            self.text(UiText::SchematicInspectorCaption),
        ));
        ui.add_space(8.0);
        self.draw_schematic_inspector_tabs(ui);
        ui.add_space(8.0);

        match self.schematic_inspector.active_tab {
            SchematicInspectorTab::Properties => self.draw_schematic_properties_tab(ui),
            SchematicInspectorTab::Inspector => self.draw_schematic_kicad_inspector_tab(ui),
            SchematicInspectorTab::Libraries => self.draw_schematic_libraries_tab(ui),
            SchematicInspectorTab::Simulator => self.draw_schematic_simulator_tab(ui),
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
