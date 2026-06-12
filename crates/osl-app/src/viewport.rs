//! 画布视口管理。管理缩放级别、平移偏移和坐标变换。
//!
use eframe::egui::{Pos2, Rect, Vec2};
use osl_kicad::{KicadBoundingBox, KicadPoint};

const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 180.0;

#[derive(Debug, Clone, Copy)]
/// 画布视口。管理缩放级别、平移偏移和屏幕/原理图坐标变换。
pub(crate) struct CanvasViewport {
    pub(crate) zoom: f32,
    pub(crate) pan: Vec2,
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
    /// for rect。
    pub(crate) fn for_rect(rect: Rect, bounds: Option<KicadBoundingBox>) -> Self {
        let mut viewport = Self::default();
        viewport.fit_rect(rect, bounds);
        viewport
    }

    /// fit scene。
    pub(crate) fn fit_scene(&mut self, bounds: Option<KicadBoundingBox>) {
        self.fit_size(Vec2::new(900.0, 560.0), bounds, 4.0..=32.0);
    }

    /// fit rect。
    pub(crate) fn fit_rect(&mut self, rect: Rect, bounds: Option<KicadBoundingBox>) {
        self.fit_size(rect.size() - Vec2::splat(24.0), bounds, 6.0..=48.0);
    }

    fn fit_size(
        &mut self,
        size: Vec2,
        bounds: Option<KicadBoundingBox>,
        zoom_range: std::ops::RangeInclusive<f32>,
    ) {
        if let Some(bounds) = bounds {
            let width = bounds.width().max(1.0) as f32;
            let height = bounds.height().max(1.0) as f32;
            self.zoom = (size.x.max(1.0) / width)
                .min(size.y.max(1.0) / height)
                .clamp(*zoom_range.start(), *zoom_range.end());
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

    /// world to screen。
    pub(crate) fn world_to_screen(self, rect: Rect, point: KicadPoint) -> Pos2 {
        rect.center() + self.pan + Vec2::new(point.x as f32 * self.zoom, point.y as f32 * self.zoom)
    }

    /// screen to world。
    pub(crate) fn screen_to_world(self, rect: Rect, point: Pos2) -> KicadPoint {
        let local = point - rect.center() - self.pan;
        KicadPoint {
            x: (local.x / self.zoom) as f64,
            y: (local.y / self.zoom) as f64,
        }
    }

    /// zoom around。
    pub(crate) fn zoom_around(&mut self, rect: Rect, screen_point: Pos2, zoom_delta: f32) {
        let before = self.screen_to_world(rect, screen_point);
        self.zoom = (self.zoom * zoom_delta).clamp(MIN_ZOOM, MAX_ZOOM);
        let after_screen = self.world_to_screen(rect, before);
        self.pan += screen_point - after_screen;
    }

    /// visible world bounds。
    pub(crate) fn visible_world_bounds(self, rect: Rect) -> KicadBoundingBox {
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

/// item visible。
pub(crate) fn item_visible(
    bounds: Option<KicadBoundingBox>,
    visible_bounds: KicadBoundingBox,
) -> bool {
    // Scene geometry stays in osl-kicad; the GUI only decides whether an item can affect this viewport.
    bounds.is_none_or(|bounds| bounds.intersects(visible_bounds))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
