use super::NekoSpiceApp;
use eframe::egui::{self, Vec2};

pub fn run_native() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NekoSpice")
            .with_inner_size([1440.0, 920.0])
            .with_min_inner_size([960.0, 640.0])
            .with_app_id("nekospice"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "NekoSpice",
        native_options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            let mut style = (*cc.egui_ctx.global_style()).clone();
            style.spacing.item_spacing = Vec2::new(8.0, 6.0);
            style.spacing.button_padding = Vec2::new(10.0, 4.0);
            cc.egui_ctx.set_global_style(style);
            Ok(Box::new(NekoSpiceApp::default()))
        }),
    )
}
