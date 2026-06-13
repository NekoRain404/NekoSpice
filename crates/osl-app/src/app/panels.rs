//! 根面板布局调度。实现画布渲染和 UI 面板的完整布局。
//!
use super::NekoSpiceApp;
use super::theme::StudioTheme;
use eframe::egui::{self, Rect, UiBuilder, pos2};

const TOP_STATUS_HEIGHT: f32 = 40.0;
const BOTTOM_STATUS_HEIGHT: f32 = 26.0;
const NAVIGATION_WIDTH: f32 = 180.0;
const PROJECT_CONTEXT_WIDTH: f32 = 260.0;
const WORKSPACE_CONTEXT_WIDTH: f32 = 320.0;
const REGION_PADDING: f32 = 8.0;
const MIN_CENTER_WIDTH: f32 = 340.0;

impl eframe::App for NekoSpiceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let theme_mode = self.theme_mode();
        let palette = self.theme_palette();
        StudioTheme::apply(ui.ctx(), theme_mode);

        // Handle global keyboard shortcuts (works in every workspace).
        self.handle_global_shortcuts(ui.ctx());
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
        let layout = ShellLayout::from_body(body);
        let navigation = Rect::from_min_max(
            body.left_top(),
            pos2(body.left() + layout.navigation_width, body.bottom()),
        );
        let project_context = Rect::from_min_max(
            pos2(navigation.right(), body.top()),
            pos2(
                navigation.right() + layout.project_context_width,
                body.bottom(),
            ),
        );
        let workspace_context = Rect::from_min_max(
            pos2(body.right() - layout.workspace_context_width, body.top()),
            body.right_bottom(),
        );
        let workspace = Rect::from_min_max(
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
                    app.draw_left_context_panel(ui);
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
        self.draw_region(ui, workspace, "studio_center_workspace", |app, ui| {
            app.draw_center_workspace(ui);
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct ShellLayout {
    navigation_width: f32,
    project_context_width: f32,
    workspace_context_width: f32,
}

impl ShellLayout {
    fn from_body(body: Rect) -> Self {
        let navigation_width = NAVIGATION_WIDTH.min(body.width() * 0.28);
        let remaining_width = (body.width() - navigation_width).max(0.0);
        let side_width = PROJECT_CONTEXT_WIDTH + WORKSPACE_CONTEXT_WIDTH;
        let center_target = MIN_CENTER_WIDTH.min(remaining_width);
        let side_scale = ((remaining_width - center_target).max(0.0) / side_width).min(1.0);

        Self {
            navigation_width,
            project_context_width: PROJECT_CONTEXT_WIDTH * side_scale,
            workspace_context_width: WORKSPACE_CONTEXT_WIDTH * side_scale,
        }
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
        if rect.width() <= REGION_PADDING * 2.0 || rect.height() <= REGION_PADDING * 2.0 {
            return;
        }
        let palette = self.theme_palette();
        ui.painter().rect_filled(rect, 0.0, palette.background);
        // Panel borders removed for clean layout
        let content_rect = rect.shrink(REGION_PADDING);
        let builder = UiBuilder::new().id_salt(id_salt).max_rect(content_rect);
        ui.scope_builder(builder, |ui| {
            ui.set_clip_rect(content_rect);
            add_contents(self, ui);
        });
    }
}
