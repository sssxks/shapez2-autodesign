# 1×2 四象限切割器：12 带输入 → 48 带输出

**原图五个接口的版本已完成：上方左口输入，左侧、右侧、下方两口输出。** 使用横向 1×2 平台，每口三层 × 四带，全部 48 条输出通过原生满速验收。

- [原图接口蓝图：quarter-cut-1x2-original.spz2bp](templates/quarter-cut-1x2-original.spz2bp)
- [原图接口的分区、联合模型、保存解和重建步骤](search/original/README.md)
- [原图接口逐带测量](results/quarter-cut-1x2-original-measurements.json)

原图接口版共 1,134 个建筑，左右两个 17×14 区域分别求出单层设计后复制到三层，没有升降机。输入 `CuCuCuCu` 和 `RrRrRrRr` 时，48 条输出每条在 120 秒内产出 240 个正确四分之一片；出口堵塞恢复通过，138 台合流器没有闲置入口。

`cargo run --locked --release -p quarter-cut -- --input <求解结果目录>` 从运行时读取的结果同时重建原图接口版与此前西侧输入版的蓝图和交互 HTML，输出到 `artifacts/quarter-cut`。默认 `--native` 验收原图接口版；传入旧模板路径可验收西侧输入版。

## 此前西侧输入版

成品使用 **横向 1×2 平台、三层建筑**。输入四个象限相同的普通单层形状，保留全部四份并统一到第一个象限：`CuCuCuCu → Cu------`。

[可导入蓝图：quarter-cut-1x2.spz2bp](templates/quarter-cut-1x2.spz2bp)。复制文件全部文本后在游戏中粘贴。需要解锁三层建筑以及切割、旋转、分流、合流、跳跃带和升降机。

## 接口

以蓝图原始朝向为准，每口三层 × 四带 = 12 带：

| 位置 | 用途 |
| --- | --- |
| 左侧 | 输入 12 带完整形状 |
| 上边左端 | 输出 12 带四分之一形状 |
| 上边右端 | 输出 12 带四分之一形状 |
| 下边左端 | 输出 12 带四分之一形状 |
| 下边右端 | 输出 12 带四分之一形状 |

12 条输入供应同一种目标形状，四个出口同时接走以保持满速。跨单元、跨层网络会交换来源，不保持输入带与输出带的一一对应。多层形状、销钉和晶体不在本次验收范围内。

## 布局

平台有效地面 `34×14×3`，划分成四个 `17×7` 区域。每区生产 12 条满带四分之一片；左侧两区同时把各三条完整形状满带送往右侧。左区联合求解三层建筑和流量，右区复用已验收的单层解。下半部反射物理接口，保留形状旋转运算。

完整成品共 **1,096 个建筑**：144 台完整切割器、164 台旋转器、144 台合流器、38 台分流器、18 台升降机，以及 588 个运输建筑。144 台切割器全部承担满负荷切割。不同单元的同向切片可以合流，共享旋转器；跳跃带和跨层连接共同完成供料穿越及出口汇集。

**全部合流器入口都有实际连接。** 142 台双输入合流器和 2 台三输入合流器均逐口通过原生检查。三输入合流器的三个入口全部使用。

## 运行和重建

求解结果在运行时读取，不嵌入可执行文件，也不要求存在于编译机器上。输入目录结构如下：

```text
<输入目录>/
  bank-left.buildings.json
  bank-right.buildings.json
  bank-left.witness.json
  bank-right.witness.json
  original/
    left.buildings.json
    right.buildings.json
    left.witness.json
    right.witness.json
```

建筑文件用于蓝图装配，witness 用于物料流可视化。缺失或无法解析的文件会报告具体路径。

```powershell
cargo run --locked --release -p quarter-cut -- --input artifacts/quarter-cut/solutions --output artifacts/quarter-cut
cargo run --release -p quarter-cut -- --native  # 本地游戏原生整机验收
cargo run --release -p quarter-cut --bin quarter-assemble -- `
  artifacts/quarter-cut/solutions/bank-left.buildings.json `
  artifacts/quarter-cut/solutions/bank-right.buildings.json
```

编译需要 Rust、OR-Tools 9.15（含 `protoc`）以及 SCIP 后端依赖。quarter-cut 始终启用搜索工具，无需 `--features search`。设置 `ORTOOLS_PREFIX` 指向 OR-Tools 安装目录、`PROTOC` 指向其 `bin/protoc.exe`，并将该 `bin` 加入 `PATH`。重建运行时读取已生成的求解结果，不运行求解器，也不需要游戏。输入默认是当前目录下的 `artifacts/quarter-cut/solutions`，可用 `--input` 指定；输出可用 `--output` 指定，默认在当前目录的 `artifacts/quarter-cut` 下生成蓝图和交互 HTML。进入 `tasks/quarter-cut` 后直接 `cargo run --release` 也可以。`quarter-assemble` 从指定分区文件重建并执行原生验收；验收需要本地游戏、.NET 和准备过的隔离宿主，并且串行运行。

[fresh-clone 复现、完整依赖与三种运行方式](search/REPRODUCIBILITY.md)。

[分区模型、已保存解及重新求解方法](search/BANKS.md)。搜索器和装配器均为 Rust，通过已有 Rust 绑定调用原生 OR-Tools CP-SAT。没有 Python 代码。

旧的 [1×3 蓝图](templates/quarter-cut-1x3.spz2bp) 保留；`cargo run --release -p quarter-cut -- --solve` 仍可重新生成旧布局。传入该模板路径时，入口按平台类型选择原来的接口和验收夹具。

其他实验入口：`--cell`、`--mirror-cell`、`--middle-cell`、`--middle-cell-2`、`--jump-cell`、`--capacity`、`--definitions`。

## 原生验收

本机 Shapez 2 **1.2.0-rc3**，完整组装验收：

- 单个 `Foundation_1x2`；原生蓝图导入、全部占地、接口槽检查通过。
- 12 条输入每条 2 个/秒；48 条输出每条 2 个/秒，总输出 96 个/秒。
- `CuCuCuCu`：预热 180 秒，测量 120 秒，48 条输出每条 **240 个 `Cu------`**。
- `RrRrRrRr`：同样测量，48 条输出每条 **240 个 `Rr------`**。
- 全部出口堵塞 20 秒，解除并预热后，48 路全部恢复满带。
- 144 台合流器全部连接，闲置接口数 0；无禁用接收器或阻塞发射器。
- 图库图标、格式版本 5、无旧版警告检查通过。

[仓库内逐带原始测量](results/quarter-cut-1x2-measurements.json)。首次完整报告：`%TEMP%/shapez-lab/runs/1794078205537-9924-1/report.json`。从默认入口重新导入模板的报告：`%TEMP%/shapez-lab/runs/1794078269676-11732-1/report.json`。

只有整机和图库用例全部通过，才更新 `%TEMP%/shapez-lab/tasks/quarter-cut/latest.json`。成品不包含测试用太空带、供料源或测量器，不操作个人游戏或存档。
