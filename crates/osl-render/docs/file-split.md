# osl-render 文件拆分说明

## 背景

原始 `lib.rs` 长达 ~1510 行，包含 SVG 渲染的全部实现。
将渲染逻辑提取到独立文件，lib.rs 保留公共 API 入口。

## 文件结构

```
crates/osl-render/src/
├── lib.rs                # 公共 API 入口（~35 行）
└── svg_render_impl.rs    # SVG 渲染完整实现（~1478 行）
```

## 各文件职责

### lib.rs
- `SvgRenderOptions` — 渲染选项配置
- `render_kicad_scene_svg` — 快捷渲染入口
- 保持公共 API 向后兼容

### svg_render_impl.rs
- `render_kicad_scene_svg_with_options` — 完整渲染入口
- `render_sheet` / `render_symbol` / `render_graphic` — 元素渲染
- `render_text_box` / `render_table` / `render_image` — 复合元素渲染
- SVG 辅助函数（颜色、描边、填充、坐标变换等）
