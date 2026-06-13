//! 文档图元测试 — 添加导线、总线、标签、子图纸等基本原理图图元。

use crate::document::NspGuiDocument;
use nsp_schema::{NspAt, NspLabelKind, NspPoint};

/// 使用工具面板 API 创建全套基本原理图图元并验证场景。
#[test]
fn document_adds_basic_schematic_items_for_gui_tools() {
    let temp = crate::test_support::temp_schematic_copy("gui_tools");
    let temp_path = temp.path();

    let mut document = NspGuiDocument::load(temp_path.to_path_buf()).unwrap();

    // 导线
    document
        .add_wire(vec![
            NspPoint { x: 101.6, y: 50.8 },
            NspPoint { x: 111.76, y: 50.8 },
        ])
        .unwrap();
    // 总线
    document
        .add_bus(vec![
            NspPoint { x: 101.6, y: 38.1 },
            NspPoint { x: 111.76, y: 38.1 },
        ])
        .unwrap();
    // 总线入口
    document
        .add_bus_entry(
            NspPoint { x: 111.76, y: 38.1 },
            nsp_schema::NspSize {
                width: 2.54,
                height: -2.54,
            },
        )
        .unwrap();
    // 全局标签
    document
        .add_label(
            "sense".to_string(),
            NspLabelKind::Global,
            NspAt {
                x: 111.76,
                y: 50.8,
                rotation: 0.0,
            },
        )
        .unwrap();
    // 层次标签
    document
        .add_label(
            "sheet_in".to_string(),
            NspLabelKind::Hierarchical,
            NspAt {
                x: 111.76,
                y: 55.88,
                rotation: 0.0,
            },
        )
        .unwrap();
    // 文本
    document
        .add_text(
            ".save v(out)".to_string(),
            NspAt {
                x: 45.72,
                y: 35.56,
                rotation: 0.0,
            },
        )
        .unwrap();
    // 仿真指令
    document
        .set_simulation_directive(
            nsp_schema::NspSimulationDirectiveKind::Tran,
            "2u 2m".to_string(),
            Some(NspAt {
                x: 45.72,
                y: 40.64,
                rotation: 0.0,
            }),
        )
        .unwrap();
    // 节点
    document
        .add_junction(NspPoint { x: 101.6, y: 50.8 })
        .unwrap();
    // 无连接标记
    document
        .add_no_connect(NspPoint { x: 111.76, y: 50.8 })
        .unwrap();
    // 子图纸
    document
        .add_sheet(
            "gain_stage".to_string(),
            "gain_stage.kicad_sch".to_string(),
            NspAt {
                x: 120.0,
                y: 40.0,
                rotation: 0.0,
            },
            nsp_schema::NspSize {
                width: 25.4,
                height: 12.7,
            },
            vec![
                nsp_schema::NspSheetPin {
                    name: "in".to_string(),
                    pin_type: "input".to_string(),
                    at: Some(NspAt {
                        x: 120.0,
                        y: 46.35,
                        rotation: 180.0,
                    }),
                    uuid: None,
                    effects: None,
                },
                nsp_schema::NspSheetPin {
                    name: "out".to_string(),
                    pin_type: "output".to_string(),
                    at: Some(NspAt {
                        x: 145.4,
                        y: 46.35,
                        rotation: 0.0,
                    }),
                    uuid: None,
                    effects: None,
                },
            ],
        )
        .unwrap();

    let scene = document.scene();
    assert!(document.is_dirty());
    assert_eq!(scene.wires.len(), 4);
    assert_eq!(scene.buses.len(), 1);
    assert_eq!(scene.bus_entries.len(), 1);
    assert_eq!(scene.sheets.len(), 1);
    assert_eq!(scene.sheets[0].name, "gain_stage");
    assert_eq!(scene.sheets[0].pins.len(), 2);
    assert!(
        scene
            .labels
            .iter()
            .any(|label| { label.text == "sense" && label.kind == NspLabelKind::Global })
    );
    assert!(
        scene
            .labels
            .iter()
            .any(|label| { label.text == "sheet_in" && label.kind == NspLabelKind::Hierarchical })
    );
    assert!(
        scene
            .text_items
            .iter()
            .any(|text| text.text == ".save v(out)")
    );
    assert!(
        scene
            .text_items
            .iter()
            .any(|text| text.text == ".tran 2u 2m")
    );
    assert!(scene.junctions.iter().any(|junction| {
        (junction.at.x - 101.6).abs() < 1e-6 && (junction.at.y - 50.8).abs() < 1e-6
    }));
    assert!(scene.no_connects.iter().any(|marker| {
        (marker.at.x - 111.76).abs() < 1e-6 && (marker.at.y - 50.8).abs() < 1e-6
    }));
}
