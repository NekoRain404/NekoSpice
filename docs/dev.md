# OpenSpiceLab-RS 开发文档 v0.1

## 1. 项目定位

OpenSpiceLab-RS 是一款基于 Rust 的开源 SPICE 仿真验证平台，目标不是简单复制 LTspice，而是在以下方向形成优势：

* 高性能波形查看器
* 自动化仿真验证
* 参数扫描、Corner、Monte Carlo
* 模型兼容性诊断
* LTspice / KiCad 工作流导入
* CI / 批量验证 / 报告系统
* 可扩展多后端仿真架构

项目核心口号：

> LTspice 适合手动仿真，OpenSpiceLab-RS 适合工程级自动验证。

---

## 2. 核心设计原则

### 2.1 Rust-first

整个应用层、数据层、调度层、GUI、波形渲染、模型管理、报告系统全部使用 Rust 编写。

允许的外部组件：

* ngspice CLI
* ngspice shared library，后期可选
* Xyce CLI，后期可选
* Python 插件，后期可选

不在第一阶段做：

* C++/Qt GUI
* Electron
* 自研完整 SPICE 求解器
* GPU SPICE 求解器
* 完整原理图编辑器

---

### 2.2 性能优先

从第一天开始建立性能基准，不等项目后期再优化。

核心要求：

* 所有关键路径必须可 benchmark
* 波形查看器必须支持百万点以上数据
* sweep / Monte Carlo 必须天然并行
* 仿真结果必须可流式读取
* GUI 不允许因为仿真任务阻塞
* 大文件必须支持 mmap / lazy loading
* 波形显示必须支持 LOD 和降采样

---

### 2.3 后端解耦

仿真后端通过统一 trait 抽象。

第一阶段使用 ngspice CLI，后期支持：

* ngspice shared library
* Xyce CLI
* 未来自研快速求解器
* 未来远程仿真 worker

---

### 2.4 数据管线优先于界面

第一阶段不要先做漂亮 GUI。

正确顺序：

1. CLI 能跑仿真
2. 能解析波形
3. 能存储结果
4. 能自动测量
5. 能输出报告
6. 能 benchmark
7. 再做 GUI 和 GPU 波形查看器

---

## 3. 总体技术栈

### 3.1 主语言

```text
Rust stable
```

### 3.2 GUI 与渲染

推荐方案：

```text
窗口系统：winit
GUI Overlay：egui
GPU 渲染：wgpu
波形渲染：自研 wgpu waveform renderer
```

说明：

* 不建议完全依赖现成 plot 库。
* 波形查看器必须自研。
* egui 用于菜单、面板、参数配置、日志窗口。
* wgpu 用于高性能波形、热力图、Monte Carlo 云图、频谱图。

可选简化方案：

```text
eframe + egui + wgpu
```

MVP 可以先用 eframe，但如果目标是极致性能，建议较早切到：

```text
winit + egui-wgpu + 自研渲染循环
```

---

### 3.3 仿真后端

第一阶段：

```text
ngspice CLI
```

第二阶段：

```text
ngspice shared library
```

第三阶段：

```text
Xyce CLI
```

长期：

```text
自研局部快速分析器
远程仿真 worker
GPU 后处理
```

---

### 3.4 数据存储

```text
SQLite：项目元数据、仿真任务、模型索引
Arrow：内存列式波形数据
Parquet：长期波形归档
mmap binary cache：大波形快速浏览
JSON：机器可读报告
HTML：人类可读报告
JUnit XML：CI 测试结果
```

原则：

* 不把大波形塞进 SQLite。
* SQLite 只存元数据。
* 大波形使用二进制列式存储。
* GUI 只读取当前视窗需要的数据。

---

### 3.5 并行与异步

```text
rayon：CPU 密集型并行任务
tokio：异步进程、任务通信、文件 IO 调度
crossbeam：高性能通道
parking_lot：轻量锁
```

用途：

* sweep 并行
* Monte Carlo 并行
* 多后端任务调度
* 波形降采样
* 批量测量
* 后台报告生成

---

### 3.6 解析器

建议：

```text
winnow 或 nom：SPICE / LTspice netlist 解析
logos：简单 token lexer
自研 line parser：性能关键路径
```

不要一开始过度设计完整语法树。第一版重点解析：

* `.include`
* `.lib`
* `.param`
* `.model`
* `.subckt`
* `.ends`
* `.tran`
* `.ac`
* `.dc`
* `.op`
* `.meas`
* 基础元件 R/C/L/V/I/D/Q/M/X/B/E/G/F/H

---

### 3.7 报告与配置

```text
serde
serde_yaml
serde_json
toml
tera 或 askama
```

配置格式优先使用 YAML：

```yaml
project: buck_converter

backend: ngspice

runs:
  - name: load_step
    analysis:
      type: tran
      stop: 5ms
      maxstep: 50ns

    sweep:
      vin: [9, 12, 15]
      load: [0.5, 1.0, 2.0]
      temp: [-40, 25, 85]

    checks:
      - name: output_voltage
        expr: avg(V(out), 3ms, 5ms)
        pass: 4.9 <= value <= 5.1

      - name: ripple
        expr: pp(V(out), 3ms, 5ms)
        pass: value < 50mV
```

---

## 4. 推荐 Rust crate

### 4.1 CLI

```toml
clap = "4"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 4.2 GUI / GPU

```toml
winit = "*"
wgpu = "*"
egui = "*"
egui-wgpu = "*"
egui-winit = "*"
```

### 4.3 数据

```toml
rusqlite = "*"
arrow = "*"
parquet = "*"
memmap2 = "*"
```

### 4.4 并行

```toml
rayon = "*"
tokio = { version = "*", features = ["full"] }
crossbeam-channel = "*"
parking_lot = "*"
```

### 4.5 数值与信号处理

```toml
ndarray = "*"
nalgebra = "*"
rustfft = "*"
statrs = "*"
```

### 4.6 解析

```toml
winnow = "*"
logos = "*"
regex = "*"
```

### 4.7 报告

```toml
serde = { version = "*", features = ["derive"] }
serde_json = "*"
serde_yaml = "*"
tera = "*"
```

---

## 5. 系统架构

```text
OpenSpiceLab-RS
│
├── osl-cli
│   ├── run
│   ├── verify
│   ├── bench
│   ├── model-check
│   ├── import-ltspice
│   └── report
│
├── osl-app
│   ├── winit shell
│   ├── egui panels
│   ├── wgpu waveform viewer
│   ├── report viewer
│   └── model browser
│
├── osl-core
│   ├── CircuitIR
│   ├── Project
│   ├── SimulationConfig
│   ├── Measurement
│   └── Diagnostics
│
├── osl-sim
│   ├── SimulatorBackend trait
│   ├── NgspiceCliBackend
│   ├── NgspiceSharedBackend
│   └── XyceCliBackend
│
├── osl-netlist
│   ├── SPICE parser
│   ├── LTspice parser
│   ├── KiCad netlist importer
│   └── netlist normalizer
│
├── osl-model
│   ├── model index
│   ├── subckt pin parser
│   ├── dialect detector
│   ├── pin mapping checker
│   └── compatibility report
│
├── osl-waveform
│   ├── raw parser
│   ├── waveform table
│   ├── mmap cache
│   ├── LOD builder
│   ├── min/max downsampling
│   └── viewport query engine
│
├── osl-render
│   ├── waveform renderer
│   ├── grid renderer
│   ├── marker renderer
│   ├── density renderer
│   └── FFT renderer
│
├── osl-experiment
│   ├── sweep
│   ├── corner
│   ├── Monte Carlo
│   ├── job scheduler
│   └── result aggregator
│
├── osl-measure
│   ├── expression parser
│   ├── measurement functions
│   ├── pass/fail evaluator
│   └── statistics
│
└── osl-report
    ├── HTML
    ├── JSON
    ├── Markdown
    └── JUnit XML
```

---

## 6. Workspace 结构

```text
openspicelab-rs/
  Cargo.toml

  crates/
    osl-core/
    osl-cli/
    osl-app/
    osl-sim/
    osl-netlist/
    osl-model/
    osl-waveform/
    osl-render/
    osl-experiment/
    osl-measure/
    osl-report/
    osl-bench/

  examples/
    rc_filter/
    rlc_resonance/
    diode_rectifier/
    opamp_amplifier/
    buck_converter/
    ltspice_import/

  benchmarks/
    basic/
    power/
    convergence/
    waveform/
    parser/

  testdata/
    netlists/
    models/
    raw/
    golden/

  docs/
    architecture.md
    performance.md
    plugin.md
    model_compatibility.md
```

---

## 7. 核心数据结构

### 7.1 仿真后端 trait

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

---

### 7.2 仿真任务

```rust
pub struct SimulationJob {
    pub id: RunId,
    pub project_root: PathBuf,
    pub netlist_path: PathBuf,
    pub working_dir: PathBuf,
    pub analysis: AnalysisConfig,
    pub parameters: Vec<ParameterOverride>,
    pub options: SimulationOptions,
}
```

---

### 7.3 波形数据

```rust
pub struct Waveform {
    pub run_id: RunId,
    pub x: WaveColumn,
    pub signals: Vec<Signal>,
    pub metadata: WaveformMetadata,
}

pub struct Signal {
    pub name: String,
    pub unit: Unit,
    pub data: WaveColumn,
}
```

---

### 7.4 视窗查询

```rust
pub struct ViewportQuery {
    pub signal_id: SignalId,
    pub x_min: f64,
    pub x_max: f64,
    pub pixel_width: u32,
    pub lod_policy: LodPolicy,
}
```

---

### 7.5 测量结果

```rust
pub struct MeasurementResult {
    pub name: String,
    pub value: Quantity,
    pub passed: bool,
    pub message: Option<String>,
}
```

---

## 8. 高性能波形设计

### 8.1 目标

第一版波形查看器必须达到：

```text
1,000,000 点：流畅缩放
10,000,000 点：可交互浏览
100 条 sweep 曲线：可叠加查看
500 次 Monte Carlo：可生成统计图
```

---

### 8.2 数据策略

波形数据分三层：

```text
原始层：
  完整仿真数据，存储在 Arrow / Parquet / raw cache

缓存层：
  min/max LOD cache
  tiled waveform cache
  FFT cache
  measurement cache

渲染层：
  当前视窗 GPU vertex buffer
```

---

### 8.3 降采样策略

不要简单丢点。

必须支持：

```text
- min/max envelope
- LTTB
- fixed bucket sampling
- peak preserving sampling
```

宽视野显示时使用 min/max envelope，防止丢失尖峰。

局部放大时显示原始数据。

---

### 8.4 GPU 渲染策略

使用 wgpu。

渲染对象：

```text
- grid
- axis
- waveform line
- min/max envelope
- markers
- cursors
- selection rectangle
- density map
```

渲染原则：

* 坐标转换在 shader 中完成
* CPU 只传当前视窗数据
* 大曲线使用 vertex buffer
* 多曲线复用 pipeline
* 文本标注仍由 egui 绘制
* 图形层与 UI 层解耦

---

## 9. 仿真执行策略

### 9.1 第一阶段：ngspice CLI

使用进程隔离：

```text
优点：
- 稳定
- 不容易拖垮主程序
- 崩溃隔离
- 跨平台简单
- 调试方便

缺点：
- 实时交互差
- callback 能力弱
- 启动开销更高
```

第一阶段优先稳定和可 benchmark。

---

### 9.2 第二阶段：ngspice shared library

用途：

```text
- 实时波形
- 运行中取消
- 运行中调参
- 更低启动开销
- GUI 内嵌仿真
```

要求：

* 独立 crate：`osl-ngspice-sys`
* unsafe 代码隔离
* FFI 边界清晰
* 崩溃隔离方案明确
* Windows / macOS / Linux 分别测试

---

### 9.3 第三阶段：Xyce CLI

用途：

```text
- 大规模电路
- 批量仿真
- 与 ngspice 结果对比
- 作为后端备选
```

---

## 10. 自动验证系统

### 10.1 目标

让电路仿真像软件测试一样可重复执行。

命令：

```bash
osl verify project.osl.yaml
```

输出：

```text
PASS startup_time
PASS output_voltage
FAIL ripple

Worst case:
  vin = 15V
  temp = 85C
  load = 2A
  ripple = 83mV
```

---

### 10.2 支持能力

第一阶段：

```text
- sweep
- measurement
- pass/fail
- HTML report
- JSON report
```

第二阶段：

```text
- corner
- Monte Carlo
- worst-case search
- JUnit XML
- GitHub Actions
```

第三阶段：

```text
- sensitivity
- optimization
- multi-backend comparison
```

---

## 11. 测量引擎

第一阶段内置函数：

```text
avg()
rms()
min()
max()
pp()
abs_max()
rise_time()
fall_time()
settling_time()
overshoot()
undershoot()
frequency()
duty_cycle()
```

第二阶段：

```text
fft()
thd()
integrated_noise()
bandwidth()
phase_margin()
gain_margin()
efficiency()
ripple()
```

表达式示例：

```text
avg(V(out), 3ms, 5ms)
pp(V(out), 3ms, 5ms)
settling_time(V(out), target=5V, tolerance=2%)
```

---

## 12. 模型兼容层

### 12.1 目标

解决 SPICE 用户最痛的问题：模型导入失败、pin mapping 错误、方言不兼容。

### 12.2 功能

```text
- 识别 .model
- 识别 .subckt
- 提取 subckt pin list
- 判断模型方言：ngspice / LTspice / PSpice / HSpice-like
- 检查 unsupported directive
- 检查 unsupported function
- 检查 symbol pin 与 subckt pin 是否匹配
- 输出兼容性评分
- 给出修复建议
```

### 12.3 示例输出

```text
Model: LM358
Type: opamp
Subckt: LM358
Pins: IN+ IN- VCC VEE OUT
Status: pin order mismatch

Suggested mapping:
  IN+ -> symbol pin 3
  IN- -> symbol pin 2
  VCC -> symbol pin 8
  VEE -> symbol pin 4
  OUT -> symbol pin 1
```

---

## 13. LTspice 导入

### 13.1 第一阶段支持

```text
- .asc 基础解析
- .asy pin / attribute 解析
- .lib / .subckt / .model
- 常见 directive
- 常见元件
```

### 13.2 不追求第一版 100% 兼容

第一版目标不是完美导入，而是：

```text
- 能导入常见电路
- 能指出不兼容位置
- 能生成修复建议
- 能生成 ngspice 可运行 netlist
```

### 13.3 导入报告

```text
Import result:
  Components: 42
  Symbols: 18
  Directives: 7
  Compatibility: 82%

Unsupported:
  - behavioral source syntax
  - special symbol attribute
  - unsupported directive

Suggestions:
  - rewrite B-source expression
  - remap U1 pin order
```

---

## 14. 性能基准系统

### 14.1 必须从第一天建立 benchmark

命令：

```bash
osl bench benchmarks/
```

记录：

```text
- 仿真时间
- raw 解析时间
- 波形加载时间
- 降采样时间
- 渲染帧率
- 内存占用
- Monte Carlo 总时间
- sweep 并行效率
```

---

### 14.2 benchmark 分类

```text
basic:
  - RC low-pass
  - RLC resonance
  - diode rectifier

analog:
  - opamp amplifier
  - active filter
  - oscillator

power:
  - buck converter
  - boost converter
  - LDO
  - load step

convergence:
  - ideal switch
  - high-Q LC
  - floating node
  - startup circuit

waveform:
  - 1M points
  - 10M points
  - 100 curves
  - 500 Monte Carlo traces
```

---

### 14.3 性能目标

v0.1：

```text
RC / RLC / diode 示例可稳定运行
raw 解析速度 > 100 MB/s
1M 点波形可加载
基础 HTML 报告生成 < 1s
```

v0.3：

```text
1M 点波形 60 FPS 缩放
10M 点波形可交互浏览
27 组 sweep 并行执行
report 可显示失败用例和波形摘要
```

v0.6：

```text
500 次 Monte Carlo 可并行执行
100 条曲线可叠加查看
模型兼容检查可在大型模型库上运行
CI 输出 JUnit XML
```

---

## 15. 开发路线图

### 阶段 0：性能基准与项目骨架，2 周

目标：

```text
建立 Rust workspace
建立 CLI
建立 benchmark 框架
建立日志系统
建立错误系统
准备 20 个基础测试电路
```

产物：

```text
osl --version
osl bench
osl run examples/rc_filter/rc.cir
```

验收：

```text
项目可以跨平台构建
benchmark 可以运行
ngspice CLI 可以被调用
日志和错误输出清晰
```

---

### 阶段 1：仿真最小闭环，1 个月

目标：

```text
完成 ngspice CLI runner
完成 raw parser 初版
完成 waveform 数据结构
完成 CSV / JSON 输出
完成 OP / DC / AC / TRAN 支持
```

命令：

```bash
osl run examples/rc_filter/rc.cir --output runs/rc_001
```

输出：

```text
run.json
waveform.csv
ngspice.log
```

验收：

```text
能稳定运行 10 个基础电路
能读取 V(out)
能导出 CSV
能记录仿真时间
```

---

### 阶段 2：验证 DSL 与测量引擎，1–2 个月

目标：

```text
完成 YAML 配置
完成 measurement engine
完成 pass/fail
完成 sweep
完成 HTML / JSON 报告
```

命令：

```bash
osl verify examples/buck_converter/validation.yaml
```

验收：

```text
可以自动跑多组参数
可以生成 PASS / FAIL
可以输出 HTML report
可以输出 JSON result
```

---

### 阶段 3：高性能波形数据层，2–3 个月

目标：

```text
完成 Arrow / Parquet 存储
完成 mmap cache
完成 LOD cache
完成 min/max downsampling
完成 viewport query engine
```

验收：

```text
1M 点波形秒级加载
10M 点波形可查询
降采样不丢尖峰
查询 API 可供 GUI 使用
```

---

### 阶段 4：GPU 波形查看器，3–4 个月

目标：

```text
完成 winit + egui + wgpu app shell
完成 waveform renderer
完成缩放 / 平移 / 游标
完成多曲线叠加
完成 sweep 分组显示
```

验收：

```text
1M 点波形缩放流畅
10M 点波形可交互浏览
27 组 sweep 可叠图
失败检查可跳转到对应波形
```

---

### 阶段 5：模型兼容与诊断，4–6 个月

目标：

```text
完成 .subckt pin 解析
完成模型库索引
完成方言检测
完成 pin mapping 检查
完成常见错误诊断
```

验收：

```text
导入厂家模型后能显示：
- subckt 名称
- pin list
- 兼容性评分
- 不兼容语法
- 修复建议
```

---

### 阶段 6：LTspice / KiCad 工作流导入，6–8 个月

目标：

```text
完成 LTspice .asc 初版导入
完成 LTspice .asy 初版导入
完成 KiCad netlist 导入
完成导入兼容性报告
```

验收：

```text
常见 LTspice 模拟电路可导入
KiCad 导出的 SPICE netlist 可运行
不能运行时必须明确指出原因
```

---

### 阶段 7：高级验证，8–12 个月

目标：

```text
Monte Carlo
corner analysis
tolerance analysis
worst-case search
JUnit XML
GitHub Actions 示例
多后端任务调度
```

验收：

```text
用户可以把电路仿真作为 CI 测试运行
可以输出良率
可以定位 worst case
可以生成工程报告
```

---

### 阶段 8：ngspice shared library，12 个月后

目标：

```text
实时波形
仿真取消
运行中调参
低延迟交互
```

要求：

```text
FFI 独立封装
unsafe 代码隔离
跨平台动态库加载
崩溃隔离策略
```

---

## 16. 质量保障

### 16.1 测试类型

```text
unit test:
  - parser
  - measurement
  - unit conversion
  - expression evaluator

integration test:
  - ngspice runner
  - raw parser
  - report generation

golden test:
  - 波形结果与 golden 数据比较
  - 测量结果误差比较

performance test:
  - raw parser throughput
  - waveform query latency
  - render FPS
  - sweep parallel speedup
```

---

### 16.2 CI

每次提交运行：

```text
cargo fmt
cargo clippy
cargo test
basic simulation tests
parser tests
report generation tests
```

每日运行：

```text
full benchmark
large waveform test
Monte Carlo test
LTspice import test
model compatibility test
```

---

## 17. 第一版 MVP 范围

### 必须做

```text
- Rust CLI
- ngspice CLI runner
- raw parser
- YAML verification
- measurement engine
- sweep
- HTML / JSON report
- basic waveform storage
- benchmark framework
```

### 可以延后

```text
- 完整 GUI
- 完整 LTspice 导入
- 完整 KiCad 项目解析
- ngspice shared library
- Python 插件
- GPU compute
- 自研 SPICE solver
```

---

## 18. v0.1 具体任务清单

### 第 1 周

```text
- 创建 Cargo workspace
- 创建 osl-cli
- 创建 osl-core
- 创建 osl-sim
- 建立 tracing 日志
- 实现 ngspice 进程调用
- 准备 RC 示例电路
```

### 第 2 周

```text
- 解析 ngspice log
- 记录仿真耗时
- 保存 run metadata
- 输出 run.json
- 建立 benchmark 命令
```

### 第 3 周

```text
- 实现 raw parser 初版
- 支持 V(node) 波形读取
- 支持 CSV 导出
- 支持 transient 示例
```

### 第 4 周

```text
- 支持 OP / DC / AC / TRAN
- 建立 10 个示例电路
- 建立基础 integration test
- 生成第一份 HTML report
```

v0.1 验收命令：

```bash
osl run examples/rc_filter/rc.cir
osl run examples/diode_rectifier/rectifier.cir
osl bench benchmarks/basic
```

---

## 19. v0.2 具体任务清单

```text
- YAML config parser
- SimulationJob builder
- sweep expansion
- measurement expression parser
- avg / min / max / pp / rms
- pass/fail evaluator
- HTML report template
- JSON report
```

v0.2 验收命令：

```bash
osl verify examples/buck_converter/validation.yaml
```

预期输出：

```text
Runs: 27
Passed: 25
Failed: 2

FAIL ripple
  expected: < 50mV
  actual: 83mV
```

---

## 20. 最终技术路线总结

OpenSpiceLab-RS 的主路线：

```text
Rust CLI
→ ngspice runner
→ raw parser
→ measurement engine
→ verification DSL
→ high-performance waveform storage
→ wgpu waveform viewer
→ model compatibility
→ LTspice / KiCad import
→ Monte Carlo / corner
→ CI integration
→ ngspice shared library
→ advanced GPU visualization
```

第一年核心目标：

> 不在 SPICE 求解速度上盲目硬刚 LTspice，而是在自动化验证、批量仿真、高性能波形、模型诊断、报告和 CI 上超过 LTspice。

性能策略：

```text
CPU 多核负责：
- sweep
- Monte Carlo
- measurement
- parser
- downsampling

GPU 负责：
- waveform rendering
- density map
- Monte Carlo visualization
- FFT visualization
- schematic canvas

ngspice / Xyce 负责：
- SPICE 求解
```

项目成功的关键不是“Rust 重写一切”，而是：

> Rust 负责构建一个高性能、可维护、可扩展的仿真验证平台；SPICE 求解器先复用成熟后端，把主要创新放在工程验证和用户体验上。
