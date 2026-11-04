# 公共能力与任务所有权

公共库的 API 以游戏对象和工程问题命名。设计名称、蓝图识别条件、具体坐标、生产目标、策略选择和报告模板由任务拥有。允许任务重复小段编排。

## 公共模型

- `shapez_core::shape`：经过语法校验的 `Shape`，以及 `Layer`、`Part`、`Material`、`Color`。形状按原生形状码序列化；层数与构造策略限制由调用方决定。
- `shapez_core::geometry`：不同类型的 `ChunkPos`、`TilePos`，以及 `Direction`、`GridBounds`。
- `shapez_core::interface`：`Payload`、`SupplyPort`、`DemandPort`、`FactoryInterface`。`RateManifest` 额外描述测量时保留接收器的岛屿。
- `shapez_core::recipe`：`Operation`、`RecipeNode`、`Recipe`；`validate()` 检查依赖顺序、操作输入、颜色参数与目标。`verified()` 消费原生 trace，返回具有明确设备输出端口的 `VerifiedRecipe`。
- `shapez_core::routing`：调用者传入 `TransportProblem` 和 `SearchOptions`，返回 `RoutePlan`。问题包含网络、障碍、端点预留和搜索边界；布线无文件 I/O。
- `shapez_core::blueprint` 和 `space`：蓝图编解码、摘要和原生空间运输建筑编码。原始蓝图 JSON 保留扩展字段，领域接口使用具体类型。

`shapez-runtime` 提供 `Host`、`Request<T>`、`Response<T>`、`Suite`、`RunDirectory`。它独立于所有设计任务，也不依赖配方规划策略。

## 原生执行

`LabWorld` 创建并释放独立地图、模拟器和系统集合。`Advance` 以显式步长推进；`UnloaderSupply` 补充原生卸货仓；`RateScenario` 组织端口速率测量；`GraphExporter` 处理连接图。请求可设置 `export_graph: false` 跳过图导出。

版本相关字段访问集中到 `GameBindings`，包括形状/流体注册表、运算和卸货仓。`HostPrep` 与启动桥接属于原生适配层。

任务需要特殊导入或实验流程时，实现 `IBlueprintExperiment`：

```csharp
void Configure(JObject args);
bool KeepBuilding(GlobalTileCoordinate position);
object Run(LabWorld world, JObject args, string outputDirectory);
```

宿主通过请求中的 `experiment.assembly` 和 `experiment.type` 加载扩展，公共宿主无需引用任务程序集。扩展项目引用 `interop/LabMod/LabMod.csproj`。测试套件的 `projects` 列出构建项目。

## 添加任务

1. 在 `tasks/<name>` 创建 Cargo package；workspace 自动包含该目录。
2. 依赖所需公共 crate。将设计参数、策略、输入识别、生成与验收编排全部放在任务内。
3. 提供自己的 `src/main.rs`，即可使用 `cargo run -p <name>`；已有兼容 CLI 只承担已注册命令的分派。
4. 将用例、原生扩展、文档和模板放在同一任务目录。任务之间共享前，先确认内容有独立于设计的语义。
5. 使用 `RunDirectory::create`，保存输入快照并写出产物；成功后调用 `publish`。套件以 `${TASK:<name>}` 引用已发布的生成结果，并保存解析后的输入配置。

新增任务无需在公共模型、路由器或原生服务中增加设计分支。
