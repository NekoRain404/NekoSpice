# KicadSchematic impl 拆分说明

## 背景

原始 `lib.rs` 中 `impl KicadSchematic` 块长达 ~1750 行，包含编辑、库解析、诊断和工具方法。
为提升可维护性和代码可读性，将其按职责拆分为 4 个独立文件，通过 `include!` 引入。

## 文件结构

```
crates/osl-schema/src/
├── lib.rs                         # 模块声明、re-export、struct 定义（~220 行）
├── schematic_edit_impl.rs         # 编辑与放置操作（~920 行）
├── schematic_library_impl.rs      # 符号库解析与定义合并（~260 行）
├── schematic_check_impl.rs        # 原理图诊断检查（~380 行）
└── schematic_util_impl.rs         # 工具方法与内部辅助（~200 行）
```

## 各文件职责

### schematic_edit_impl.rs
- `apply_edit` — 统一编辑入口
- `move_symbol` / `move_item_by_uuid` — 元件移动
- `delete_item_by_uuid` — 元件删除
- `configure_symbol` / `set_symbol_property` — 符号属性编辑
- `place_symbol` — 符号放置
- `add_wire` / `add_bus` / `add_bus_entry` / `add_junction` / `add_no_connect` / `add_label` / `add_text` / `add_sheet` — 图形元素添加

### schematic_library_impl.rs
- `configured_symbol_pin_refs` — 引用解析
- `connectivity_graph` / `canvas_scene` — 场景生成
- `check_report` — 诊断报告
- `symbol_definition` / `resolved_symbol_definition` — 符号定义查找
- `resolve_project_symbol_libraries` / `resolve_missing_symbol_definitions_from_table` — 库解析
- `merge_library_symbol` / `merge_symbol_placement_library_symbol` / `merge_library_symbol_with_parents` — 库合并

### schematic_check_impl.rs
- `check_duplicate_references` — 重复引用检查
- `check_symbols` — 符号完整性检查
- `check_wires` / `check_labels` / `check_sheets` / `check_no_connects` / `check_buses` — 各类元素检查
- `check_spice_directives` — SPICE 指令检查

### schematic_util_impl.rs
- `symbol_index_by_reference` — 按引用名查找符号
- `edit_uuid` / `edit_uuid_excluding` — UUID 编辑辅助
- `used_uuids` — 已用 UUID 收集
- `symbol_pin_points` / `sheet_pin_points` — 引脚坐标
- `has_no_connect_at` — 无连接标记查询

## 技术说明

- 使用 `include!` 宏保持与 `lib.rs` 相同的作用域（共享所有 `use` 导入）。
- 每个文件包含完整的 `impl KicadSchematic { ... }` 块。
- Rust 允许同一类型在同模块内有多个 `impl` 块。
