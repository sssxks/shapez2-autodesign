# 架构约定

## Crate 边界与依赖方向

公共 Rust 库：

- `shapez-model`：形状、颜色、坐标、端口、加工 DAG、验证后配方及运输问题/结果模型、格子级路线、平台 socket 和端口清单。
- `shapez-routing`：平台间及平台内运输求解算法；依赖 `shapez-model`。
- `shapez-blueprint`：蓝图编解码、摘要、生产单元放置及游戏建筑编码；依赖 `shapez-model`。

蓝图在输入边界解析为 `Blueprint`、`BlueprintContent`、`IslandBlueprint`、`BuildingBlueprint`、`Island` 和 `Building`。配置为解码后的 `Configuration(Vec<u8>)`，旋转为受约束的 `Rotation`；图标为四槽 `BlueprintIcon`。公共模型和任务算法均不携带原始 JSON、`serde_json::Value` 或未知字段字典。Newtonsoft 的 `$values`/`$value` 包装只在解码时处理；遇到未建模字段会报错，避免重写时丢失信息。具体格式与原生验证见[蓝图格式说明](blueprint-format.md)。
- `shapez-runtime`：原生宿主生命周期、请求协议、批量执行与运行目录。

`shapez-routing` 与 `shapez-blueprint` 相互独立，调用方负责将路由结果交给蓝图编码器。没有汇总所有功能的 core crate，也没有公共可执行入口。

`tasks/<name>` 是设计任务 package，可以提供可执行文件及本任务使用的库。任务依赖所需公共库。

## 任务所有权

设计名称、输入蓝图约定、具体坐标、生产目标、策略选择、CLI 编排、模板和验收规则都在任务目录。允许任务重复少量胶水或模板。新增任务不需要修改公共库或注册到统一 CLI。

CMAM 任务负责接入布局、旧版生成、生产/接入实验插件和套件；配方任务负责单层/脚手架竖列构造策略及用例枚举。

## 原生接口与运行产物

公共 `LabWorld` 管理世界与模拟器，`Advance` 显式推进模拟；`BlueprintRunner` 拥有蓝图运行流程，`BlueprintLoader` 负责原生导入和放置；`UnloaderSupply` 供料，`RateScenario` 测量通道，`GraphExporter` 导出连接图，`GameBindings` 集中游戏版本适配。

C# 在协议边界通过 `JsonInput` 严格校验 JSON，一次性解码为 `LabArguments` 及配方、端口、供料等类型。操作和插件只接收类型化参数，业务逻辑不使用 `JObject`/`JToken`。`BlueprintRunOptions`、`RateManifest`、`SupplyManifest`、`RecipePlan` 负责语义校验；错误类型、负数时间、无效测量窗口和配方前向引用均在执行前拒绝。`TilePosition` 与 `ChunkPosition` 区分格子和区块坐标。实验返回 `ExperimentResult.Accepted(details)` 或 `Observed(details)`，宿主直接读取类型化的 `Verified`，不再序列化后查找字段。

`LabWorld` 按创建顺序登记独立资源，并按逆序释放；构造中途失败也回收已创建对象。清理会尝试全部资源，并聚合异常。原生 `MapModel` 和 `Simulator` 的 `Dispose` 只解除自己注册的事件，不代替世界释放其余资源。

`SimulationStepper` 是新世界和存档副本共用的推进实现。`Advance` 的前后回调均接收 `(模拟时间, 实际步长)`；前回调收到起始时间，后回调收到更新后的时间。探针和信号驱动直接使用传入步长：

```csharp
world.Advance(seconds, 10, production.BeforeTick);
// void BeforeTick(Ticks start, Ticks delta)
```

游戏适配明确针对 1.2.0-rc3。模拟系统直接调用原生构造函数，私有字段和会话初始化反射集中在 `GameBindings`。`HostPrep` 先在内存中生成所有补丁，再检查预期的五处数据路径调用、一个快速启动构造函数、一个 Prepare 和一个 Tick；缺项时不写 DLL，也不保留旧的 `patch-audit.json` 成功标记。

运行入口仍通过 Rust 调用 `dotnet build`。宿主启动时使用 `SHAPEZ_LAB_PIPE` 指定专属命名管道；C# 创建服务端，Rust 连接并校验协议版本、宿主 PID 和隔离数据路径。双方交换 4 字节小端长度前缀的 UTF-8 JSON 帧，单帧上限 64 MiB，一次执行一个请求。C# 的 `LabPipe` 在线程中收发，`LabService.Update` 在 Unity 主线程执行请求；退出也作为请求确认后执行。管道断连使宿主退出，请求超时使 Rust 终止宿主。Rust 的 I/O 线程随宿主退出解除阻塞并回收。运行目录只承担输入与产物存储，`ready.json` 只归档已收到的握手。

协议分帧测试随 `cargo test -p shapez-runtime` 执行。准备好本地宿主后，可运行 `cargo test -p shapez-runtime live_pipe -- --ignored`，验证错误请求后的继续使用、控制端断连退出、请求超时回收和游戏异常退出。

可识别请求 ID 的信封或参数错误返回本次请求的失败响应；无效 ID、损坏的帧或断连终止连接。只有成功执行并写出确认响应的 shutdown 才结束服务。Unity 主线程使用非阻塞回包入队；后台任务负责全部管道 I/O，关闭时中断 I/O，任务完成后释放队列和取消源。Rust 传输线程只读写字节，协议层直接序列化请求和解码类型化响应；归档结果保留为 `RawValue`，不构建 `Value` 树。

`catalog` 直接返回 `BuildingCapability[]`，只包含建筑 ID、占格、连接器种类与位置、切割器速度与延迟；不再遍历 Unity 对象导出全部字段，也不生成中间定义文件。蓝图文件入口只接受游戏的 `SHAPEZ2-…$` 格式，不接受展开的 JSON；任务也不额外导出完整蓝图 JSON。

`cargo test -p shapez-runtime live_prepare -- --ignored` 验证不完整副本的补丁准备会失败、保持输入 DLL 字节不变并移除旧成功标记。

任务特殊实验实现 `IBlueprintExperiment`：

```csharp
void Configure(LabArguments args);
bool KeepBuilding(GlobalTileCoordinate position);
ExperimentResult Run(LabWorld world, LabArguments args, string outputDirectory);
```

宿主根据请求中的程序集路径和类型加载任务插件，不引用任务程序集。套件 `projects` 声明需要构建的扩展工程。

任务通过 `RunDirectory` 保存输入快照和产物，成功后发布 `latest.json`；套件通过 `${TASK:<name>}` 解析已发布产物。`Recipe::verified()` 将原生 trace 解析为带明确设备输出端口的 `VerifiedRecipe`。

## 添加任务

1. 在 `tasks/<name>` 创建 Cargo package，workspace 自动包含它。
2. 依赖所需公共 crate，并在本任务内实现设计、命令入口、模板与验收。
3. 通过 `cargo run -p <name> -- ...` 执行；宿主准备、测试编排等胶水也由任务入口调用公共库。
4. 本任务库同样使用私有模块和 `lib.rs` 显式重导出。
5. 语义独立于具体设计的能力才提取到对应公共 crate。

## 可组合的实体产线设计

`priority-merge`、`diagonal-cut` 和 `quarter-cut` 共同使用以下设计链路：

1. 任务定义 `Cell`，包含局部建筑、完整 `footprint` 和命名 `CellPort`。多格建筑的非锚点占地在单元定义中声明。
2. `Cell::placed(Placement)` 同时旋转、平移建筑、占地和端口。布局通过 `cell.port("input")` 等名字连接单元。
3. 任务建立 `TileRoutingProblem`，声明地板几何、障碍及 `TileNet`。`ExchangeGroup` 由任务显式声明等价加工阶段；只有同组终点可以交换。不同优先级或不同流量阶段不得共用组。
4. `shapez-routing` 返回 `Vec<TileRoute>`；结果保留实际选择的终点。`route_buildings` 将路线转换为原生建筑，求解器不依赖蓝图 crate。
5. `FoundationSocket` 同时生成外部路由端点、`Blueprint::with_harness` 测试夹具，以及传给原生插件的 `FactoryBeltPort` 清单。

`FoundationGeometry::strip` 对应当前使用的原生 1×N 平台族，采用设计坐标系中的长轴 x、短轴 y；不是任意平台形状生成器。`socket_tile` 提供四带接口的格子坐标，`socket_harness` 提供相邻运输平台坐标。原生加载器继续使用游戏 `IslandLayoutQuery` 验证实际占地。

### 路由策略

- `solve_tiles_sequential`：固定楼层、按排列顺序占地、确定性重试；用于优先合并器和对角切割器。
- `solve_tiles_negotiated`：多层、拥堵历史成本、终点交换及发射器干扰资源协商；用于四象限切割器。

两者共用 `TileRouteStep` 的移动与终点规则，以及同一蓝图编码器。`TileResource::Occupancy` 表示真实占地（含升降途中楼层），`LauncherRange` 表示同向发射器的干扰范围；不存在用于模拟干扰的虚拟楼层。算法预算由任务传入。平台间运输仍使用 `TransportProblem` / `solve_transport`，其区块级拓扑独立于格子级路线。

### 原生实验组合

`BeltPortManifest.Parse` 在插件 `Configure` 中解析端口清单；`Bind` 在世界建立后绑定精确位置的原生发送器、接收器。角色及楼层内通道身份来自设计，插件不再重复计算平台边缘坐标。

- `BeltSource`：带背压的速率供料，最多保留一个物品的供料额度；速率为零时不发送。
- `BeltMeter`：按形状计数、可选允许形状集合以及输出阻塞；允许形状集合用于检查串线。
- `BeltMeasurement.Run`：预热、清零、测量；任务继续拥有供料函数、时间和验收标准。

这组组件用于逐带产线验收。已有 `RateScenario` 继续承担包含流体的工厂供需清单测量，复用同一个 `LabWorld.Advance` 模拟推进入口。

### 验证生成链路

```powershell
cargo run -p priority-merge
cargo run -p diagonal-cut
cargo run --release -p quarter-cut -- factory artifacts/quarter-cut --native
```

原生验收串行执行，共享宿主暂存目录只允许一个实例持有。quarter-cut 的 `factory` 在内存中求解并装配 1×2 蓝图及离线交互视图；添加 `--native` 验收当前结果。`grid` 求解区域，`native <blueprint>` 验收已有 1×2 工厂蓝图。

`shapez-viz` 只接受类型化场景（占格、建筑 owner、连接、分区、外部接口）并生成离线 HTML，不依赖游戏或求解器。任务负责把自己的蓝图及流量见证适配成场景。quarter-cut 的确定性装配位于任务库，默认入口与原生验收入口共享同一实现。

## Task execution boundary

Each task owns one executable with subcommands. Solver, assembly, visualization and acceptance exchange typed in-memory values. A generated JSON file must not be the input to the next stage.

`run_suite` accepts a typed `Suite`; `run_suite_file` is only the CLI boundary for human-authored TOML. Path substitution visits declared path fields, not arbitrary payload strings. Native recipe, rate and production requests contain their typed payloads directly. JSON remains the game's blueprint representation, the native wire encoding and terminal diagnostic output.

quarter-cut uses one `RegionSolution` for both buildings and material flow. Its factory command solves both regions and passes a `FactorySolution` to assembly and visualization. Explicit restart checkpoints are versioned binary files, separate from the ordinary execution path. Search constraints are TOML; final user artifacts are blueprints and HTML.
