use super::NekoSpiceApp;
use super::theme::{StudioTheme, StudioThemeMode};
use eframe::egui::{self};
use std::fs;

const CJK_FONT_CANDIDATES: [&str; 3] = [
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
];

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
            install_system_cjk_font(&cc.egui_ctx);
            StudioTheme::apply(&cc.egui_ctx, StudioThemeMode::default());
            Ok(Box::new(NekoSpiceApp::default()))
        }),
    )
}

fn install_system_cjk_font(ctx: &egui::Context) {
    let Some((name, bytes)) = CJK_FONT_CANDIDATES.iter().find_map(|path| {
        fs::read(path)
            .ok()
            .map(|bytes| ((*path).to_string(), bytes))
    }) else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert(name.clone(), egui::FontData::from_owned(bytes).into());
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, name.clone());
    }
    ctx.set_fonts(fonts);
}
