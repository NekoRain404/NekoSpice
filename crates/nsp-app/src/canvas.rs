//! 画布渲染管线模块。组织原理图的图元渲染、坐标变换和场景绘制。
//!
//! 子模块：
//! - [`colors`] — 主题感知的画布颜色定义
//! - [`primitives`] — 底层绘制图元（网格、线条、图形、文本）
//! - [`transforms`] — 引脚文本偏移和符号坐标变换
//! - [`scene_renderer`] — 编排函数和结构层（sheets、graphics、symbols）
//! - [`scene_renderer_wires`] — 连接层（wires、buses）
//! - [`scene_renderer_annotations`] — 标注层（labels、text、junctions）
//! - [`hover`] — 悬停高亮绘制

pub(crate) mod colors;
pub(crate) mod hover;
mod primitives;
pub(crate) mod scene_renderer;
pub(crate) mod scene_renderer_annotations;
pub(crate) mod scene_renderer_wires;
pub(crate) mod transforms;

pub(crate) use hover::draw_hover_highlight;
pub(crate) use primitives::{draw_bounds, draw_grid, draw_line};
pub(crate) use scene_renderer::draw_scene;
