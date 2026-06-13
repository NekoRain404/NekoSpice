//! 文档测试模块根。按职责拆分为子模块，共享测试辅助函数定义在此。

mod editing;
mod items;
mod placement;
mod simulation;

use nsp_schema::{NspSymbolDef, parse_symbol_library};

/// 创建空的 `NspSymbolDef` 用于测试。
pub(super) fn test_symbol_definition(name: &str) -> NspSymbolDef {
    NspSymbolDef {
        name: name.to_string(),
        extends: None,
        power: None,
        body_styles: None,
        exclude_from_sim: None,
        in_bom: None,
        on_board: None,
        in_pos_files: None,
        duplicate_pin_numbers_are_jumpers: None,
        jumper_pin_groups: Vec::new(),
        embedded_fonts: None,
        pin_names: None,
        pin_numbers: None,
        unit_names: Default::default(),
        properties: Vec::new(),
        graphics: Vec::new(),
        pins: Vec::new(),
    }
}

/// 创建带引脚交替属性的 `NspSymbolDef` 用于测试。
pub(super) fn test_symbol_with_alternate(name: &str) -> NspSymbolDef {
    parse_symbol_library(
        &format!(
            r#"(nsp_symbol_lib
  (version 20230121)
  (symbol "{name}"
    (property "Reference" "U" (at 0 0 0))
    (property "Value" "Alt" (at 0 -2.54 0))
    (symbol "Alt_0_1"
      (pin passive line (at -2.54 0 0) (length 2.54) (name "~") (number "1"))
      (pin passive line (at 2.54 0 180) (length 2.54) (name "~") (number "2") (alternate "ALT2" output line))
    )
  )
)"#
        ),
        "gui_alt_symbol.nsp_sym",
    )
    .unwrap()
    .symbol(name)
    .unwrap()
    .clone()
}
