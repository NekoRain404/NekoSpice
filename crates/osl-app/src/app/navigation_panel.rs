//! 左侧导航栏面板。绘制垂直导航图标列表并响应工作区切换。
//!
use super::NekoSpiceApp;
use super::localization::UiText;
use super::navigation::StudioWorkspace;
use super::theme::{StudioTheme, StudioThemeMode};
use super::widgets::metric_row;
use eframe::egui::{self, CornerRadius, Color32, RichText, Stroke, Vec2};

/// Returns the Unicode icon character for each workspace.
///
/// Uses distinct Unicode symbols that are visually clear at small sizes.
fn workspace_icon(ws: StudioWorkspace) -> &'static str {
    match ws {
        StudioWorkspace::Home => "\u{2302}",        // house
        StudioWorkspace::Schematic => "\u{25CE}",   // target/crosshair
        StudioWorkspace::Library => "\u{2630}",      // trigram (three lines)
        StudioWorkspace::Simulation => "\u{25B6}",   // play button
        StudioWorkspace::Optimization => "\u{2699}", // gear/cog
        StudioWorkspace::Review => "\u{2714}",       // checkmark
        StudioWorkspace::Waveforms => "\u{223F}",    // sine wave
        StudioWorkspace::Reports => "\u{2261}",      // hamburger menu (list)
        StudioWorkspace::Settings => "\u{2692}",     // hammer/wrench
    }
}

/// Draw a single workspace button with icon and label.
///
/// Active state uses accent fill with a left bar indicator.
/// Hovered state lightens the background.
fn draw_workspace_button(
    ui: &mut egui::Ui,
    mode: StudioThemeMode,
    ws: StudioWorkspace,
    selected: bool,
    label: &str,
) -> bool {
    let palette = StudioTheme::palette(mode);
    let icon = workspace_icon(ws);
    let width = ui.available_width();
    let height = 40.0;

    let fg = if selected {
        palette.accent
    } else {
        palette.text
    };

    let response = ui.allocate_ui_with_layout(
        Vec2::new(width, height),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add_space(10.0);
            ui.label(
                RichText::new(icon)
                    .color(if selected { palette.accent } else { palette.text })
                    .strong()
                    .size(17.0),
            );
            ui.add_space(8.0);
            ui.label(RichText::new(label).color(fg).size(13.5).strong());
        },
    );

    let rect = response.response.rect;
    let click = ui.interact(rect, egui::Id::new(("nav_ws", ws)), egui::Sense::click());

    let painter = ui.painter();
    let corner = CornerRadius::same(6);

    if click.hovered() && !selected {
        painter.rect_filled(rect, corner, palette.panel_hover);
    }

    if selected {
        // Bright left bar indicator
        let bar = eframe::egui::Rect::from_min_size(
            rect.left_top(),
            Vec2::new(3.0, rect.height()),
        );
        painter.rect_filled(bar, CornerRadius::same(0), palette.accent);
        // Subtle background tint for selected item
        painter.rect_filled(rect, corner, palette.accent_soft);
    }

    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, if selected { palette.accent } else { Color32::TRANSPARENT }),
        eframe::egui::StrokeKind::Inside,
    );

    click.clicked()
}

impl NekoSpiceApp {
    /// draw workspace navigation。
    pub(super) fn draw_workspace_navigation(&mut self, ui: &mut egui::Ui) {
        let mode = self.theme_mode();
        let palette = self.theme_palette();
        let locale = self.locale();

        // Branding header with logo icon
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("\u{25CE}")
                    .color(palette.accent)
                    .size(22.0)
                    .strong(),
            );
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(self.text(UiText::StudioTitle))
                        .color(palette.text)
                        .size(16.0)
                        .strong(),
                );
                ui.label(
                    RichText::new(self.text(UiText::StudioSubtitle))
                        .color(palette.text_muted)
                        .size(10.0),
                );
            });
        });

        ui.add_space(12.0);

        for workspace in StudioWorkspace::ALL {
            let selected = self.active_workspace == workspace;
            let label = workspace.localized_label(locale);
            if draw_workspace_button(ui, mode, workspace, selected, label) {
                self.active_workspace = workspace;
            }
            ui.add_space(2.0);
        }

        // Push system info to the bottom
        let remaining = ui.available_height();
        if remaining > 130.0 {
            ui.add_space(remaining - 130.0);
        }

        StudioTheme::panel_frame_for(mode).show(ui, |ui| {
            ui.label(StudioTheme::section_title_for(
                mode,
                self.text(UiText::System),
            ));
            ui.add_space(4.0);
            metric_row(ui, mode, self.text(UiText::Renderer), "wgpu");
            metric_row(ui, mode, self.text(UiText::Solver), "ngspice");
            let dirty = self.text(UiText::Dirty);
            let clean = self.text(UiText::Clean);
            let missing = self.text(UiText::Missing);
            metric_row(
                ui,
                mode,
                self.text(UiText::Document),
                self.document
                    .as_ref()
                    .map(|document| if document.is_dirty() { dirty } else { clean })
                    .unwrap_or(missing),
            );
            ui.separator();
            metric_row(
                ui,
                mode,
                self.text(UiText::Theme),
                self.theme_mode_label(self.theme_mode()),
            );
            metric_row(
                ui,
                mode,
                self.text(UiText::Language),
                self.locale().native_name(),
            );
        });
    }
}
