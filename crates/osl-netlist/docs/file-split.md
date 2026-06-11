# osl-netlist 文件拆分说明

## 背景

原始 `lib.rs` 长达 ~1866 行，混合了类型定义和解析逻辑。
拆分为类型定义和解析实现两个部分。

## 文件结构

```
crates/osl-netlist/src/
├── lib.rs                    # 模块声明、re-export、import 函数（~713 行）
├── netlist_parse_impl.rs     # parse_netlist 及项目规范化逻辑（~1156 行）
├── kicad_import.rs           # KiCad 原理图导入（原有）
└── ltspice_import.rs         # LTspice 导入（原有）
```

## 各文件职责

### lib.rs
- 模块声明和 re-export
- `ImportInput` / `ImportReport` 等核心类型定义
- `read_import_input` — 输入读取
- 保持公共 API 向后兼容

### netlist_parse_impl.rs
- `parse_netlist` — 网表解析入口
- 项目规范化和依赖解析
- 诊断信息生成
- 信号建议和检查模板
