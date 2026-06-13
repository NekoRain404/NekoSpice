//! Domain-focused tests for nsp-schema.

use crate::{NspBoundingBox, NspPoint};

#[test]
fn bounding_boxes_report_intersections() {
    let bounds = NspBoundingBox {
        min: NspPoint { x: 10.0, y: 20.0 },
        max: NspPoint { x: 30.0, y: 40.0 },
    };
    assert!(bounds.contains(NspPoint { x: 20.0, y: 30.0 }));
    assert!(bounds.intersects(NspBoundingBox {
        min: NspPoint { x: 25.0, y: 35.0 },
        max: NspPoint { x: 45.0, y: 55.0 },
    }));
    assert!(!bounds.intersects(NspBoundingBox {
        min: NspPoint { x: 31.0, y: 41.0 },
        max: NspPoint { x: 45.0, y: 55.0 },
    }));
}
