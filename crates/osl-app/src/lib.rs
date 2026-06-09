use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use osl_kicad::{
    KicadBoundingBox, KicadCanvasBusEntry, KicadCanvasGraphic, KicadCanvasHit, KicadCanvasScene,
    KicadCanvasSheet, KicadEditSummary, KicadPoint, KicadSchematic, KicadSchematicEdit,
    read_kicad_schematic_with_libraries, write_kicad_schematic,
};
use std::path::{Path, PathBuf};

const DEFAULT_SCHEMATIC: &str = "examples/kicad_schematic/rc.kicad_sch";
const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 180.0;
const EDIT_NUDGE_MM: f64 = 2.54;

#[derive(Debug)]
struct KicadGuiDocument {
    path: PathBuf,
    schematic: KicadSchematic,
    dirty: bool,
}

impl KicadGuiDocument {
    fn load(path: PathBuf) -> Result<Self, String> {
        read_kicad_schematic_with_libraries(&path)
            .map(|schematic| Self {
                path,
                schematic,
                dirty: false,
            })
            .map_err(|error| error.to_string())
    }

    fn scene(&self) -> KicadCanvasScene {
        self.schematic.canvas_scene()
    }

    fn delete_item(&mut self, uuid: &str) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(KicadSchematicEdit::DeleteItem {
                uuid: uuid.to_string(),
            })
            .inspect(|_| {
                self.dirty = true;
            })
            .map_err(|error| error.to_string())
    }

    fn move_item(&mut self, uuid: &str, delta: KicadPoint) -> Result<KicadEditSummary, String> {
        self.schematic
            .apply_edit(KicadSchematicEdit::MoveItem {
                uuid: uuid.to_string(),
                delta,
            })
            .inspect(|_| {
                self.dirty = true;
            })
            .map_err(|error| error.to_string())
    }

    fn save(&mut self) -> Result<(), String> {
        write_kicad_schematic(&self.path, &self.schematic)
            .inspect(|_| {
                self.dirty = false;
            })
            .map_err(|error| error.to_string())
    }
}

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
    document: Option<KicadGuiDocument>,
    scene: Option<KicadCanvasScene>,
    selected_hit: Option<KicadCanvasHit>,
    load_error: Option<String>,
    status_message: Option<String>,
    viewport: CanvasViewport,
}

#[derive(Debug, Clone, Copy)]
enum EditNudgeDirection {
    Left,
    Right,
    Up,
    Down,
}

impl EditNudgeDirection {
    fn delta(self) -> KicadPoint {
        match self {
            Self::Left => KicadPoint {
                x: -EDIT_NUDGE_MM,
                y: 0.0,
            },
            Self::Right => KicadPoint {
                x: EDIT_NUDGE_MM,
                y: 0.0,
            },
            Self::Up => KicadPoint {
                x: 0.0,
                y: -EDIT_NUDGE_MM,
            },
            Self::Down => KicadPoint {
                x: 0.0,
                y: EDIT_NUDGE_MM,
            },
        }
    }
}

impl Default for NekoSpiceApp {
    fn default() -> Self {
        let mut app = Self {
            schematic_path: DEFAULT_SCHEMATIC.to_string(),
            document: None,
            scene: None,
            selected_hit: None,
            load_error: None,
            status_message: None,
            viewport: CanvasViewport::default(),
        };
        app.load_schematic(PathBuf::from(DEFAULT_SCHEMATIC));
        app
    }
}

impl NekoSpiceApp {
    fn load_schematic(&mut self, path: PathBuf) {
        match KicadGuiDocument::load(path.clone()) {
            Ok(document) => {
                let scene = document.scene();
                self.schematic_path = path.display().to_string();
                self.viewport.fit_scene(scene.bounds);
                self.document = Some(document);
                self.scene = Some(scene);
                self.selected_hit = None;
                self.load_error = None;
                self.status_message = Some("Loaded schematic".to_string());
            }
            Err(error) => {
                self.load_error = Some(error.to_string());
                self.status_message = None;
            }
        }
    }

    fn delete_selected(&mut self) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        match document.delete_item(&uuid) {
            Ok(summary) => {
                let scene = document.scene();
                self.viewport.fit_scene(scene.bounds);
                self.scene = Some(scene);
                self.selected_hit = None;
                self.load_error = None;
                self.status_message =
                    Some(format!("Deleted {} {}", summary.operation, summary.target));
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    fn move_selected(&mut self, delta: KicadPoint) {
        let Some(uuid) = self.selected_hit.as_ref().and_then(|hit| hit.uuid.clone()) else {
            self.status_message = Some("Selected item has no KiCad UUID".to_string());
            return;
        };
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        match document.move_item(&uuid, delta) {
            Ok(summary) => {
                let scene = document.scene();
                self.selected_hit = scene.item_hit_by_uuid(&uuid);
                self.scene = Some(scene);
                self.load_error = None;
                self.status_message =
                    Some(format!("Moved {} {}", summary.operation, summary.target));
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    fn nudge_selected(&mut self, direction: EditNudgeDirection) {
        self.move_selected(direction.delta());
    }

    fn save_document(&mut self) {
        let Some(document) = &mut self.document else {
            self.status_message = Some("No editable schematic loaded".to_string());
            return;
        };

        match document.save() {
            Ok(()) => {
                self.status_message = Some(format!("Saved {}", document.path.display()));
                self.load_error = None;
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
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
            let can_edit = self.document.is_some();
            if ui
                .add_enabled(can_edit, egui::Button::new("Save"))
                .clicked()
            {
                self.save_document();
            }
            let can_delete = self
                .selected_hit
                .as_ref()
                .and_then(|hit| hit.uuid.as_ref())
                .is_some();
            if ui
                .add_enabled(can_edit && can_delete, egui::Button::new("Delete Selected"))
                .clicked()
            {
                self.delete_selected();
            }
            if can_edit && can_delete {
                ui.separator();
                if ui.button("Left").clicked() {
                    self.nudge_selected(EditNudgeDirection::Left);
                }
                if ui.button("Right").clicked() {
                    self.nudge_selected(EditNudgeDirection::Right);
                }
                if ui.button("Up").clicked() {
                    self.nudge_selected(EditNudgeDirection::Up);
                }
                if ui.button("Down").clicked() {
                    self.nudge_selected(EditNudgeDirection::Down);
                }
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
        if let Some(document) = &self.document {
            ui.label(format!(
                "Dirty: {}",
                if document.dirty { "yes" } else { "no" }
            ));
        }
        if let Some(message) = &self.status_message {
            ui.separator();
            ui.label(message);
        }

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

        if !ui.ctx().text_edit_focused() {
            if ui.input(|input| input.key_pressed(egui::Key::Delete)) {
                self.delete_selected();
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowLeft)) {
                self.nudge_selected(EditNudgeDirection::Left);
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowRight)) {
                self.nudge_selected(EditNudgeDirection::Right);
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                self.nudge_selected(EditNudgeDirection::Up);
            }
            if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                self.nudge_selected(EditNudgeDirection::Down);
            }
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(250, 251, 252));
        self.draw_grid(&painter, rect);

        if let Some(scene) = &self.scene {
            let visible_bounds = self.viewport.visible_world_bounds(rect);
            self.draw_scene(&painter, rect, scene, visible_bounds);
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

    fn draw_scene(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        scene: &KicadCanvasScene,
        visible_bounds: KicadBoundingBox,
    ) {
        for sheet in &scene.sheets {
            if !item_visible(sheet.bounds, visible_bounds) {
                continue;
            }
            self.draw_sheet(painter, rect, sheet);
        }
        for rule_area in &scene.rule_areas {
            if !item_visible(rule_area.bounds, visible_bounds) {
                continue;
            }
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
            if !item_visible(graphic.bounds(), visible_bounds) {
                continue;
            }
            self.draw_graphic(painter, rect, graphic, Color32::from_rgb(90, 90, 90));
        }
        for symbol in &scene.symbols {
            if !item_visible(symbol.bounds, visible_bounds) {
                continue;
            }
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
            if !item_visible(wire.bounds, visible_bounds) {
                continue;
            }
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
            if !item_visible(bus.bounds, visible_bounds) {
                continue;
            }
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
            if !item_visible(entry.bounds, visible_bounds) {
                continue;
            }
            self.draw_bus_entry(painter, rect, entry);
        }
        for label in &scene.directive_labels {
            if !item_visible(label.bounds, visible_bounds) {
                continue;
            }
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
            if !item_visible(label.bounds, visible_bounds) {
                continue;
            }
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
            if !item_visible(text.bounds, visible_bounds) {
                continue;
            }
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
            if !item_visible(text_box.bounds, visible_bounds) {
                continue;
            }
            if let Some(bounds) = text_box.bounds {
                self.draw_bounds(painter, rect, bounds, Color32::from_rgb(120, 120, 120), 1.0);
            }
        }
        for junction in &scene.junctions {
            if !junction.bounds.intersects(visible_bounds) {
                continue;
            }
            let center = self.viewport.world_to_screen(rect, junction.at);
            painter.circle_filled(center, 3.0, Color32::from_rgb(0, 150, 72));
        }
        for marker in &scene.no_connects {
            if !marker.bounds.intersects(visible_bounds) {
                continue;
            }
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

    fn visible_world_bounds(self, rect: Rect) -> KicadBoundingBox {
        let top_left = self.screen_to_world(rect, rect.left_top());
        let bottom_right = self.screen_to_world(rect, rect.right_bottom());
        KicadBoundingBox {
            min: KicadPoint {
                x: top_left.x.min(bottom_right.x),
                y: top_left.y.min(bottom_right.y),
            },
            max: KicadPoint {
                x: top_left.x.max(bottom_right.x),
                y: top_left.y.max(bottom_right.y),
            },
        }
    }
}

pub fn load_canvas_scene(path: &Path) -> Result<KicadCanvasScene, String> {
    read_kicad_schematic_with_libraries(path)
        .map(|schematic| schematic.canvas_scene())
        .map_err(|error| error.to_string())
}

fn item_visible(bounds: Option<KicadBoundingBox>, visible_bounds: KicadBoundingBox) -> bool {
    // Scene geometry stays in osl-kicad; the GUI only decides whether an item can affect this viewport.
    bounds.is_none_or(|bounds| bounds.intersects(visible_bounds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn viewport_exposes_visible_world_bounds_for_canvas_culling() {
        let viewport = CanvasViewport {
            zoom: 10.0,
            pan: Vec2::ZERO,
        };
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(200.0, 100.0));
        let visible = viewport.visible_world_bounds(rect);

        assert!((visible.min.x + 10.0).abs() < 1e-6);
        assert!((visible.max.x - 10.0).abs() < 1e-6);
        assert!((visible.min.y + 5.0).abs() < 1e-6);
        assert!((visible.max.y - 5.0).abs() < 1e-6);
        assert!(item_visible(
            Some(KicadBoundingBox {
                min: KicadPoint { x: 9.0, y: 4.0 },
                max: KicadPoint { x: 12.0, y: 6.0 },
            }),
            visible
        ));
        assert!(!item_visible(
            Some(KicadBoundingBox {
                min: KicadPoint { x: 12.0, y: 6.0 },
                max: KicadPoint { x: 14.0, y: 8.0 },
            }),
            visible
        ));
    }

    #[test]
    fn document_deletes_selected_uuid_and_saves_schematic() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let source = workspace_root.join(DEFAULT_SCHEMATIC);
        let temp_path = std::env::temp_dir().join(format!(
            "nekospice_gui_delete_{}.kicad_sch",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::copy(&source, &temp_path).unwrap();

        let mut document = KicadGuiDocument::load(temp_path.clone()).unwrap();
        assert!(!document.dirty);
        assert_eq!(document.scene().wires.len(), 3);

        let summary = document
            .delete_item("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert_eq!(summary.operation, "delete-wire");
        assert!(document.dirty);
        assert_eq!(document.scene().wires.len(), 2);

        document.save().unwrap();
        assert!(!document.dirty);
        let saved = fs::read_to_string(&temp_path).unwrap();
        assert!(!saved.contains("22222222-2222-2222-2222-222222222222"));

        fs::remove_file(temp_path).unwrap();
    }

    #[test]
    fn document_moves_selected_uuid_and_keeps_canvas_hit_addressable() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let source = workspace_root.join(DEFAULT_SCHEMATIC);
        let temp_path = std::env::temp_dir().join(format!(
            "nekospice_gui_move_{}.kicad_sch",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::copy(&source, &temp_path).unwrap();

        let mut document = KicadGuiDocument::load(temp_path.clone()).unwrap();
        let original_hit = document
            .scene()
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();

        let summary = document
            .move_item(
                "22222222-2222-2222-2222-222222222222",
                KicadPoint { x: 2.54, y: 0.0 },
            )
            .unwrap();
        assert_eq!(summary.operation, "move-wire");
        assert!(document.dirty);

        let moved_hit = document
            .scene()
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert!((moved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
        assert_eq!(moved_hit.kind, "wire");

        document.save().unwrap();
        assert!(!document.dirty);
        let reloaded_scene = read_kicad_schematic_with_libraries(&temp_path)
            .unwrap()
            .canvas_scene();
        let saved_hit = reloaded_scene
            .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
            .unwrap();
        assert_eq!(saved_hit.kind, "wire");
        assert!((saved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
        assert!((saved_hit.bounds.min.y - original_hit.bounds.min.y).abs() < 1e-6);

        fs::remove_file(temp_path).unwrap();
    }
}
