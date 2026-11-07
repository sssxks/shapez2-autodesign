# 1×2 分区联合合成

最终设计已经完成，见[成品及整机测量](../README.md)。核心方法是按实际输出口划分四个 17×7 区域，把完整平台变成两个可组合的有限域问题。

## 两种区域

坐标均相对于区域左上角，x 向右、y 向下，三层 z=0..2。四个区域占满 34×14 地面。

- **左区**：17×7×3。每层西边 y=5、6 各输入一条满带；北边 x=5..8 输出四条四分之一片满带；东边 y=5 输出一条完整形状满带。总计 6 条完整输入、12 条四分之一片输出、3 条完整穿越输出。
- **右区**：17×7×1。西边 y=5 一条满带输入，北边 x=8..11 四条四分之一片满带输出，复制到三层。
- 下半部将上述建筑作 y→13−y 反射并翻转物理连接器；形状旋转操作保留。右半部平移 x+17。最后整体平移到平台有效地面原点 (3,3)。

左区两种输入允许交换用途，保证整体流量守恒和每个出口满速。接口固定后，原生完整产线已通过 48 路吞吐验收。

## 求解模型

`src/grid.rs` 为每个格子建立建筑类型、12 个有向端口、形状、流量及跳跃配对变量。几何、形状运算、容量分别使用可复用的有限关系表，避免将形状与流量的所有组合展开到每格几何表。

`--cores` 固定每组的三级完整切割及半片旋转位置；四分之一片的合并、旋转、出口归属、三层升降与跳跃线路由求解器共同决定。模型允许跨原单元共享旋转器。所有选中的合流器输入流量均为正。

每条满带流量为 4 个半物件/秒单位，完整切割器处理 1 单位，旋转器最多 2 单位。全局约束同时守恒物件流、活动流分支数量与切割数量。供料穿越单独守恒完整形状，不能计入四分之一片输出。

求解器使用原生 OR-Tools CP-SAT，Rust 绑定为 `cpsat-rs`。共享 `shapez-solver` 提供整数线性约束与有限关系；任务层负责游戏语义。首次左区三层求解 90 秒得到 12/12 个四分之一片满速出口以及三条满速穿越带。右区单层固定切割核心的模型约 2.4 秒返回最优解。这里的最优指本模型的建筑格数目标，不宣称全球最低密度。

## 已保存的解

- `bank-left.buildings.json`、`bank-right.buildings.json`：可直接装配的原生建筑。
- `bank-left.witness.json`、`bank-right.witness.json`：逐格建筑及端口形状/流量，可作为求解提示。
- `quarter-assemble`：反射、拼接、接口与验收夹具生成；全部原生验收通过后发布成品。

## 重现

```powershell
. tasks/quarter-cut/search/environment.ps1
cargo run --release -p quarter-cut --features search --bin quarter-grid -- `
  "$env:TEMP/shapez-lab/left-bank" --bank --bank-through --bank-height 7 `
  --cores --layers 3 --minimum 12 --seconds 90 --workers 8 `
  --hint tasks/quarter-cut/search/bank-left.witness.json --native
cargo run --release -p quarter-cut --features search --bin quarter-grid -- `
  "$env:TEMP/shapez-lab/right-bank" --bank --bank-height 7 --bank-input-row 5 `
  --bank-output 8 --cores --minimum 4 --seconds 30 --workers 8 `
  --hint tasks/quarter-cut/search/bank-right.witness.json --native
cargo run --release -p quarter-cut --bin quarter-assemble -- `
  "$env:TEMP/shapez-lab/left-bank.buildings.json" `
  "$env:TEMP/shapez-lab/right-bank.buildings.json"
```

去掉 `--hint` 可从头求解。两次带 `--native` 的实验应串行执行，共享宿主暂存目录有独占租约。分区测试同时测量完整形状穿越和四分之一片输出，整机验收则直接测量全部 48 条实体外部输出。

旧的整块单元摆放模型和探索结果保留在 [README.md](README.md) 与 [WEAVE.md](WEAVE.md)。最终成功的结构是固定切割核心、自由跨单元后处理与分区接口的组合。
