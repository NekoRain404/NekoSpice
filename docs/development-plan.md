# NekoSpice / OpenSpiceLab-RS 开发计划

本文档把 [dev.md](./dev.md) 中的技术设想整理成可执行开发计划。目标不是第一阶段重写 SPICE 求解器，也不是复制 LTspice 或 KiCad，而是先做一个工程级仿真验证平台：复用成熟求解后端，重点突破批量验证、高性能波形、模型诊断、导入工作流、报告和 CI。

## 1. 产品目标

### 1.1 核心定位

NekoSpice 是 Rust-first 的开源 SPICE 仿真验证平台，第一年目标是在以下方面超过传统桌面仿真工作流：

- 自动化验证：让电路仿真像软件测试一样可重复运行。
- 批量仿真：内置 sweep、corner、Monte Carlo、worst-case search。
- 高性能波形：支持百万到千万点波形的快速加载、缩放、查询和叠加。
- 模型诊断：主动发现模型方言、pin mapping、unsupported directive 等问题。
- 工作流导入：导入 LTspice / KiCad 工作流，并生成可运行 netlist 和兼容性报告。
- 工程报告：输出 HTML、JSON、JUnit XML，支持 CI 和团队评审。

第一阶段的竞争策略是：

```text
不在 SPICE 求解速度上硬刚 LTspice；
先在自动验证、批量执行、波形数据层、模型诊断和 CI 报告上形成优势。
```

### 1.2 非目标

v0.x 阶段不做以下事情：

- 自研完整 SPICE 求解器。
- 完整原理图编辑器或 PCB 设计工具。
- Electron / Qt 前端。
- GPU SPICE 求解器。
- 100% LTspice / KiCad 项目兼容。

这些不是永久放弃，而是避免第一年分散主线。

## 2. 当前仓库状态

截至本计划编写时，当前工作区只有文档基础：

- `docs/dev.md`：总体技术设想、模块划分、路线图和 MVP 范围。
- `docs/development-plan.md`：本文档，作为可执行计划和验收依据。

当前尚未落地：

- Rust workspace。
- CLI 二进制。
- ngspice runner。
- 示例电路、测试数据、benchmark。
- 有效 Git 仓库元数据。

因此下一步应该从项目骨架和最小仿真闭环开始，而不是直接做 GUI。

## 3. 架构边界

### 3.1 主要 crate

第一轮 workspace 使用以下 crate 边界：

- `osl-core`：项目模型、任务模型、分析配置、错误类型、基础单位。
- `osl-cli`：命令行入口，提供 `run`、`verify`、`bench`、`model-check`、`report`。
- `osl-sim`：仿真后端 trait 和 ngspice CLI 实现。
- `osl-netlist`：SPICE/LTspice/KiCad netlist 解析、规范化和导入。
- `osl-waveform`：raw 解析、波形列存、CSV/JSON 导出、LOD 和视窗查询。
- `osl-measure`：测量表达式、测量函数、pass/fail evaluator。
- `osl-experiment`：sweep、corner、Monte Carlo、任务调度和结果聚合。
- `osl-report`：HTML、JSON、Markdown、JUnit XML。
- `osl-bench`：性能基准、测试电路集合和结果记录。
- `osl-app`：后期 GUI shell，先不进入 v0.1 主线。
- `osl-render`：后期 wgpu 波形渲染，先不进入 v0.1 主线。

### 3.2 后端策略

后端必须通过统一 trait 解耦：

```rust
pub trait SimulatorBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> BackendCapabilities;
    fn run(
        &self,
        job: SimulationJob,
        sink: &mut dyn SimulationSink,
    ) -> Result<SimulationResult, SimError>;
}
```

阶段顺序：

- v0.1-v0.3：ngspice CLI，优先稳定、进程隔离、跨平台。
- v0.6+：Xyce CLI，用于大规模电路和后端对比。
- v1.0+：ngspice shared library，用于实时波形、运行中取消和 GUI 内嵌仿真。
- 长期：自研局部快速分析器和远程 worker。

### 3.3 数据策略

数据层遵守以下边界：

- SQLite 只存项目元数据、任务记录和索引，不存大波形。
- 波形主路径使用 Arrow / Parquet / mmap cache。
- GUI 只查询当前视窗所需数据。
- 大波形必须支持 lazy loading、LOD、min/max envelope。
- 所有关键路径必须可 benchmark。

## 4. 版本路线图

### 4.1 v0.1：最小仿真闭环

周期：4 周。

目标：

- 建立 Rust workspace。
- 建立 CLI、日志、错误系统。
- 调用 ngspice CLI 运行 `.cir`。
- 记录仿真耗时和 ngspice 日志。
- 解析 ngspice raw 初版。
- 导出 CSV / JSON。
- 提供基础 benchmark。
- 准备至少 10 个基础示例电路。

必需命令：

```bash
osl --version
osl run examples/rc_filter/rc.cir --output runs/rc_001
osl run examples/diode_rectifier/rectifier.cir --output runs/rectifier_001
osl bench benchmarks/basic
```

验收标准：

- Linux 本机可构建并通过 `cargo fmt`、`cargo clippy`、`cargo test`。
- `osl run` 能稳定运行 RC、RLC、整流、简单放大器等基础电路。
- 每次运行生成 `run.json`、`ngspice.log`、`waveform.csv`。
- `run.json` 记录 backend、输入文件、退出码、耗时、输出路径。
- raw parser 对 v0.1 测试数据可重复解析。
- raw 解析速度有 benchmark 记录。

### 4.2 v0.2：验证 DSL 与测量引擎

周期：1-2 个月。

目标：

- YAML 验证配置。
- sweep expansion。
- measurement expression parser。
- 内置 `avg`、`min`、`max`、`pp`、`rms`。
- pass/fail evaluator。
- HTML / JSON report。

必需命令：

```bash
osl verify examples/buck_converter/validation.yaml --output reports/buck_001
```

验收标准：

- YAML 可声明 backend、analysis、sweep、checks。
- 27 组 sweep 可并行执行。
- 报告中显示 Passed / Failed / Worst case。
- 失败项可以追溯到具体参数组合和波形文件。
- JSON report 可被 CI 或脚本读取。

### 4.3 v0.3：高性能波形数据层

周期：2-3 个月。

目标：

- Arrow / Parquet 存储。
- mmap cache。
- LOD cache。
- min/max downsampling。
- viewport query engine。

验收标准：

- 1M 点波形秒级加载。
- 10M 点波形可视窗查询。
- min/max envelope 不丢尖峰。
- 100 条 sweep 曲线可以按需查询。
- 查询 API 可供 GUI 和报告复用。

### 4.4 v0.4：GPU 波形查看器

周期：3-4 个月。

目标：

- winit + egui + wgpu app shell。
- waveform renderer。
- 缩放、平移、游标、marker。
- 多曲线叠加。
- sweep 分组显示。

验收标准：

- 1M 点波形缩放流畅。
- 10M 点波形可交互浏览。
- 27 组 sweep 可叠图。
- 失败检查可跳转到对应波形。
- GUI 不阻塞后台仿真任务。

### 4.5 v0.5：模型兼容与诊断

周期：4-6 个月。

目标：

- `.model` / `.subckt` 解析。
- 模型库索引。
- 方言检测。
- pin mapping 检查。
- unsupported directive / function 诊断。
- 兼容性评分和修复建议。

验收标准：

- 厂商模型导入后能显示 subckt 名称、pin list、模型类型和兼容性评分。
- pin 数量或顺序不匹配时给出明确诊断。
- 不支持语法必须定位到文件和行号。
- model-check 报告可导出 JSON / HTML。

### 4.6 v0.6：LTspice / KiCad 工作流导入

周期：6-8 个月。

目标：

- LTspice `.asc` 基础解析。
- LTspice `.asy` pin / attribute 解析。
- KiCad SPICE netlist 导入。
- 导入兼容性报告。
- 输出 ngspice 可运行 netlist。

验收标准：

- 常见 LTspice 模拟电路可导入并运行。
- KiCad 导出的 SPICE netlist 可运行。
- 不能运行时必须明确指出原因和修复建议。
- 导入报告包含组件数量、symbol 数量、directive 数量、兼容性评分。

### 4.7 v0.7：高级验证与 CI

周期：8-12 个月。

目标：

- Monte Carlo。
- corner analysis。
- tolerance analysis。
- worst-case search。
- JUnit XML。
- GitHub Actions 示例。
- 多后端任务调度。

验收标准：

- 用户可以把电路仿真作为 CI 测试运行。
- Monte Carlo 可输出良率和统计分布。
- worst-case search 可定位失败参数组合。
- 多后端结果可以对比并报告差异。

## 5. 近期执行计划

### 5.1 第 1 周

- 初始化有效 Git 仓库，建立 `.gitignore`。
- 创建 Cargo workspace。
- 创建 `crates/osl-cli`、`crates/osl-core`、`crates/osl-sim`。
- 建立 `clap` CLI 框架。
- 建立 `tracing` 日志。
- 建立 `thiserror` / `anyhow` 错误边界。
- 准备 `examples/rc_filter/rc.cir`。
- 实现 ngspice 可执行文件探测。

完成定义：

- `cargo build` 成功。
- `osl --version` 可运行。
- `osl run examples/rc_filter/rc.cir` 至少能调用 ngspice 并保存日志。

### 5.2 第 2 周

- 实现 `SimulationJob`、`SimulationResult`、`BackendCapabilities`。
- 实现 `NgspiceCliBackend`。
- 建立运行目录结构 `runs/<run_id>/`。
- 输出 `run.json`。
- 记录退出码、耗时、stdout、stderr、ngspice log。
- 建立 `osl bench benchmarks/basic` 命令骨架。

完成定义：

- 每次运行有可追溯 metadata。
- ngspice 异常退出时 CLI 返回清晰错误。
- benchmark 可以记录至少一个基础用例耗时。

### 5.3 第 3 周

- 实现 ngspice raw parser 初版。
- 支持 transient 数据中的 time 和 `V(node)`。
- 实现 `Waveform`、`Signal`、`WaveColumn`。
- 导出 `waveform.csv`。
- 添加 raw parser unit tests。

完成定义：

- RC transient 波形可解析。
- CSV 列名、单位和行数稳定。
- parser 对 golden raw 文件通过测试。

### 5.4 第 4 周

- 支持 OP / DC / AC / TRAN 的 v0.1 数据路径。
- 准备 10 个基础示例电路。
- 建立基础 integration tests。
- 生成第一版 HTML report。
- 形成 v0.1 release checklist。

完成定义：

- v0.1 验收命令全部通过。
- 文档记录依赖安装、ngspice 要求和常见错误。
- 产生可打包的 CLI artifact。

## 6. 仓库治理

### 6.1 Git 策略

项目应作为 Git 工程管理。建议初始化后采用：

- `main`：始终保持可构建。
- `feat/<topic>`：功能开发分支。
- `fix/<topic>`：缺陷修复分支。
- `docs/<topic>`：文档变更分支。

提交粒度：

- 每个提交只解决一个明确问题。
- 代码提交必须能通过 `cargo fmt` 和相关测试。
- 文档计划更新可以独立提交。

### 6.2 忽略规则

必须忽略：

- Rust 构建目录 `target/`。
- 本地仿真输出 `runs/`。
- 生成报告 `reports/`。
- benchmark 输出 `bench-results/`。
- 临时波形缓存 `*.rawcache`、`*.mmap`。
- 编辑器和系统临时文件。

### 6.3 CI 门禁

每次提交：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

每日或 release 前：

```bash
osl bench benchmarks/basic
osl bench benchmarks/waveform
osl verify examples/buck_converter/validation.yaml
```

## 7. 风险清单

### 7.1 ngspice raw 格式兼容

风险：raw 文件格式和平台行为可能不一致。

控制措施：

- 建立 binary raw / ascii raw 双路径测试。
- 保存多平台 golden raw。
- parser 错误必须包含文件偏移或行号。

### 7.2 波形性能目标过高

风险：如果数据层设计错误，后期 GUI 很难补救。

控制措施：

- v0.1 就建立 parser throughput benchmark。
- v0.3 前不把 GUI 作为主线。
- 所有视窗查询 API 都用大数据测试。

### 7.3 LTspice / KiCad 兼容范围失控

风险：导入功能容易变成无底洞。

控制措施：

- v0.6 只承诺常见模拟电路和明确诊断。
- 每个 unsupported feature 都必须生成报告，而不是静默失败。
- 先导入 netlist 工作流，再考虑完整 schematic 体验。

### 7.4 模型诊断准确性

风险：错误建议可能比没有建议更糟。

控制措施：

- 将诊断分为 error、warning、suggestion 三类。
- suggestion 必须带置信度或依据。
- 建立厂商模型测试集。

### 7.5 过早做 GUI

风险：GUI 会消耗大量时间，但没有数据层和验证闭环就无法形成核心竞争力。

控制措施：

- v0.1-v0.3 以 CLI、数据和验证为主。
- GUI 只消费稳定 API。
- 渲染目标必须由 benchmark 约束。

## 8. 下一步开发任务

优先执行以下任务：

1. 修复或初始化 Git 仓库。
2. 创建 Rust workspace。
3. 创建 v0.1 三个核心 crate：`osl-cli`、`osl-core`、`osl-sim`。
4. 实现 `osl --version`。
5. 实现 ngspice 探测和最小 `run` 命令。
6. 添加 RC 示例电路。
7. 输出第一份 `run.json`。

完成这些任务后，项目才进入真实实现阶段。

