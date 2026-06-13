//! 文档放置测试 — 符号放置、引脚交替和属性编辑。

use crate::document::NspGuiDocument;
use crate::document_ops::reference_prefix;
use crate::placement_config::SymbolPlacementConfig;
use nsp_schema::NspAt;

use super::{test_symbol_definition, test_symbol_with_alternate};

/// 从库定义放置符号，验证自动递增引用编号和脏标记。
#[test]
fn document_places_library_symbol_with_next_reference() {
    let temp = crate::test_support::temp_schematic_copy("gui_place");
    let temp_path = temp.path();

    let mut document = NspGuiDocument::load(temp_path.to_path_buf()).unwrap();
    let definition = document
        .schematic
        .library_symbols
        .iter()
        .find(|symbol| symbol.name == "NekoSpice:R")
        .cloned()
        .unwrap();

    let placement = document
        .place_symbol_from_definition(
            definition,
            Vec::new(),
            NspAt {
                x: 101.6,
                y: 50.8,
                rotation: 0.0,
            },
            SymbolPlacementConfig::default(),
        )
        .unwrap();

    assert_eq!(placement.summary.operation, "place-symbol");
    assert_eq!(placement.summary.target, "R2 NekoSpice:R");
    assert_eq!(placement.reference, "R2");
    assert_eq!(placement.lib_id, "NekoSpice:R");
    assert!(document.is_dirty());
    assert!(
        document
            .scene()
            .symbols
            .iter()
            .any(|symbol| symbol.reference == "R2")
    );
}

/// 放置符号时指定引脚交替，验证交替信息正确写入。
#[test]
fn document_places_symbol_with_selected_pin_alternate() {
    let temp = crate::test_support::temp_schematic_copy("gui_place_alt");
    let temp_path = temp.path();

    let mut document = NspGuiDocument::load(temp_path.to_path_buf()).unwrap();
    let definition = test_symbol_with_alternate("NekoSpice:Alt");
    let mut config = SymbolPlacementConfig::default();
    config
        .pin_alternates
        .insert("2".to_string(), "ALT2".to_string());

    let placement = document
        .place_symbol_from_definition(
            definition,
            Vec::new(),
            NspAt {
                x: 101.6,
                y: 50.8,
                rotation: 0.0,
            },
            config,
        )
        .unwrap();

    assert_eq!(placement.reference, "U1");
    let placed = document
        .schematic
        .symbols
        .iter()
        .find(|symbol| symbol.reference() == Some("U1"))
        .unwrap();
    assert_eq!(placed.pins[1].alternate.as_deref(), Some("ALT2"));
}

/// 通过属性编辑 API 修改符号属性和镜像，验证场景同步和持久化。
#[test]
fn document_sets_selected_symbol_properties_for_gui_editor() {
    let temp = crate::test_support::temp_schematic_copy("gui_properties");
    let temp_path = temp.path();

    let mut document = NspGuiDocument::load(temp_path.to_path_buf()).unwrap();
    document
        .set_symbol_property(
            "R1".to_string(),
            "Reference".to_string(),
            "RLOAD".to_string(),
        )
        .unwrap();
    document
        .set_symbol_property("RLOAD".to_string(), "Value".to_string(), "2k".to_string())
        .unwrap();
    document
        .configure_symbol_mirror("RLOAD".to_string(), Some("x y".to_string()))
        .unwrap();

    let scene = document.scene();
    let symbol = scene
        .symbols
        .iter()
        .find(|symbol| symbol.reference == "RLOAD")
        .unwrap();
    assert_eq!(symbol.value, "2k");
    assert_eq!(symbol.mirror.as_deref(), Some("x y"));
    assert!(document.is_dirty());
}

/// 符号引用前缀应忽略 schema 占位符后缀（如 "R?" → "R"）。
#[test]
fn symbol_reference_prefix_ignores_schema_placeholder_suffix() {
    let mut definition = test_symbol_definition("NekoSpice:R");
    definition.properties.push(nsp_schema::NspProperty {
        name: "Reference".to_string(),
        value: "R?".to_string(),
        id: None,
        at: None,
        hide: None,
        show_name: None,
        do_not_autoplace: None,
        effects: None,
    });

    assert_eq!(reference_prefix(&definition), "R");
}
