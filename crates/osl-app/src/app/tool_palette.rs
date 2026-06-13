//! 垂直工具面板。在原理图工作区左侧绘制可切换的绘图工具图标列。
//!
use super::NekoSpiceApp;
use super::schematic::tools::SchematicTool;
// StudioThemeMode used below for palette access
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

/// Tool button definition: (icon, tooltip, tool).
struct ToolDef(&'static str, &'static str, SchematicTool);

/// Returns the list of tools displayed in the vertical palette.
fn tool_palette_items() -> Vec<ToolDef> {
    vec![
        ToolDef("\u{27A1}", "Select (V)", SchematicTool::Select),
        ToolDef("\u{250C}", "Wire (W)", SchematicTool::Wire),
        ToolDef("\u{2550}", "Bus (B)", SchematicTool::Bus),
        ToolDef("\u{2570}", "Bus Entry (E)", SchematicTool::BusEntry),
        ToolDef("\u{1F517}", "Net Label (L)", SchematicTool::Label),
        ToolDef("\u{1F3F0}", "Global Label (G)", SchematicTool::GlobalLabel),
        ToolDef("\u{1F3E0}", "Hierarchical Label (H)", SchematicTool::HierarchicalLabel),
        ToolDef("\u{25A3}", "Sheet Symbol (S)", SchematicTool::Sheet),
        ToolDef("\u{1F4DD}", "Text (T)", SchematicTool::Text),
        ToolDef("\u{2B24}", "Junction (J)", SchematicTool::Junction),
        ToolDef("\u{2716}", "No Connect (Q)", SchematicTool::NoConnect),
    ]
}

impl NekoSpiceApp {
    /// Draw the vertical tool palette on the left side of the schematic canvas.
    ///
    /// Each tool gets a square button with an icon. The active tool is highlighted
    /// with an accent background and left bar indicator. Returns the width allocated.
    pub(crate) fn draw_tool_palette(&mut self, ui: &mut egui::Ui) -> f32 {
        let palette = self.theme_palette();
        let button_size = 32.0;
        let panel_width = 38.0;

        egui::Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::same(3))
            .show(ui, |ui| {
                ui.set_width(panel_width);
                for item in tool_palette_items() {
                    let selected = self.schematic_tools.active == item.2;
                    let (bg, icon_color) = if selected {
                        (palette.accent_soft, palette.accent)
                    } else {
                        (Color32::TRANSPARENT, palette.text_muted)
                    };

                    let btn = egui::Button::new(
                        RichText::new(item.0).size(14.0).color(icon_color),
                    )
                    .fill(bg)
                    .stroke(if selected {
                        Stroke::new(1.0, palette.accent)
                    } else {
                        Stroke::NONE
                    })
                    .corner_radius(CornerRadius::same(4));

                    let response = ui
                        .add_sized([panel_width - 4.0, button_size], btn)
                        .on_hover_text(item.1);

                    // Hover effect for non-selected tools
                    if response.hovered() && !selected {
                        let painter = ui.painter();
                        painter.rect_filled(
                            response.rect,
                            CornerRadius::same(4),
                            palette.panel_hover,
                        );
                        painter.text(
                            response.rect.center(),
                            egui::Align2::CENTER_CENTER,
                            item.0,
                            egui::FontId::proportional(14.0),
                            palette.text,
                        );
                    }

                    // Active tool indicator: left accent bar
                    if selected {
                        let painter = ui.painter();
                        let bar = eframe::egui::Rect::from_min_size(
                            response.rect.left_top(),
                            egui::Vec2::new(2.0, response.rect.height()),
                        );
                        painter.rect_filled(
                            bar,
                            CornerRadius::same(1),
                            palette.accent,
                        );
                    }

                    if response.clicked() {
                        self.activate_schematic_tool_direct(item.2);
                    }
                }
            });

        panel_width + 4.0
    }
}
