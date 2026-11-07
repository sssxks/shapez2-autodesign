# 1×2 联合布局与布线实验

当前可交付成品仍是上级目录的 **1×3** 蓝图。这里保存 1×2 的搜索模型、起始摆法及来自原生成品的正例。

新的[跨单元建筑级模型](WEAVE.md)已实现并通过原生两输出测试：允许不同切割机的半片与四分之一片合流，共享旋转器，联合选择单台建筑和连接。下面的 `quarter-search` 保留整块切割单元，用于原模型复现与对照。

## 模型

平台有效地面为 `34 × 14 × 3 = 1,428` 格，外部接口另计。模型保留 48 个四象限切割单元，联合选择位置、镜像、四种出料结构、分流方向及运输网络。

独立饱和测试得到：完整切割器每个输出在 120 秒内排出 60 个半片，即每台处理 `0.5 个输入/秒`；旋转器为 `1 个/秒`。12 条满带供应 24 个完整形状/秒，48 条满带需要 96 个四分之一形状/秒，每秒必须净增加 72 个物件。一次完整切割最多净增加一个物件，因此至少需要 `72 / 0.5 = 144` 台完整切割器，占 288 格。这是切割阶段的容量下界。

独立容量实验：`cargo run -p quarter-cut -- --capacity`，原始数据位于运行目录的 `cutter-capacity.json` 和 `rotator-capacity.json`。

- 完整形状以 `0.5 个/秒` 为流量单位：12 条输入各供应 4 单位。
- 每个单元消耗 1 单位。分流单元接收 2–4 单位，剩余流量送往后续单元；末端单元接收 1 单位。
- 每个单元产生一条 `2 个/秒` 的四分之一形状输出，48 个源共同连接 48 条外部输出带。
- 建筑、普通带、升降机中间层和接口共用逐格容量约束。
- 跳跃长度为 2–5 格，接收器只能直出。两个跳跃的发射器若能看到对方的接收器，则不能同时选择。
- 输入和输出可直接连接外部接口；内部接收器可直接接机器输入。
- 一条供料连接的首尾可以是同一个带格。

建模、整数约束验证和路线导出全部使用 Rust。`shapez-solver` 通过已有 `cpsat-rs` 和 `russcip` 绑定调用原生 CP-SAT 与 SCIP/SoPlex；不需要 Python。首次运行前 dot-source `environment.ps1`，它将固定版本的官方 OR-Tools Windows SDK 安装在临时实验目录，并只设置当前 PowerShell 的环境变量。SCIP 由其 Rust 绑定安装。

**模型范围是限定单元结构、起始摆法、移动半径及旋转选项。** `INFEASIBLE` 对应该范围，`UNKNOWN` 表示到达搜索时限。模型没有覆盖任意原生建筑组合，也没有给出全局不可行性结论。

## 运行

从仓库根目录执行：

```powershell
. tasks/quarter-cut/search/environment.ps1
$env:QUARTER_PORTS = 'original'
$candidate = "$env:TEMP/shapez-lab/quarter-1x2-search"
cargo run -p quarter-cut --features search --bin quarter-search -- `
  tasks/quarter-cut/search/original-ports-seed.json $candidate `
  --radius 1 --seconds 300 --workers 8
```

`original` 对应上方输入、左/右/下方两口输出。另一种布局使用 `$env:QUARTER_PORTS = 'west'` 和 `west-input-seed.json`，对应左侧输入、上下各两口输出；搜索命令同时加 `--ports west`。

`outward-rows-seed.json` 同样使用 `west` 接口，每层单元分为朝向外侧的两排，中央留作供料。

| 参数 | 意义 |
| --- | --- |
| `--radius N` | 单元中心横纵移动半径 |
| `--half-turn` | 允许单元在原包围盒内掉转 180° |
| `--backend scip` | 使用 SCIP/SoPlex 整数规划后端 |
| `--seconds N` | 求解时间上限，不含模型构造 |
| `--workers N` | 工作线程数；SCIP 会创建多个并发求解器 |

每次写出 `<candidate>.status.json`。可行解还写出逐弧整数流量 `<candidate>.flows.json`。只有可行解才写出摆法 `<candidate>.json` 和路线 `<candidate>.routes.json`。生成蓝图并运行原生整机验收：

```powershell
$env:QUARTER_PACK = "$candidate.json"
$env:QUARTER_ROUTES = "$candidate.routes.json"
cargo run --release -p quarter-cut -- --compact
```

Rust 检查 96 条路线、来源与目的地唯一性、端点及实际占格。原生宿主测量两种形状、48 路吞吐和堵塞恢复，并检查每台合流器的全部接口及跳跃带状态。全部通过才发布 `latest.json`。

## 原生正例

`known-1x3-witness.json` 从已验收的 1×3 成品提取，包含 **96 条连接、734 个运输步骤**。将这些步骤固定在同一模型中：

```powershell
cargo run -p quarter-cut --features search --bin quarter-search -- `
  tasks/quarter-cut/search/known-1x3-seed.json `
  "$env:TEMP/shapez-lab/quarter-model-witness" `
  --chunks 3 --radius 0 --seconds 30 --workers 1 `
  --witness tasks/quarter-cut/search/known-1x3-witness.json
```

CP-SAT 和 SCIP 均返回 `OPTIMAL` 并保留 96 条连接。这里是零目标可行性问题，不代表密度最优。

## 2026-11-08 搜索记录

| 搜索范围 | 结果 |
| --- | --- |
| 原接口、固定摆法、自由镜像与接口 | `INFEASIBLE` |
| 扩大出口接线区、固定摆法、自由接口 | `INFEASIBLE` |
| 原摆法、固定中心、允许 180° 掉转 | `INFEASIBLE`，约 17 秒 |
| 原摆法、横纵各移动一格、自由接口和布线 | `UNKNOWN`，约 302 秒 |
| 左侧输入摆法、横纵各移动一格 | `UNKNOWN`，300 秒时限 |
| 原摆法、一格移动、SCIP 后端 | `UNKNOWN`，后端总耗时约 618 秒 |
| 两排朝外摆法、一格移动 | `UNKNOWN`，约 240 秒 |

完整后端状态和统计保存在 [results.json](results.json)。目前没有经过整机验收的 1×2 蓝图。
