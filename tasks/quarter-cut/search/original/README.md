# 原图接口的 1×2 四象限切割器

已完成原生整机验收。使用横向 `Foundation_1x2`，上边左口输入，左、右、下边两口输出；每口三层 × 四带。全部 48 条输出在两种输入下均为 2 个/秒。

- [可导入蓝图](../../templates/quarter-cut-1x2-original.spz2bp)
- [逐带测量](../../results/quarter-cut-1x2-original-measurements.json)
- [原生整机及图库报告](../../results/quarter-cut-1x2-original-report.json)

原生宿主只操作隔离实验环境。报告中五个开放的太空带端点是测试夹具的外部端点；所有工厂合流器入口都已连接。

## 固定接口与分区

有效地面是 34×14，坐标从有效地面左上角开始，x 向右、y 向下。原生建筑坐标加 (3,3)。所有接口在三层重复。

| 接口 | 局部坐标 | 每层用途 |
| --- | --- | --- |
| 北侧左口 | x=5..8, y=-1 | 输入四条完整形状满带 |
| 西侧 | x=-1, y=5..8 | 输出四条四分之一片满带 |
| 东侧 | x=34, y=5..8 | 输出四条四分之一片满带 |
| 南侧左口 | x=5..8, y=14 | 输出四条四分之一片满带 |
| 南侧右口 | x=25..28, y=14 | 输出四条四分之一片满带 |

沿 x=17 分成左右两个 17×14 区域。左区从北侧接收四条满带，生产西侧和南侧左口的八条满带，并在 y=5、8 各向右区输送一条完整形状满带。右区接收这两条原料带，生产东侧与南侧右口的八条满带。左右单层设计分别复制到三层；此版本不使用升降机。

17×14 是本次选定的分区边界，不是求解器优化分区大小的结果。中间两个原料接口同时固定了两区的流量与形状，使两次区域求解可以直接拼接。

## 如何得到布局

右区固定八组切割核心，联合求解共享旋转、合流、分流与跳跃运输，36 秒左右得到单层解并通过原生测试。

左区不能直接照搬右区。早期固定排布把切割输出夹在北侧输入和相邻机器之间；还有排布把切割器放得太靠近连续四条满带出口，挤掉了后处理的位置。固定排布的无解结果对应这些具体排布。完全自由的核心装填与完整格网联合求解在本次时间限制内没有返回满产解。

最终模型给八组核心指定不同的位置范围：两组朝西、六组朝南；范围见 `left-core-domains.json`。范围中的原点坐标、镜像以及所有运输建筑、形状、流量一起交给 CP-SAT 选择。每个核心含三台完整切割器和两台半片旋转器；不同核心的四分之一片可以共享旋转器与合流器。这里没有先固定最终位置再单独布线。

这一模型有 23,684 个变量、45,131 条约束，本次冷启动 16 workers 用时 76.42 秒获得 8/8 个满带输出及两条原料穿越带。`--feasibility` 使用常数目标，状态 `OPTIMAL` 表示找到了满足全部约束的解，不表示建筑数量最少。

整机共 1,185 个建筑：144 台切割器、177 台旋转器、144 台合流器、48 台分流器和 672 个运输建筑。132 台双输入合流器、12 台三输入合流器的所有入口均通过原生连接检查。两种输入 `CuCuCuCu`、`RrRrRrRr` 的 48 条输出分别在 120 秒内产出 240 个正确形状；出口堵塞恢复通过。

## 从保存的解重建

`left.buildings.json`、`right.buildings.json` 是区域局部建筑；对应的 `*.witness.json` 保存逐格端口、形状与流量。它们全部属于仓库输入，不依赖临时目录中的设计数据。

从两个保存的区域装配并执行原生验收：

```powershell
cargo run --release -p quarter-cut --bin quarter-assemble -- --original `
  tasks/quarter-cut/search/original/left.buildings.json `
  tasks/quarter-cut/search/original/right.buildings.json
```

需要与主任务相同的本地游戏、.NET 和隔离宿主。此命令已通过：`factory: PASS`、`library-icon: PASS`。

## 重新搜索

```powershell
. tasks/quarter-cut/search/environment.ps1
cargo run --release -p quarter-cut --features search --bin quarter-grid -- `
  artifacts/original-left --original-half left --free-core-pack `
  --core-domains tasks/quarter-cut/search/original/left-core-domains.json `
  --minimum 8 --feasibility --seconds 240 --workers 16
cargo run --release -p quarter-cut --features search --bin quarter-grid -- `
  artifacts/original-right --original-half right --cores `
  --minimum 8 --seconds 120 --workers 8 `
  --hint tasks/quarter-cut/search/original/right.witness.json
```

左区也可加 `--hint tasks/quarter-cut/search/original/left.witness.json`。冷启动并行搜索的用时和返回布局会变化；保存的解用于确定性重建。整个流程使用 Rust 和已有原生求解器绑定，没有 Python。
