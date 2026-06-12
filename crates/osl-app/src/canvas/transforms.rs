//! 画布坐标变换工具。提供引脚文本偏移计算和符号属性点的坐标变换。
//!
//! 所有坐标变换遵循 KiCad 的变换顺序：镜像 → 旋转 → 平移。

use eframe::egui::Align2;
use osl_kicad::{KicadAt, KicadPoint};

/// 根据引脚方向计算引脚名称或编号的文本偏移量与对齐方式。
///
/// `is_name=true` 将文本置于引脚主体端；`is_name=false` 置于外部端。
/// 返回值：(偏移x, 偏移y, 对齐方式)。
pub(crate) fn pin_text_offsets(
    start: &KicadPoint,
    end: &KicadPoint,
    is_name: bool,
) -> (f64, f64, Align2) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < 1e-6 {
        return (1.0, -1.5, Align2::LEFT_TOP);
    }

    // 归一化方向向量
    let nx = dx / dist;
    let ny = dy / dist;

    // 垂直偏移（始终在引脚方向的"右侧"）
    let perp_x = ny;
    let perp_y = -nx;
    let offset = 0.8;

    if is_name {
        // 名称置于主体端，沿垂直方向偏移
        let align = if perp_x.abs() > perp_y.abs() {
            if perp_x > 0.0 { Align2::LEFT_CENTER } else { Align2::RIGHT_CENTER }
        } else {
            if perp_y > 0.0 { Align2::CENTER_TOP } else { Align2::CENTER_BOTTOM }
        };
        (perp_x * offset, perp_y * offset, align)
    } else {
        // 编号置于外部端，沿垂直方向反向偏移
        let align = if perp_x.abs() > perp_y.abs() {
            if perp_x > 0.0 { Align2::RIGHT_CENTER } else { Align2::LEFT_CENTER }
        } else {
            if perp_y > 0.0 { Align2::CENTER_BOTTOM } else { Align2::CENTER_TOP }
        };
        (-perp_x * offset, -perp_y * offset, align)
    }
}

/// 将元件本地坐标点变换到原理图全局坐标。
///
/// 变换顺序：镜像 → 旋转 → 平移（与 KiCad 一致）。
pub(crate) fn transform_property_point(
    local: KicadPoint,
    symbol_at: KicadAt,
    mirror: Option<&str>,
) -> KicadPoint {
    // 1. 应用镜像
    let mut mirrored = local;
    if let Some(mirror_str) = mirror {
        if mirror_str.contains('x') {
            mirrored.y = -mirrored.y;
        }
        if mirror_str.contains('y') {
            mirrored.x = -mirrored.x;
        }
    }

    // 2. 应用旋转（归一化到 0-360°）
    let rotation = symbol_at.rotation % 360.0;
    let normalized = if rotation < 0.0 { rotation + 360.0 } else { rotation };
    let rotated = match normalized.round() as i32 {
        0 => mirrored,
        90 => KicadPoint { x: -mirrored.y, y: mirrored.x },
        180 => KicadPoint { x: -mirrored.x, y: -mirrored.y },
        270 => KicadPoint { x: mirrored.y, y: -mirrored.x },
        _ => {
            let radians = rotation.to_radians();
            KicadPoint {
                x: mirrored.x * radians.cos() - mirrored.y * radians.sin(),
                y: mirrored.x * radians.sin() + mirrored.y * radians.cos(),
            }
        }
    };

    // 3. 平移到符号位置
    KicadPoint {
        x: symbol_at.x + rotated.x,
        y: symbol_at.y + rotated.y,
    }
}
