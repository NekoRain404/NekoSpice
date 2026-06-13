# osl-cli 文件拆分说明

## 背景

原始 `main.rs` 长达 ~1920 行，包含 CLI 调度、Schema 命令和验证基础设施。
按职责拆分为 3 个文件，通过 `include!` 引入。

## 文件结构

```
crates/osl-cli/src/
├── main.rs           # CLI 入口、命令调度、辅助函数（~820 行）
├── cli_schema.rs      # Schema 子命令（~310 行）
└── cli_verify.rs     # verify 命令基础设施（~790 行）
```

## 各文件职责

### main.rs
- `main` / `run_cli` / `run_command` — CLI 入口与命令分发
- `print_help` — 帮助文本
- `bench_command` / `model_check_command` / `import_command` / `waveform_command` — 独立命令
- `report_command` — 报告生成
- `positional` / `flag_value` / `has_flag` — 参数解析辅助

### cli_schema.rs
- `schema_inspect_command` — 原理图检查
- `schema_select_command` — 符号选择
- `schema_check_command` — 诊断检查
- `schema_export_command` — 导出
- `schema_edit_command` — 编辑操作
- `schema_render_command` — SVG 渲染
- `copy_import_dependencies` — 依赖复制

### cli_verify.rs
- `VerifyConfig` / `VerifyRun` / `VerifyTask` / `VerifyCheck` — 数据结构
- `VerifyConfigYaml` / `VerifyRunYaml` / `VerifyCheckYaml` — YAML 反序列化
- `validate_run` / `validate_check` — 输入验证
- `run_verify_tasks` / `run_verify_task` — 任务执行
- `YamlNumber` — 数值解析
