//! nsp-schema 集成测试。按功能域拆分为独立模块。

mod tests_canvas;
mod tests_core;
mod tests_editing;
mod tests_geometry;
mod tests_hierarchy;
mod tests_index;
mod tests_items;
mod tests_labels;
mod tests_library;
mod tests_metadata;
mod tests_sexpr;
mod tests_simulation;
mod tests_spice;

pub(super) fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {actual} to be close to {expected}"
    );
}
