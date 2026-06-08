# NekoSpice 三天冲刺计划

用户给定时间窗口：3 天。

三天内不能完成完整替代 LTspice / KiCad，但可以完成一个有竞争力的垂直切片：用 ngspice 作为求解后端，交付自动化批量仿真、可追溯运行元数据、HTML/JSON 报告和 CI 友好的 pass/fail 命令。这一切是 LTspice 手工工作流最弱的部分，也是后续扩展测量、sweep、模型诊断和 GUI 的地基。

## Day 1：跑起来

目标：

- 初始化 Rust workspace。
- 实现 `osl` CLI。
- 实现 `osl run <netlist.cir>`。
- 调用 ngspice CLI。
- 输出 `run.json`、`ngspice.log`、`stdout.txt`、`stderr.txt`、`report.html`。
- 准备 RC、整流、RLC 示例电路。

验收：

```bash
cargo build --workspace
cargo run -p osl-cli -- run examples/rc_filter/rc.cir --output runs/rc_001
```

## Day 2：批量验证

目标：

- 实现 `osl verify <project.osl.yaml>`。
- 支持一个最小 YAML 子集：`project` 和 `runs`。
- 批量运行多个 netlist。
- 支持 ngspice ASCII raw 解析。
- 支持最小测量检查：`final_value`、`avg`、`min`、`max`、`pp`、`rms`。
- 支持测量窗口：`from` / `to`，并支持 `ms`、`us`、`ns`、`k`、`Meg` 等后缀。
- checks 可以读取 raw 变量表中的任意信号，例如 `v(out)`、`i(v1)`。
- 支持最小 sweep expansion，例如 `rload: [500, 1000, 2000]` 自动展开多次运行。
- 支持 `--jobs <n>` 并发执行独立验证任务，并保持报告顺序稳定。
- 输出 `verify.json` 和 `report.html`。
- 失败时返回非零退出码。

验收：

```bash
cargo run -p osl-cli -- verify examples/basic_validation.osl.yaml --output reports/basic_001
```

## Day 3：工程化闭环

目标：

- 实现 `osl bench <directory>`。
- 补充文档和使用命令。
- 建立 Git 工程。
- 固化三天后下一步任务：measurement、sweep、raw parser、模型诊断。

验收：

```bash
cargo fmt --check
cargo test --workspace
cargo run -p osl-cli -- bench examples --output bench-results/basic_001
```

## 三天后继续做什么

优先级从高到低：

1. richer YAML parser：替换当前最小子集解析器。
2. report failure drilldown：失败项跳转到具体 run 和波形。
3. binary raw parser：提升大型波形解析速度。
4. model-check：`.subckt` pin list、方言检测、unsupported directive。
5. waveform data layer：LOD、mmap、viewport query。
