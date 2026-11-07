# Shapez Lab

面向 Shapez 2 设计任务的 Rust 工具与原生执行宿主。当前适配本机 **1.2.0-rc3**。

## 代码边界

通用部分提供模型与能力；每个设计任务拥有自己的输入约定、策略、编排、模板与验收规则。任务可重复少量胶水代码。依赖方向固定为任务调用公共库，公共库独立于任务。公共模块私有，所有 API 从各 crate 的 `lib.rs` 显式重导出，使用者直接从 crate 根导入。

```text
crates/
  shapez-model/      形状、坐标、工厂端口、配方 DAG、运输问题与结果模型
  shapez-routing/    空间布线算法，依赖 model
  shapez-blueprint/  蓝图编解码与游戏建筑编码，依赖 model
  shapez-runtime/    宿主生命周期、请求协议、批量执行、运行目录
interop/
  HostPrep/          临时游戏副本与启动裁剪
  LabMod/            世界生命周期、原生运算、供料、测量、连接图和扩展接口
tasks/
  cmam-compact/      CMAM 接入设计、旧版生成、原生生产/接入插件、模板与套件
  scaffold-recipe/   脚手架竖列配方策略、原生验证编排与用例生成
  priority-merge/    三种 1×1 平台的 12 带优先合并器、蓝图生成与原生验收
```

具体设计及已有诊断保存在 [CMAM 任务](tasks/cmam-compact/README.md) 和 [配方任务](tasks/scaffold-recipe/README.md)。公共接口与添加任务的方法见 [架构说明](docs/architecture.md)。

[12 带优先合并器](tasks/priority-merge/README.md) 提供直列优先、反向直列优先和左右输入左侧优先三个版本；运行 `cargo run -p priority-merge` 一次生成并验收三份带图库图标的单平台蓝图。

## 构建与验证

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -p cmam-compact -- prepare
cargo run -p cmam-compact -- probe
```

公共部分全部为库，任务提供可执行入口：`cargo run -p cmam-compact -- --help`、`cargo run -p scaffold-recipe -- --help`。

C# 三个工程统一使用 C# 14、nullable 和 .NET 分析器，构建警告视为错误；格式由根目录 `.editorconfig` 管理。可单独执行构建和格式检查：

```powershell
& "$env:TEMP/shapez-lab/tools/dotnet/dotnet.exe" build ShapezLab.slnx -c Release
& "$env:TEMP/shapez-lab/tools/dotnet/dotnet.exe" format whitespace ShapezLab.slnx --verify-no-changes --no-restore
```

原生执行层的类型边界、世界资源所有权和模拟回调约定见[架构说明](docs/architecture.md)。

构建产物、SDK、游戏副本、生成蓝图和运行日志统一置于 `%TEMP%/shapez-lab`。每次生成或执行使用 `runs/<run-id>`；输入快照与执行报告随运行保存。成功任务通过 `tasks/<task>/latest.json` 发布产物，历史运行保留。

批量套件支持 `${LAB}`、`${REPO}`、`${LIBRARY}` 和 `${TASK:<task-id>}`。`projects` 声明需要构建的扩展项目；case 的 `experiment` 声明扩展程序集路径与类型。任务参数由任务解释。

测试失败返回非零退出状态，并保留报告。CMAM 生产套件保留两个尚未解决的销钉/晶体目标；具体状态见任务文档。

通用蓝图加载器在创建建筑前调用游戏原生 `IslandLayoutQuery`，检查建筑所有占地格的楼层、平台地板和接口槽限制；非法位置直接报出建筑类型、岛内坐标及原因。这一检查适用于所有设计任务，不依赖任务插件。模拟宿主使用模式允许的最大楼层，不验证玩家存档的科技解锁或建造货币；实际游戏放置应作为交付验收的一部分。

## 独立实例与启动裁剪

首次需要本地游戏安装、Rust 工具链以及 .NET SDK：

```powershell
.\scripts\bootstrap.ps1  # SDK 已安装时跳过
cargo run -p cmam-compact -- prepare
cargo run -p cmam-compact -- probe
```

`SHAPEZ_GAME_ROOT` 可覆盖安装目录，`SHAPEZ_LAB_DOTNET` 可指定 SDK 的 `dotnet.exe`。游戏升级或 HostPrep 修改后重新 `prepare`。每次 `probe` / `batch` 自动构建 Mod。

实例使用自己的临时数据目录和 Mod 目录。Rust 校验 readiness PID/路径，通过独占文件租约和 Windows Job Object 管理生命周期；退出、超时及父进程异常时均回收游戏。每个用例重新建立地图、资源地图、模拟器和信号注册表。不会操作个人游戏进程或存档。

Rust 继续在运行入口调用 `dotnet build` 构建公共 Mod 和任务插件。宿主控制使用一条持续连接的双向 Windows 命名管道：消息采用 4 字节小端长度前缀和 UTF-8 JSON，单条上限 64 MiB。就绪握手、请求、结果和退出均通过管道发送。C# 后台线程收发，Unity 主线程执行游戏操作；断开控制连接后宿主退出，请求超时由 Rust 回收宿主。`ready.json` 是 Rust 收到握手后保存的记录，其他输入快照、产物和报告仍保留在运行目录，均不作为控制消息的收发通道。

临时副本的启动桥接只运行原生设置、玩家基础对象、游戏模式/核心数据初始化；跳过交互控制、音频系统、渲染系统、HUD、成就和调试控制台初始化，禁用摄像机与宿主地图每帧 Tick。保留内容加载所需的图形设备。此版本的资源加载不能直接使用 Unity `-nographics`，因此采用停止渲染而保留资源加载的方式。

本次裁剪实例启动约 **3–4 秒**；新版的 420 模拟秒满速用例（含导入、测量和图导出）实测约 **5.6 秒**。完整七项套件结果在运行目录 `report.json`，各项含 `wall_seconds`，启动数据在 `ready.json`。

Managed DLL 是独立副本，其他只读发行资源使用硬链接。HostPrep 只处理带标记的暂存目录，并输出 `patch-audit.json`；游戏 DLL 不提交或分发。


## 任务命令

CMAM 入口的 `inspect` 输出蓝图摘要，`decode` 解压到独立运行目录；两个任务都提供 `prepare`、`probe` 和 `test`。`probe --operation capacities` 查询原生速度及运输类型。任务的 `test [suite.json]` 在同一宿主中执行用例，错误请求可通过 `expected_ok: false` 验收。

`decode` 输出完整类型化模型（包含图标和来源格式版本），可以直接通过 `Blueprint::read` 读取。`reencode <blueprint>` 将既有蓝图转换为当前格式，输出到新的独立运行目录，保留源文件。蓝图模型、格式及图库旧版警告的原因见[蓝图格式说明](docs/blueprint-format.md)。

读取存档副本时设置 `SHAPEZ_LAB_SAVE`，再运行 `probe --operation inspect-save`。原生宿主复制存档后加载，不保存回源文件；批量请求可通过 `seconds` 推进副本。

`.cargo/config.toml` 为本机设置临时构建目录；换机器时调整它，或通过 `CARGO_TARGET_DIR` 覆盖。`scripts/dev.ps1` 自动根据系统临时目录设置该变量。

```powershell
cargo run -p cmam-compact -- test
cargo run -p cmam-compact -- test tasks/cmam-compact/tests/production-smoke.json
cargo run -p scaffold-recipe -- plan 'WwcbWwcb:P-cwP-cw:WgP---P-'
cargo run -p scaffold-recipe -- check
cargo run -p scaffold-recipe -- test
```

CMAM 的 `test` 默认执行本任务的紧凑接入套件；配方任务的 `test` 默认读取最近一次 `check` 生成的套件。`prepare` 与 `probe` 也可从配方任务独立执行。
