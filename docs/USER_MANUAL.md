# NekoSpice 用户手册

NekoSpice 是一款基于 Rust 原生构建的 SPICE 仿真平台，兼容 KiCad 原理图格式，支持 ngspice 和 Xyce 双后端求解器。

## 快速开始

### 启动应用

```bash
cargo run -p osl-app
```

应用启动后默认加载 `examples/cm5_minima/CM5.kicad_sch` 示例原理图。

### 基本工作流

1. **打开原理图** — `Ctrl+O` 或菜单 File > Open
2. **编辑仿真参数** — 切换到 Simulation 工作区
3. **运行仿真** — `F5` 或点击 Run 按钮
4. **查看结果** — 切换到 Waveforms 工作区查看波形

---

## 工作区导航

NekoSpice 提供 9 个工作区，通过左侧导航栏切换：

| 快捷键 | 工作区 | 说明 |
|--------|--------|------|
| `Ctrl+1` | Home | 项目仪表板和快捷入口 |
| `Ctrl+2` | Schematic | 原理图编辑器 |
| `Ctrl+3` | Library | 符号库浏览和放置 |
| `Ctrl+4` | Simulation | 仿真参数配置和运行 |
| `Ctrl+5` | Optimization | 参数优化和扫描 |
| `Ctrl+6` | Review | 设计审查和 ERC |
| `Ctrl+7` | Waveforms | 波形分析和测量 |
| `Ctrl+8` | Reports | 仿真报告生成 |
| `Ctrl+9` | Settings | 主题和语言设置 |

---

## 原理图编辑器

### 工具栏

| 快捷键 | 工具 | 说明 |
|--------|------|------|
| `V` | Select | 选择模式 |
| `W` | Wire | 绘制导线 |
| `L` | Label | 添加网络标签 |
| `B` | Bus | 绘制总线 |
| `S` | Sheet | 添加子图纸 |
| `J` | Junction | 添加节点 |
| `Q` | No Connect | 添加无连接标记 |
| `R` | Rotate | 旋转选中项 90° |
| `F` | Fit | 适应视图 |
| `Del` | Delete | 删除选中项 |

### 鼠标操作

- **左键点击**: 选择元素
- **右键拖拽**: 平移视图
- **滚轮**: 缩放视图
- **右键点击**: 打开上下文菜单

### 编辑操作

| 快捷键 | 操作 |
|--------|------|
| `Ctrl+Z` | 撤销 |
| `Ctrl+Shift+Z` / `Ctrl+Y` | 重做 |
| `Ctrl+S` | 保存 |
| `Ctrl+Shift+S` | 另存为 |
| `Ctrl+X` | 剪切 |
| `Ctrl+C` | 复制 |
| `Ctrl+V` | 粘贴 |
| 方向键 | 微调位置 (2.54mm) |

### 底部停靠面板

原理图工作区底部提供 7 个标签页：

- **Waveforms**: 仿真波形信号列表和堆叠预览
- **FFT**: 频域分析，显示幅度频谱
- **Bode**: 波特图，幅度和相位频率响应
- **Console**: 控制台输出，状态消息和仿真日志
- **Netlist**: SPICE 网表预览
- **ERC**: 电气规则检查结果
- **Inspector**: 选中元素属性检查器

---

## 仿真工作区

### 分析类型

| 类型 | 说明 | 典型应用 |
|------|------|----------|
| `.tran` | 瞬态分析 | 时域波形、开关响应 |
| `.ac` | 交流分析 | 频率响应、波特图 |
| `.dc` | 直流扫描 | 传输特性、IV 曲线 |
| `.op` | 工作点 | DC 偏置条件 |
| `.noise` | 噪声分析 | 噪声频谱密度 |
| `.disto` | 失真分析 | 谐波失真 |
| `.sens` | 灵敏度分析 | 参数灵敏度 |

### 求解器预设

NekoSpice 内置 10 种求解器预设，适用于不同电路类型：

| 预设 | 说明 |
|------|------|
| Default | 标准 SPICE 默认值 |
| Fast | 放松容差，快速迭代 |
| Accurate | 严格容差，Gear 积分 |
| High Frequency | 高频电路优化 |
| Convergence Aid | 激进收敛辅助 |
| Power Electronics | 开关变换器、电机驱动 |
| Low Power | 超低功耗 IoT、电池电路 |
| Precision | 精密仪器、ADC/DAC |
| Mixed Signal | 混合信号、PLL |
| RF | 射频电路、混频器 |

### 步进扫描

支持参数扫描和温度扫描：

- **参数扫描**: `.step param R1 lin 1k 100k 10k`
- **温度扫描**: `.step TEMP lin -40 125 10`

### 仿真流程

```
配置分析类型和参数
    ↓
选择求解器预设（可选）
    ↓
配置步进扫描（可选）
    ↓
添加厂商模型（可选）
    ↓
运行仿真 (F5)
    ↓
查看波形和测量结果
```

---

## 波形分析工作区

### 分析标签页

- **Time Domain**: 电压/电流 vs 时间
- **Bode**: 幅度和相位 vs 频率
- **FFT**: 时域信号的频谱分析
- **Noise**: 噪声频谱密度
- **Eye**: 眼图（信号完整性）

### 交互操作

- **鼠标滚轮**: 缩放波形
- **鼠标拖拽**: 平移波形
- **Cursors**: 启用光标叠加层查看精确数值
- **Overlay**: 多信号叠加模式
- **AutoScale**: 自动缩放适配所有信号

### 测量结果

波形面板自动计算以下测量值：

- First / Last: 首末值
- Min / Max: 最小最大值
- Avg: 平均值
- RMS: 均方根值
- P-P: 峰峰值

---

## 厂商模型库

### 支持的厂商

- **Texas Instruments (TI)**: `.lib` 格式 SPICE 模型
- **Analog Devices (ADI)**: `.lib`, `.mod` 格式 SPICE 模型
- **通用 SPICE 模型**: `.sub`, `.sp`, `.cir` 格式

### 导入流程

1. 切换到 Library 工作区
2. 点击 "Browse..." 选择模型目录
3. 搜索或浏览子电路和模型
4. 点击 "+" 将模型添加到仿真配置

---

## 导出功能

| 格式 | 说明 |
|------|------|
| Netlist (.cir) | ngspice/Xyce 兼容的 SPICE 网表 |
| Waveform CSV | 所有信号的逗号分隔值文件 |
| Simulation Log | ngspice.log 或 xyce.log 仿真日志 |

---

## 设置

### 主题

支持 Dark、Light、Midnight 三种主题模式，点击顶栏主题按钮切换。

### 语言

支持 English 和 简体中文，点击顶栏语言按钮切换。

### 求解器路径

在 Settings 工作区配置 ngspice 和 Xyce 可执行文件路径。

---

## 键盘快捷键汇总

### 全局快捷键

| 快捷键 | 操作 |
|--------|------|
| `F5` | 运行仿真 |
| `Ctrl+S` | 保存 |
| `Ctrl+Shift+S` | 另存为 |
| `Ctrl+O` | 打开文件 |
| `Ctrl+N` | 新建 |
| `Ctrl+Z` | 撤销 |
| `Ctrl+Shift+Z` | 重做 |
| `Ctrl+Y` | 重做 |
| `Ctrl+X` | 剪切 |
| `Ctrl+C` | 复制 |
| `Ctrl+V` | 粘贴 |
| `Ctrl+1~9` | 切换工作区 |
| `Ctrl+Shift+E` | 导出网表 |
| `?` | 显示快捷键帮助 |

### 原理图工具

| 快捷键 | 工具 |
|--------|------|
| `V` | 选择 |
| `W` | 导线 |
| `L` | 标签 |
| `B` | 总线 |
| `S` | 子图纸 |
| `J` | 节点 |
| `Q` | 无连接 |
| `R` | 旋转 |
| `F` | 适应视图 |
| `Del` | 删除 |
| `Esc` | 取消/返回选择 |
| `←↑↓→` | 微调位置 |

---

## 文件结构

```
~/.config/nekospice/
├── settings.json              # UI 偏好 + 仿真选项 + 区块开关
├── simulation_history.json    # 最近 20 次仿真记录
└── presets/                   # 用户自定义预设
    └── *.preset
```


## 新增功能

### 新建原理图
- 首页点击 **New Schematic** 可创建空白 A4 原理图
- 自动切换到原理图工作区，可开始放置元件和连线

### 波形分析工作区
- **Time Domain**: 时域波形，支持鼠标拖拽平移、滚轮缩放、光标叠加
- **FFT**: 使用 Hanning 窗的快速傅里叶变换，显示 dB 幅度频谱
- **Bode**: 双面板显示幅度 (dB) 和相位 (deg) vs 频率
- **Noise**: 噪声谱密度分析
- **Eye**: 眼图模式，适用于周期性数字信号分析
- 支持 **Overlay 模式** 同时显示多条信号轨迹
- 支持 **Export CSV** 和 **Export Report** (HTML)

### 底部停靠面板
- **Waveforms**: 信号列表和堆叠预览
- **FFT**: 频域分析，实时计算显示
- **Bode**: 波特图，幅度和相位频率响应
- **Console**: 状态消息、仿真日志输出
- **Netlist**: SPICE 网表预览
- **ERC**: 电气规则检查结果
- **Inspector**: 选中项属性查看

### 设计审查
- 原理图风险评分和优先级排序
- ERC 检查结果自动汇总
- 推荐操作步骤

### 参数优化
- Monte Carlo 分析（参数分布和良率）
- 参数扫描设置
- 分布预览和统计摘要

---

## 构建和开发

```bash
# 构建 GUI
cargo build -p osl-app

# 运行所有测试
cargo test --workspace

# 启动 GUI
cargo run -p osl-app
```
