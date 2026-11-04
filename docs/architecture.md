# 架构约定

## Crate 边界与依赖方向

公共 Rust 代码全部是库，各自拥有独立职责：

- `shapez-model`：形状、颜色、坐标、端口、加工 DAG、验证后配方及运输问题/结果模型。
- `shapez-routing`：运输求解算法；依赖 `shapez-model`。
- `shapez-blueprint`：蓝图编解码、摘要及游戏建筑编码；依赖 `shapez-model`。

蓝图在输入边界解析为 `Blueprint`、`BlueprintContent`、`IslandBlueprint`、`BuildingBlueprint`、`Island` 和 `Building`。配置为解码后的 `Configuration(Vec<u8>)`，旋转为受约束的 `Rotation`；图标为四槽 `BlueprintIcon`。公共模型和任务算法均不携带原始 JSON、`serde_json::Value` 或未知字段字典。Newtonsoft 的 `$values`/`$value` 包装只在解码时处理；遇到未建模字段会报错，避免重写时丢失信息。具体格式与原生验证见[蓝图格式说明](blueprint-format.md)。
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

运行入口仍通过 Rust 调用 `dotnet build`。宿主启动时使用 `SHAPEZ_LAB_PIPE` 指定专属命名管道；C# 创建服务端，Rust 连接并校验协议版本、宿主 PID 和隔离数据路径。双方交换 4 字节小端长度前缀的 UTF-8 JSON 帧，单帧上限 64 MiB，一次执行一个请求。C# 的 `LabPipe` 在线程中收发，`LabService.Update` 在 Unity 主线程执行请求；退出也作为请求确认后执行。管道断连使宿主退出，请求超时使 Rust 终止宿主。Rust 的 I/O 线程随宿主退出解除阻塞并回收。运行目录只承担输入与产物存储，`ready.json` 只归档已收到的握手。

协议分帧测试随 `cargo test -p shapez-runtime` 执行。准备好本地宿主后，可运行 `cargo test -p shapez-runtime live_pipe -- --ignored`，验证错误请求后的继续使用、控制端断连退出、请求超时回收和游戏异常退出。

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
