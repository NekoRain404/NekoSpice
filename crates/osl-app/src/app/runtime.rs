use super::NekoSpiceApp;
use super::theme::StudioTheme;
use eframe::egui::{self};

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
            StudioTheme::apply(&cc.egui_ctx);
            Ok(Box::new(NekoSpiceApp::default()))
        }),
    )
}
