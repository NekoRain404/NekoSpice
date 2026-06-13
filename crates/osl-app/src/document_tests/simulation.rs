//! 文档仿真测试 — 仿真指令设置和网表预览。

use crate::document::KicadGuiDocument;
use osl_kicad::KicadSimulationDirectiveKind;

/// 设置仿真指令后，验证指令可见、检查报告计数和网表包含。
#[test]
fn document_exposes_simulation_preview_for_gui_panel() {
    let temp = crate::test_support::temp_schematic_copy("gui_simulation_preview");
    let temp_path = temp.path();

    let mut document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
    document
        .set_simulation_directive(
            KicadSimulationDirectiveKind::Tran,
            "2u 2m".to_string(),
            None,
        )
        .unwrap();

    let directives = document.simulation_directives();
    assert!(
        directives
            .iter()
            .any(|directive| directive.text == ".tran 2u 2m")
    );
    let report = document.check_report();
    assert_eq!(report.spice_directive_count, directives.len());
    let netlist = document.spice_netlist_preview().unwrap();
    assert!(netlist.contains(".tran 2u 2m"));
    assert!(netlist.ends_with(".end\n"));
}

/// 网表必须包含 `.end` 结束指令。
#[test]
fn document_netlist_includes_ground_and_end() {
    let temp = crate::test_support::temp_schematic_copy("gui_netlist_gnd");
    let temp_path = temp.path();

    let document = KicadGuiDocument::load(temp_path.to_path_buf()).unwrap();
    let netlist = document.spice_netlist_preview().unwrap();
    assert!(netlist.contains(".end\n"), "netlist must end with .end");
}
