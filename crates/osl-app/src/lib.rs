use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use osl_kicad::{
    KicadBoundingBox, KicadCanvasBusEntry, KicadCanvasGraphic, KicadCanvasHit, KicadCanvasScene,
    KicadCanvasSheet, KicadPoint, read_kicad_schematic_with_libraries,
};
use std::path::{Path, PathBuf};

const DEFAULT_SCHEMATIC: &str = "examples/kicad_schematic/rc.kicad_sch";
const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 180.0;

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
        Box::new(|_cc| Ok(Box::new(NekoSpiceApp::default()))),
    )
}

#[derive(Debug)]
pub struct NekoSpiceApp {
    schematic_path: String,
    scene: Option<KicadCanvasScene>,
    selected_hit: Option<KicadCanvasHit>,
    load_error: Option<String>,
    viewport: CanvasViewport,
}

impl Default for NekoSpiceApp {
    fn default() -> Self {
        let mut app = Self {
            schematic_path: DEFAULT_SCHEMATIC.to_string(),
            scene: None,
            selected_hit: None,
            load_error: None,
            viewport: CanvasViewport::default(),
        };
        app.load_schematic(PathBuf::from(DEFAULT_SCHEMATIC));
        app
    }
}

impl NekoSpiceApp {
    fn load_schematic(&mut self, path: PathBuf) {
        match read_kicad_schematic_with_libraries(&path) {
            Ok(schematic) => {
                let scene = schematic.canvas_scene();
                self.schematic_path = path.display().to_string();
                self.viewport.fit_scene(scene.bounds);
                self.scene = Some(scene);
                self.selected_hit = None;
                self.load_error = None;
            }
            Err(error) => {
                self.load_error = Some(error.to_string());
            }
        }
    }

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Schematic");
            let path_response = ui.text_edit_singleline(&mut self.schematic_path);
            let load_requested = ui.button("Open").clicked()
                || (path_response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter)));
            if load_requested {
                self.load_schematic(PathBuf::from(self.schematic_path.trim()));
            }
            if ui.button("Fit").clicked() {
                self.viewport
                    .fit_scene(self.scene.as_ref().and_then(|scene| scene.bounds));
            }
        });
    }

    fn draw_sidebar(&self, ui: &mut egui::Ui) {
        ui.heading("NekoSpice");
        ui.label("GPU renderer: wgpu");
        ui.separator();

        if let Some(error) = &self.load_error {
            ui.colored_label(Color32::from_rgb(190, 40, 40), error);
            return;
        }

        let Some(scene) = &self.scene else {
            ui.label("No schematic loaded");
            return;
        };

        ui.label(format!("Source: {}", scene.source));
        ui.label(format!("Symbols: {}", scene.symbols.len()));
        ui.label(format!("Wires: {}", scene.wires.len()));
        ui.label(format!("Buses: {}", scene.buses.len()));
        ui.label(format!("Labels: {}", scene.labels.len()));
        ui.label(format!("Sheets: {}", scene.sheets.len()));
        ui.label(format!("Graphics: {}", scene.graphics.len()));
        ui.label(format!("Zoom: {:.1} px/mm", self.viewport.zoom));

        ui.separator();
        ui.heading("Selection");
        if let Some(hit) = &self.selected_hit {
            ui.label(format!("Kind: {}", hit.kind));
            ui.label(format!("Label: {}", hit.label));
            if let Some(uuid) = &hit.uuid {
                ui.monospace(uuid);
            }
        } else {
            ui.label("None");
        }
    }

    fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        let available = ui.available_size_before_wrap();
        let desired_size = Vec2::new(available.x.max(240.0), available.y.max(240.0));
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if response.dragged_by(egui::PointerButton::Middle) {
            self.viewport.pan += response.drag_delta();
        }

        let pointer_over_canvas = ui
            .input(|input| input.pointer.hover_pos())
            .is_some_and(|position| rect.contains(position));
        if pointer_over_canvas {
            let zoom_delta = ui.input(|input| input.zoom_delta());
            if (zoom_delta - 1.0).abs() > f32::EPSILON
                && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            {
                self.viewport.zoom_around(rect, pointer, zoom_delta);
            }

            let scroll = ui.input(|input| input.smooth_scroll_delta);
            if scroll != Vec2::ZERO {
                self.viewport.pan += scroll;
            }
        }

        if response.clicked()
            && let (Some(scene), Some(pointer)) = (&self.scene, response.interact_pointer_pos())
        {
            let schematic_point = self.viewport.screen_to_world(rect, pointer);
            self.selected_hit = scene.hit_test(schematic_point).hits.into_iter().next();
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(250, 251, 252));
        self.draw_grid(&painter, rect);

        if let Some(scene) = &self.scene {
            self.draw_scene(&painter, rect, scene);
            if let Some(hit) = &self.selected_hit {
                self.draw_bounds(
                    &painter,
                    rect,
                    hit.bounds,
                    Color32::from_rgb(20, 120, 220),
                    2.0,
                );
            }
        }
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect) {
        let major = (10.0 * self.viewport.zoom).max(16.0);
        let origin = rect.center() + self.viewport.pan;
        let stroke = Stroke::new(1.0, Color32::from_gray(226));

        let mut x = origin.x % major;
        while x < rect.width() {
            let screen_x = rect.left() + x;
            painter.line_segment(
                [
                    Pos2::new(screen_x, rect.top()),
                    Pos2::new(screen_x, rect.bottom()),
                ],
                stroke,
            );
            x += major;
        }

        let mut y = origin.y % major;
        while y < rect.height() {
            let screen_y = rect.top() + y;
            painter.line_segment(
                [
                    Pos2::new(rect.left(), screen_y),
                    Pos2::new(rect.right(), screen_y),
                ],
                stroke,
            );
            y += major;
        }
    }

    fn draw_scene(&self, painter: &egui::Painter, rect: Rect, scene: &KicadCanvasScene) {
        for sheet in &scene.sheets {
            self.draw_sheet(painter, rect, sheet);
        }
        for rule_area in &scene.rule_areas {
            self.draw_polyline(
                painter,
                rect,
                &rule_area.points,
                true,
                Color32::from_rgb(150, 110, 20),
                1.5,
            );
        }
        for graphic in &scene.graphics {
            self.draw_graphic(painter, rect, graphic, Color32::from_rgb(90, 90, 90));
        }
        for symbol in &scene.symbols {
            for graphic in &symbol.graphics {
                self.draw_graphic(painter, rect, graphic, Color32::from_rgb(25, 25, 25));
            }
            for pin in &symbol.pins {
                self.draw_line(
                    painter,
                    rect,
                    pin.start,
                    pin.end,
                    Color32::from_rgb(30, 30, 30),
                    1.5,
                );
            }
            if let Some(bounds) = symbol.bounds {
                let label_pos = self.viewport.world_to_screen(rect, bounds.min);
                painter.text(
                    label_pos,
                    Align2::LEFT_BOTTOM,
                    &symbol.reference,
                    FontId::monospace(12.0),
                    Color32::from_rgb(25, 25, 25),
                );
            }
        }
        for wire in &scene.wires {
            self.draw_polyline(
                painter,
                rect,
                &wire.points,
                false,
                Color32::from_rgb(0, 150, 72),
                2.0,
            );
        }
        for bus in &scene.buses {
            self.draw_polyline(
                painter,
                rect,
                &bus.points,
                false,
                Color32::from_rgb(70, 95, 220),
                3.0,
            );
        }
        for entry in &scene.bus_entries {
            self.draw_bus_entry(painter, rect, entry);
        }
        for label in &scene.directive_labels {
            if let Some(bounds) = label.bounds {
                self.draw_bounds(painter, rect, bounds, Color32::from_rgb(180, 95, 35), 1.0);
            }
            if let Some(at) = label.at {
                painter.text(
                    self.viewport
                        .world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                    Align2::LEFT_TOP,
                    &label.text,
                    FontId::monospace(12.0),
                    Color32::from_rgb(150, 65, 20),
                );
            }
        }
        for label in &scene.labels {
            if let Some(at) = label.at {
                painter.text(
                    self.viewport
                        .world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                    Align2::LEFT_TOP,
                    &label.text,
                    FontId::monospace(12.0),
                    Color32::from_rgb(0, 95, 180),
                );
            }
        }
        for text in &scene.text_items {
            if let Some(at) = text.at {
                let color = if text.is_spice_directive {
                    Color32::from_rgb(165, 45, 45)
                } else {
                    Color32::from_rgb(55, 55, 55)
                };
                painter.text(
                    self.viewport
                        .world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                    Align2::LEFT_TOP,
                    &text.text,
                    FontId::monospace(12.0),
                    color,
                );
            }
        }
        for text_box in &scene.text_boxes {
            if let Some(bounds) = text_box.bounds {
                self.draw_bounds(painter, rect, bounds, Color32::from_rgb(120, 120, 120), 1.0);
            }
        }
        for junction in &scene.junctions {
            let center = self.viewport.world_to_screen(rect, junction.at);
            painter.circle_filled(center, 3.0, Color32::from_rgb(0, 150, 72));
        }
        for marker in &scene.no_connects {
            let center = self.viewport.world_to_screen(rect, marker.at);
            let size = 5.0;
            painter.line_segment(
                [
                    Pos2::new(center.x - size, center.y - size),
                    Pos2::new(center.x + size, center.y + size),
                ],
                Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
            );
            painter.line_segment(
                [
                    Pos2::new(center.x - size, center.y + size),
                    Pos2::new(center.x + size, center.y - size),
                ],
                Stroke::new(1.5, Color32::from_rgb(55, 55, 55)),
            );
        }
    }

    fn draw_sheet(&self, painter: &egui::Painter, rect: Rect, sheet: &KicadCanvasSheet) {
        let Some(at) = sheet.at else {
            return;
        };
        let Some(size) = sheet.size else {
            return;
        };
        let start = self
            .viewport
            .world_to_screen(rect, KicadPoint { x: at.x, y: at.y });
        let end = self.viewport.world_to_screen(
            rect,
            KicadPoint {
                x: at.x + size.width,
                y: at.y + size.height,
            },
        );
        let sheet_rect = Rect::from_two_pos(start, end);
        painter.rect_filled(sheet_rect, 0.0, Color32::from_rgb(245, 248, 255));
        painter.rect_stroke(
            sheet_rect,
            0.0,
            Stroke::new(1.5, Color32::from_rgb(90, 120, 190)),
            StrokeKind::Inside,
        );
        painter.text(
            sheet_rect.left_top() + Vec2::new(4.0, 4.0),
            Align2::LEFT_TOP,
            &sheet.name,
            FontId::monospace(12.0),
            Color32::from_rgb(50, 80, 150),
        );
    }

    fn draw_graphic(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        graphic: &KicadCanvasGraphic,
        color: Color32,
    ) {
        match graphic {
            KicadCanvasGraphic::Polyline { points, .. }
            | KicadCanvasGraphic::Bezier { points, .. } => {
                self.draw_polyline(painter, rect, points, false, color, 1.5);
            }
            KicadCanvasGraphic::Rectangle { start, end, .. } => {
                let start = self.viewport.world_to_screen(rect, *start);
                let end = self.viewport.world_to_screen(rect, *end);
                painter.rect_stroke(
                    Rect::from_two_pos(start, end),
                    0.0,
                    Stroke::new(1.5, color),
                    StrokeKind::Inside,
                );
            }
            KicadCanvasGraphic::Circle { center, radius, .. } => {
                painter.circle_stroke(
                    self.viewport.world_to_screen(rect, *center),
                    (*radius as f32 * self.viewport.zoom).abs(),
                    Stroke::new(1.5, color),
                );
            }
            KicadCanvasGraphic::Arc {
                start, mid, end, ..
            } => {
                let mut points = vec![*start];
                if let Some(mid) = mid {
                    points.push(*mid);
                }
                points.push(*end);
                self.draw_polyline(painter, rect, &points, false, color, 1.5);
            }
            KicadCanvasGraphic::Text { text, at, .. } => {
                if let Some(at) = at {
                    painter.text(
                        self.viewport
                            .world_to_screen(rect, KicadPoint { x: at.x, y: at.y }),
                        Align2::LEFT_TOP,
                        text,
                        FontId::monospace(12.0),
                        color,
                    );
                }
            }
        }
    }

    fn draw_polyline(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        points: &[KicadPoint],
        closed: bool,
        color: Color32,
        width: f32,
    ) {
        for segment in points.windows(2) {
            self.draw_line(painter, rect, segment[0], segment[1], color, width);
        }
        if closed && points.len() > 2 {
            self.draw_line(
                painter,
                rect,
                *points.last().unwrap(),
                points[0],
                color,
                width,
            );
        }
    }

    fn draw_line(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        start: KicadPoint,
        end: KicadPoint,
        color: Color32,
        width: f32,
    ) {
        painter.line_segment(
            [
                self.viewport.world_to_screen(rect, start),
                self.viewport.world_to_screen(rect, end),
            ],
            Stroke::new(width, color),
        );
    }

    fn draw_bus_entry(&self, painter: &egui::Painter, rect: Rect, entry: &KicadCanvasBusEntry) {
        self.draw_line(
            painter,
            rect,
            entry.at,
            entry.end(),
            Color32::from_rgb(70, 95, 220),
            2.0,
        );
    }

    fn draw_bounds(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        bounds: KicadBoundingBox,
        color: Color32,
        width: f32,
    ) {
        let min = self.viewport.world_to_screen(rect, bounds.min);
        let max = self.viewport.world_to_screen(rect, bounds.max);
        painter.rect_stroke(
            Rect::from_two_pos(min, max),
            0.0,
            Stroke::new(width, color),
            StrokeKind::Inside,
        );
    }
}

impl eframe::App for NekoSpiceApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.vertical(|ui| {
            self.draw_toolbar(ui);
            ui.separator();
            ui.horizontal(|ui| {
                ui.set_height(ui.available_height());
                ui.vertical(|ui| {
                    ui.set_width(260.0);
                    self.draw_sidebar(ui);
                });
                ui.separator();
                ui.vertical(|ui| {
                    self.draw_canvas(ui);
                });
            });
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct CanvasViewport {
    zoom: f32,
    pan: Vec2,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            zoom: 12.0,
            pan: Vec2::ZERO,
        }
    }
}

impl CanvasViewport {
    fn fit_scene(&mut self, bounds: Option<KicadBoundingBox>) {
        if let Some(bounds) = bounds {
            let width = bounds.width().max(1.0) as f32;
            let height = bounds.height().max(1.0) as f32;
            self.zoom = (900.0 / width).min(560.0 / height).clamp(4.0, 32.0);
            let center = KicadPoint {
                x: (bounds.min.x + bounds.max.x) / 2.0,
                y: (bounds.min.y + bounds.max.y) / 2.0,
            };
            self.pan = Vec2::new(
                -(center.x as f32) * self.zoom,
                -(center.y as f32) * self.zoom,
            );
        }
    }

    fn world_to_screen(self, rect: Rect, point: KicadPoint) -> Pos2 {
        rect.center() + self.pan + Vec2::new(point.x as f32 * self.zoom, point.y as f32 * self.zoom)
    }

    fn screen_to_world(self, rect: Rect, point: Pos2) -> KicadPoint {
        let local = point - rect.center() - self.pan;
        KicadPoint {
            x: (local.x / self.zoom) as f64,
            y: (local.y / self.zoom) as f64,
        }
    }

    fn zoom_around(&mut self, rect: Rect, screen_point: Pos2, zoom_delta: f32) {
        let before = self.screen_to_world(rect, screen_point);
        self.zoom = (self.zoom * zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
        let after_screen = self.world_to_screen(rect, before);
        self.pan += screen_point - after_screen;
    }
}

pub fn load_canvas_scene(path: &Path) -> Result<KicadCanvasScene, String> {
    read_kicad_schematic_with_libraries(path)
        .map(|schematic| schematic.canvas_scene())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_canvas_scene_for_gui() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let scene = load_canvas_scene(&workspace_root.join(DEFAULT_SCHEMATIC)).unwrap();
        assert!(!scene.symbols.is_empty());
        assert!(scene.bounds.is_some());
    }

    #[test]
    fn viewport_roundtrips_screen_and_world_points() {
        let viewport = CanvasViewport {
            zoom: 20.0,
            pan: Vec2::new(40.0, -10.0),
        };
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let world = KicadPoint { x: 12.7, y: 5.08 };
        let screen = viewport.world_to_screen(rect, world);
        let roundtrip = viewport.screen_to_world(rect, screen);
        assert!((roundtrip.x - world.x).abs() < 1e-6);
        assert!((roundtrip.y - world.y).abs() < 1e-6);
    }
}
