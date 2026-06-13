//! 文档编辑测试 — 删除和移动操作的往返验证。

use crate::document::NspGuiDocument;
use nsp_schema::NspPoint;
use std::fs;

/// 删除操作：验证删除后场景更新、脏标记设置和持久化写回。
#[test]
fn document_deletes_selected_uuid_and_saves_schematic() {
    let temp = crate::test_support::temp_schematic_copy("gui_delete");
    let temp_path = temp.path();

    let mut document = NspGuiDocument::load(temp_path.to_path_buf()).unwrap();
    assert!(!document.is_dirty());
    assert_eq!(document.scene().wires.len(), 3);

    let summary = document
        .delete_item("22222222-2222-2222-2222-222222222222")
        .unwrap();
    assert_eq!(summary.operation, "delete-wire");
    assert!(document.is_dirty());
    assert_eq!(document.scene().wires.len(), 2);

    document.save().unwrap();
    assert!(!document.is_dirty());
    let saved = fs::read_to_string(temp_path).unwrap();
    assert!(!saved.contains("22222222-2222-2222-2222-222222222222"));
}

/// 移动操作：验证移动后坐标偏移、命中测试可达性和持久化。
#[test]
fn document_moves_selected_uuid_and_keeps_canvas_hit_addressable() {
    let temp = crate::test_support::temp_schematic_copy("gui_move");
    let temp_path = temp.path();

    let mut document = NspGuiDocument::load(temp_path.to_path_buf()).unwrap();
    let original_hit = document
        .scene()
        .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
        .unwrap();

    let summary = document
        .move_item(
            "22222222-2222-2222-2222-222222222222",
            NspPoint { x: 2.54, y: 0.0 },
        )
        .unwrap();
    assert_eq!(summary.operation, "move-wire");
    assert!(document.is_dirty());

    let moved_hit = document
        .scene()
        .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
        .unwrap();
    assert!((moved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
    assert_eq!(moved_hit.kind, "wire");

    document.save().unwrap();
    assert!(!document.is_dirty());
    let reloaded_scene = nsp_schema::read_schematic_with_libraries(temp_path)
        .unwrap()
        .canvas_scene();
    let saved_hit = reloaded_scene
        .item_hit_by_uuid("22222222-2222-2222-2222-222222222222")
        .unwrap();
    assert_eq!(saved_hit.kind, "wire");
    assert!((saved_hit.bounds.min.x - original_hit.bounds.min.x - 2.54).abs() < 1e-6);
    assert!((saved_hit.bounds.min.y - original_hit.bounds.min.y).abs() < 1e-6);
}
