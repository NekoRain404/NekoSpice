/// Schematic workspace: canvas, toolbar, document tabs, and bottom dock.
///
/// The bottom dock switches between Waveforms, FFT, Bode, Console, Netlist,
/// ERC, and Inspector views based on the active tab.
use super::NekoSpiceApp;
use super::SchematicBottomTab;
use super::localization::UiText;
use super::schematic_workspace_widgets::{
    canvas_toolbar_button, document_tab, toolbar_icon_button_active,
};
use super::theme::StudioTheme;
use eframe::egui::{self, CornerRadius, Stroke, Vec2};

impl NekoSpiceApp {
    /// Main entry point for the schematic center workspace.
    pub(super) fn draw_schematic_center_workspace(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            self.draw_schematic_workspace_toolbar(ui);
            ui.add_space(4.0);
            self.draw_schematic_document_tabs(ui);
            ui.add_space(4.0);
            let canvas_height = (ui.available_height() - 220.0).max(280.0);
            let inspector_width = 280.0;
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), canvas_height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    // Vertical tool palette on the left
                    let _palette_width = self.draw_tool_palette(ui);
                    // Main canvas area (occupies remaining width minus inspector)
                    let canvas_width = (ui.available_width() - inspector_width - 8.0).max(200.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(canvas_width, canvas_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.draw_canvas(ui),
                    );
                    // Right-side inspector panel
                    ui.add_space(4.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(inspector_width, canvas_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .max_height(canvas_height)
                                .show(ui, |ui| {
                                    self.draw_schematic_inspector_panel(ui);
                                });
                        },
                    );
                },
            );
            ui.add_space(6.0);
            self.draw_schematic_bottom_dock(ui);
        });
    }

    /// Toolbar row: file ops, drawing tools, zoom, DRC status.
    ///
    /// Drawing tool buttons now switch the active tool in the tool palette.
    /// Zoom buttons provide +/- controls with the current zoom percentage.
    fn draw_schematic_workspace_toolbar(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();

        ui.horizontal(|ui| {
            // File operations group
            if canvas_toolbar_button(ui, mode, "\u{25B6} Open", true).clicked() {
                self.open_file_dialog();
            }
            if canvas_toolbar_button(ui, mode, "\u{2913} Save", self.document.is_some()).clicked() {
                self.save_document_with_dialog();
            }
            ui.add_space(2.0);
            if canvas_toolbar_button(ui, mode, "\u{21A9} Undo", self.history.can_undo()).clicked() {
                self.undo();
            }
            if canvas_toolbar_button(ui, mode, "\u{21AA} Redo", self.history.can_redo()).clicked() {
                self.redo();
            }
            ui.add_space(2.0);
            if canvas_toolbar_button(ui, mode, "\u{2316} Fit", true).clicked() {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }
            if canvas_toolbar_button(ui, mode, "\u{25B6} Run", self.document.is_some())
                .clicked()
            {
                self.run_simulation_from_panel();
            }

            // Visual separator: vertical line
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Drawing tools — clicking switches the active tool
            use super::schematic_tools::SchematicTool;
            let tools: &[(&str, &str, SchematicTool)] = &[
                ("\u{250C}", "Wire (W)", SchematicTool::Wire),
                ("\u{2190}", "Label (L)", SchematicTool::Label),
                ("\u{2550}", "Bus (B)", SchematicTool::Bus),
                ("\u{25A3}", "Sheet (S)", SchematicTool::Sheet),
                ("\u{2B24}", "Junction (J)", SchematicTool::Junction),
                ("\u{2716}", "NoConn (Q)", SchematicTool::NoConnect),
            ];
            for &(icon, tooltip, tool) in tools {
                let is_active = self.schematic_tools.active == tool;
                if toolbar_icon_button_active(ui, mode, icon, tooltip, true, is_active).clicked() {
                    self.activate_schematic_tool_direct(tool);
                }
            }

            // Visual separator
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Zoom controls — zoom around canvas center
            if canvas_toolbar_button(ui, mode, "-", true).clicked() {
                if let Some(rect) = self.last_canvas_rect {
                    self.viewport.zoom_around(rect, rect.center(), 0.8);
                } else {
                    self.viewport.zoom = (self.viewport.zoom * 0.8).max(1.0);
                }
            }
            ui.label(StudioTheme::accent_for(
                mode,
                format!("{:.0}%", self.viewport.zoom * 100.0),
            ));
            if canvas_toolbar_button(ui, mode, "+", true).clicked() {
                if let Some(rect) = self.last_canvas_rect {
                    self.viewport.zoom_around(rect, rect.center(), 1.25);
                } else {
                    self.viewport.zoom = (self.viewport.zoom * 1.25).min(180.0);
                }
            }

            // Visual separator
            ui.add_space(6.0);
            ui.separator();

            // Backend indicator
            ui.label(StudioTheme::muted_for(mode, self.simulation_panel.backend.label()));
            ui.separator();

            // DRC status
            ui.label(StudioTheme::muted_for(mode, "DRC"));
            let report = self
                .document
                .as_ref()
                .map(|doc| doc.check_report());
            let (dot_color, drc_text) = match report {
                Some(r) if r.error_count() > 0 => (palette.danger, format!("{} errors", r.error_count())),
                Some(r) if r.warning_count() > 0 => (palette.warning, format!("{} warnings", r.warning_count())),
                Some(_) => (palette.success, "Clean".to_string()),
                None => (palette.text_muted, "No doc".to_string()),
            };
            ui.label(
                egui::RichText::new(format!("\u{25CF} {drc_text}"))
                    .color(dot_color)
                    .size(12.0),
            );

            // Push status message to the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(msg) = &self.status_message {
                    ui.label(StudioTheme::accent_for(mode, msg));
                }
            });
        });
    }

    /// Document tab bar: shows loaded schematic and placeholder sub-sheets.
    fn draw_schematic_document_tabs(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        ui.horizontal_wrapped(|ui| {
            let active_name = self
                .document
                .as_ref()
                .and_then(|document| document.path().file_name())
                .and_then(|name| name.to_str())
                .unwrap_or(self.text(UiText::NoDocument));
            if document_tab(ui, mode, active_name, true).clicked() {
                self.status_message = Some(format!("Root schematic: {active_name}"));
            }
            // Show real sub-sheets from loaded schematic — clicking opens the sub-sheet file.
            if let Some(document) = &self.document {
                let scene = document.scene();
                let sheets: Vec<_> = scene.sheets.iter().map(|s| {
                    (s.name.clone(), s.file.clone())
                }).collect();
                for (name, file) in sheets {
                    let tab_label = if file.is_empty() { name.clone() } else { file.clone() };
                    if document_tab(ui, mode, &tab_label, false).clicked() {
                        // Try to load the sub-sheet schematic
                        if !file.is_empty() {
                            if let Some(document) = &self.document {
                                let base_dir = document.path().parent()
                                    .map(|p| p.to_path_buf())
                                    .unwrap_or_default();
                                let sub_path = base_dir.join(&file);
                                if sub_path.exists() {
                                    self.load_schematic(sub_path);
                                    self.status_message = Some(format!("Opened sub-sheet: {tab_label}"));
                                } else {
                                    self.status_message = Some(format!("Sub-sheet not found: {tab_label}"));
                                }
                            }
                        } else {
                            self.status_message = Some(format!("Sub-sheet: {name}"));
                        }
                    }
                }
            }
            if ui.add(egui::Button::new(egui::RichText::new("+").size(14.0).strong().color(palette.text)).fill(palette.panel_soft).stroke(egui::Stroke::new(1.0, palette.border_strong))).on_hover_text(self.text(UiText::NewSchematic)).clicked() {
                self.open_file_dialog();
            }
        });
    }

    /// Bottom dock panel with tab switching between views.
    fn draw_schematic_bottom_dock(&mut self, ui: &mut egui::Ui) {
        let palette = self.theme_palette();
        let current_tab = self.schematic_bottom_tab;

        egui::Frame::new()
            .fill(palette.panel_soft)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                // Tab bar
                ui.horizontal_wrapped(|ui| {
                    let tab_defs: &[(SchematicBottomTab, &str)] = &[
                        (SchematicBottomTab::Waveforms, self.text(UiText::Waveforms)),
                        (SchematicBottomTab::Fft, "FFT"),
                        (SchematicBottomTab::Bode, "Bode"),
                        (SchematicBottomTab::Console, self.text(UiText::StatusConsole)),
                        (SchematicBottomTab::Netlist, self.text(UiText::Netlist)),
                        (SchematicBottomTab::Erc, "ERC"),
                        (SchematicBottomTab::Inspector, self.text(UiText::Inspector)),
                    ];
                    for &(tab, label) in tab_defs {
                        let is_active = current_tab == tab;
                        let fill = if is_active {
                            palette.accent_soft
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let text_color = if is_active {
                            palette.accent
                        } else {
                            palette.text_muted
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(label)
                                        .size(12.0)
                                        .color(text_color),
                                )
                                .fill(fill)
                                .stroke(if is_active {
                                    Stroke::new(1.0, palette.accent)
                                } else {
                                    Stroke::NONE
                                })
                                .corner_radius(CornerRadius::same(4)),
                            )
                            .clicked()
                        {
                            self.schematic_bottom_tab = tab;
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                match current_tab {
                    SchematicBottomTab::Waveforms => self.draw_bottom_waveforms_tab(ui),
                    SchematicBottomTab::Fft => self.draw_bottom_fft_tab(ui),
                    SchematicBottomTab::Bode => self.draw_bottom_bode_tab(ui),
                    SchematicBottomTab::Console => self.draw_bottom_console_tab(ui),
                    SchematicBottomTab::Netlist => self.draw_bottom_netlist_tab(ui),
                    SchematicBottomTab::Erc => self.draw_bottom_erc_tab(ui),
                    SchematicBottomTab::Inspector => self.draw_bottom_inspector_tab(ui),
                }
            });
    }

}