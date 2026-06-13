//! Bottom dock panel for the schematic workspace.
//!
//! Dispatches tab rendering to focused sub-modules:
//! - [`waveforms`] — Waveforms, FFT, Bode tabs
//! - [`debug`] — Console, Netlist, ERC, Inspector tabs

mod debug;
mod waveforms;

use crate::app::NekoSpiceApp;
use crate::app::SchematicBottomTab;
use crate::app::localization::UiText;
use eframe::egui::{self, CornerRadius, Stroke};

impl NekoSpiceApp {
    /// 底部停靠面板：在不同视图之间切换标签页。
    pub(crate) fn draw_schematic_bottom_dock(&mut self, ui: &mut egui::Ui) {
        let palette = self.theme_palette();
        let current_tab = self.schematic_bottom_tab;

        egui::Frame::new()
            .fill(palette.panel_soft)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                // 标签页栏
                ui.horizontal_wrapped(|ui| {
                    let tab_defs: &[(SchematicBottomTab, &str)] = &[
                        (SchematicBottomTab::Waveforms, self.text(UiText::Waveforms)),
                        (SchematicBottomTab::Fft, "FFT"),
                        (SchematicBottomTab::Bode, "Bode"),
                        (
                            SchematicBottomTab::Console,
                            self.text(UiText::StatusConsole),
                        ),
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
                                    egui::RichText::new(label).size(12.0).color(text_color),
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
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);

                // 分发到对应的标签页渲染
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
