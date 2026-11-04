# 架构约定

## Crate 边界与依赖方向

公共 Rust 代码全部是库，各自拥有独立职责：

- `shapez-model`：形状、颜色、坐标、端口、加工 DAG、验证后配方及运输问题/结果模型。
- `shapez-routing`：运输求解算法；依赖 `shapez-model`。
- `shapez-blueprint`：蓝图编解码、摘要及游戏建筑编码；依赖 `shapez-model`。
- `shapez-runtime`：原生宿主生命周期、请求协议、批量执行与运行目录。

`shapez-routing` 与 `shapez-blueprint` 相互独立，调用方负责将路由结果交给蓝图编码器。没有汇总所有功能的 core crate，也没有公共可执行入口。

`tasks/<name>` 是设计任务 package，可以提供可执行文件及本任务使用的库。任务依赖所需公共库；公共 crate 不能依赖任务，任务之间也不通过依赖复用设计实现。

## 导出约定

所有 Rust 模块私有，使用 `mod`；公开接口统一在各 crate 的 `lib.rs` 显式 `pub use`。不使用 `pub mod` 或通配重导出。使用者直接导入：

```rust
use shapez_model::{Shape, ChunkPos, TransportProblem, VerifiedRecipe};
use shapez_routing::solve_transport;
use shapez_blueprint::{Blueprint, space_island};
use shapez_runtime::{Host, RunDirectory, run_suite};
```

## 任务所有权

设计名称、输入蓝图约定、具体坐标、生产目标、策略选择、CLI 编排、模板和验收规则都在任务目录。允许任务重复少量胶水或模板。新增任务不需要修改公共库或注册到统一 CLI。

CMAM 任务负责接入布局、旧版生成、生产/接入实验插件和套件；配方任务负责单层/脚手架竖列构造策略及用例枚举。

## 原生接口与运行产物

公共 `LabWorld` 管理世界与模拟器，`Advance` 显式推进模拟；`UnloaderSupply` 供料，`RateScenario` 测量通道，`GraphExporter` 导出连接图，`GameBindings` 集中游戏版本适配。

任务特殊实验实现 `IBlueprintExperiment`：

```csharp
void Configure(JObject args);
bool KeepBuilding(GlobalTileCoordinate position);
object Run(LabWorld world, JObject args, string outputDirectory);
```

宿主根据请求中的程序集路径和类型加载任务插件，不引用任务程序集。套件 `projects` 声明需要构建的扩展工程。

任务通过 `RunDirectory` 保存输入快照和产物，成功后发布 `latest.json`；套件通过 `${TASK:<name>}` 解析已发布产物。`Recipe::verified()` 将原生 trace 解析为带明确设备输出端口的 `VerifiedRecipe`。

## 添加任务

1. 在 `tasks/<name>` 创建 Cargo package，workspace 自动包含它。
2. 依赖所需公共 crate，并在本任务内实现设计、命令入口、模板与验收。
3. 通过 `cargo run -p <name> -- ...` 执行；宿主准备、测试编排等胶水也由任务入口调用公共库。
4. 本任务库同样使用私有模块和 `lib.rs` 显式重导出。
5. 语义独立于具体设计的能力才提取到对应公共 crate。
