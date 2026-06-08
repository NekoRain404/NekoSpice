# NekoSpice 三天冲刺计划

用户给定时间窗口：3 天。

三天内不能完成完整替代 LTspice / KiCad，但可以完成一个有竞争力的垂直切片：用 ngspice 作为求解后端，交付自动化批量仿真、可追溯运行元数据、HTML/JSON 报告和 CI 友好的 pass/fail 命令。这一切是 LTspice 手工工作流最弱的部分，也是后续扩展测量、sweep、模型诊断和 GUI 的地基。

## Day 1：跑起来

目标：

- 初始化 Rust workspace。
- 实现 `osl` CLI。
- 实现 `osl run <netlist.cir>`。
- 调用 ngspice CLI。
- 输出 `run.json`、`ngspice.log`、`stdout.txt`、`stderr.txt`、`waveform.raw`、`waveform.csv`、`waveform-summary.json`、`report.html`。
- 准备 RC、整流、RLC 示例电路。

验收：

```bash
cargo build --workspace
cargo run -p osl-cli -- run examples/rc_filter/rc.cir --output runs/rc_001
```

## Day 2：批量验证

目标：

- 实现 `osl verify <project.osl.yaml>`。
- 使用 `serde_yaml` 解析验证配置，支持标准 YAML map/list、flow-style 写法、quoted 字符串和带 SPICE 后缀的数值字符串。
- 批量运行多个 netlist。
- 支持 ngspice binary / ASCII raw 自动解析。
- 支持最小测量检查：`final_value`、`avg`、`min`、`max`、`pp`、`rms`。
- 支持测量窗口：`from` / `to`，并支持 `ms`、`us`、`ns`、`k`、`Meg` 等后缀。
- checks 可以读取 raw 变量表中的任意信号，例如 `v(out)`、`i(v1)`。
- 支持最小 sweep expansion，例如 `rload: [500, 1000, 2000]` 自动展开多次运行。
- 支持 `--jobs <n>` 并发执行独立验证任务，并保持报告顺序稳定。
- 报告支持失败摘要和 artifact drilldown：失败 check、参数组合、窗口波形摘要、`run.json`、`waveform.raw`、`waveform.csv`、`waveform-summary.json`、`ngspice.log`、`input.cir`。
- 输出 `verify.json` 和 `report.html`。
- 失败时返回非零退出码。

验收：

```bash
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --output reports/basic_001
```

## Day 3：工程化闭环

目标：

- 实现 `osl bench <directory>`。
- 实现 `osl model-check <netlist-or-directory>` 的最小模型诊断闭环。
- 输出 `.subckt` pin list、`.model` 索引、unsupported directive、方言风险和兼容性评分。
- 支持 LTspice `.asy` symbol pin mapping：解析 `PINATTR PinName` / `SpiceOrder` 并对齐 `.subckt` pin list。
- 实现 `osl import <spice-netlist>` 的导入报告和 normalized project 输出：组件数量、symbol 数量、directive 数量、include、兼容性评分、`project/input.cir`、`project/project.osl.yaml`、`project/manifest.json`，并复制相对 `.include` / `.lib` 模型依赖。
- 准备一个 KiCad-style SPICE netlist fixture，并确认它可以被 ngspice 运行。
- 实现 `osl waveform <waveform.raw>` 的视窗 min/max envelope JSON 查询，为后续 GUI 波形查看器提供数据接口。
- 补充文档和使用命令。
- 建立 Git 工程。
- 固化三天后下一步任务：measurement、sweep、KiCad/LTspice 规范化导入、波形数据层。

验收：

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- verify examples/structured_validation.osl.yaml --jobs 3 --output reports/structured_001
cargo run -p osl-cli -- bench examples --output bench-results/basic_001
cargo run -p osl-cli -- model-check examples/diode_rectifier/rectifier.cir --output reports/modelcheck_001
cargo run -p osl-cli -- model-check examples/pin_mapping/good_opamp.lib --symbol examples/pin_mapping/good_opamp.asy --output reports/pinmap_001
cargo run -p osl-cli -- import examples/kicad_import/kicad_rc.cir --output reports/import_001
cargo run -p osl-cli -- verify reports/import_001/project/project.osl.yaml --output reports/import_001_verify
cargo run -p osl-cli -- import examples/kicad_import/kicad_diode_include.cir --output reports/import_models_001
cargo run -p osl-cli -- verify reports/import_models_001/project/project.osl.yaml --output reports/import_models_001_verify
cargo run -p osl-cli -- run examples/kicad_import/kicad_rc.cir --output runs/kicad_rc_001
cargo run -p osl-cli -- waveform runs/kicad_rc_001/waveform.raw --signal 'v(out)' --points 100 --output reports/kicad_vout_envelope.json
```

## 三天后继续做什么

优先级从高到低：

1. normalized import v2：解析 LTspice `.asc` / KiCad project metadata，并生成更完整的 checks 模板。
2. waveform data layer：持久 LOD cache、mmap、大文件 viewport query 优化。
3. richer verification DSL：backend、analysis、corner、Monte Carlo 和 worst-case search。
