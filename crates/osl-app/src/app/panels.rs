use super::NekoSpiceApp;
use super::theme::StudioTheme;
use eframe::egui::{self, Rect, UiBuilder, pos2};

const TOP_STATUS_HEIGHT: f32 = 68.0;
const BOTTOM_STATUS_HEIGHT: f32 = 32.0;
const NAVIGATION_WIDTH: f32 = 190.0;
const PROJECT_CONTEXT_WIDTH: f32 = 280.0;
const WORKSPACE_CONTEXT_WIDTH: f32 = 360.0;
const REGION_PADDING: f32 = 8.0;

impl eframe::App for NekoSpiceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let theme_mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::apply(ui.ctx(), theme_mode);

        let root = ui.max_rect();
        ui.painter().rect_filled(root, 0.0, palette.background);
        let top = Rect::from_min_max(
            root.left_top(),
            pos2(root.right(), root.top() + TOP_STATUS_HEIGHT),
        );
        let bottom = Rect::from_min_max(
            pos2(root.left(), root.bottom() - BOTTOM_STATUS_HEIGHT),
            root.right_bottom(),
        );
        let body = Rect::from_min_max(
            pos2(root.left(), top.bottom()),
            pos2(root.right(), bottom.top()),
        );
        let navigation = Rect::from_min_max(
            body.left_top(),
            pos2(body.left() + NAVIGATION_WIDTH, body.bottom()),
        );
        let project_context = Rect::from_min_max(
            pos2(navigation.right(), body.top()),
            pos2(navigation.right() + PROJECT_CONTEXT_WIDTH, body.bottom()),
        );
        let workspace_context = Rect::from_min_max(
            pos2(body.right() - WORKSPACE_CONTEXT_WIDTH, body.top()),
            body.right_bottom(),
        );
        let canvas = Rect::from_min_max(
            pos2(project_context.right(), body.top()),
            pos2(workspace_context.left(), body.bottom()),
        );

        self.draw_region(ui, top, "studio_top_status", |app, ui| {
            app.draw_studio_top_bar(ui);
        });
        self.draw_region(ui, bottom, "studio_bottom_status", |app, ui| {
            app.draw_bottom_status_strip(ui);
        });
        self.draw_region(ui, navigation, "studio_navigation", |app, ui| {
            app.draw_workspace_navigation(ui);
        });
        self.draw_region(ui, project_context, "studio_project_context", |app, ui| {
            egui::ScrollArea::vertical()
                .id_salt("studio_project_context_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    app.draw_project_sidebar(ui);
                });
        });
        self.draw_region(
            ui,
            workspace_context,
            "studio_workspace_context",
            |app, ui| {
                egui::ScrollArea::vertical()
                    .id_salt("studio_workspace_context_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        app.draw_right_workspace_panel(ui);
                    });
            },
        );
        self.draw_region(ui, canvas, "studio_canvas", |app, ui| {
            app.draw_studio_canvas_frame(ui);
        });
    }
}

impl NekoSpiceApp {
    fn draw_region(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        id_salt: &'static str,
        add_contents: impl FnOnce(&mut Self, &mut egui::Ui),
    ) {
        let palette = self.theme_palette();
        ui.painter().rect_filled(rect, 0.0, palette.background);
        ui.painter().rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, palette.border),
            egui::StrokeKind::Inside,
        );
        let content_rect = rect.shrink(REGION_PADDING);
        let builder = UiBuilder::new().id_salt(id_salt).max_rect(content_rect);
        ui.scope_builder(builder, |ui| {
            ui.set_clip_rect(content_rect);
            add_contents(self, ui);
        });
    }
}
