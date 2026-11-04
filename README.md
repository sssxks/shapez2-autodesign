# Shapez Lab

Rust 蓝图求解/生成工具与独立 Shapez 2 原生测试实例。当前适配本机 **1.2.0-rc3**。

## 当前设计：CMAM 紧凑满速接入

输入为原始 `CMAM not connected.spz2bp` 与蓝图库中的 RGB 转置器。

- 两组卸货站，每组保持 **销钉、圆形、方形、星形、风车、R、G、B** 顺序，兼容原来的八货种列车编组。沿同一条向西行驶的铁路串联，两组车站分别供应三个、两个 CMAM 模块。
- 原生星际带/管道为普通带速度的 **4 倍**，每口三层四宽相当于 48 条普通带。一口最多供四个 12 带入口，五个同类入口需要至少两口。此版按所有入口同时满速设计，无需依赖形状互斥。
- 分流全部在星际带/管道上进行。普通平台内新增建筑仅有 **5 个 RGB 转置器，共 5,265 个建筑**；无平台内分带。
- 使用三个空间层，升降带跨越其他线路及生产主体。CMAM 保持原始坐标和内部建筑；铁路位于其北侧。
- 整张蓝图 **54,906 建筑、422 个空间实体**：20 个实体建筑平台、375 个星际运输段、27 个铁路/车站实体。平面包围框 **30×15**。
- 旧版为 142,208 建筑、500 空间实体、45×19 包围框。新版建筑数减少 **61.4%**，包围框面积减少 **47.4%**。面积包括所有岛屿锚点和建筑实际占用格；不是简单以锚点计算多格平台。

生成命令：

```powershell
$library=Join-Path $env:USERPROFILE 'AppData\LocalLow\tobspr Games\shapez 2\blueprints'
cargo run --release -- compact "$library\CMAM not connected.spz2bp" `
  --rgb "$library\personal\09 流体运输 Fluid Transfer\RBG三口转三层 RGB Transposer.spz2bp"
```

输出统一在 `%TEMP%/shapez-lab/compact`：

- `CMAM compact full-rate.spz2bp`：完整蓝图。
- `manifest.json`：供料来源、480 条目标通道、生产主体边界、逐段三维路由与统计。
- `review.html`：可切换空间层的交互线路图和旧版对比。
- `validation.json`、`rate-report.json`：交付时复制的原生验证结果。

求解器在有障碍的三维网格中搜索共享运输树，支持同层转弯、星际分流、上下升降及升降转弯；保留终端进入空间。固定枚举 80 种布线顺序，选择星际段数量最少的可行解。此次解为 375 段；这是有界搜索结果，未声明全局最优。

## 满速验收

```powershell
cargo run -- batch tests/suites/compact.json --timeout 300
cargo test
cargo clippy --all-targets -- -D warnings
```

本机通过以下原生测试：

1. 读取游戏当前速度配置，普通带 1 世界单位/秒，星际带/管道 4 世界单位/秒。
2. 原始 52,375 建筑及新版 54,906 建筑完整导入并模拟。
3. **全部 40 个输入口同时持续拉满**：25 个形状口 × 12 通道、15 个流体口 × 12 通道。
4. 预热 300 模拟秒，测量 120 模拟秒。300 条形状通道全部 **2 件/秒**；180 条流体通道全部 **20 升/秒**。最慢通道均为额定速度的 **100%**。
5. 同一宿主内连续创建/释放世界、错误请求恢复、实例正常回收。

测试持续补充真实卸货站的出料仓，保留其原生出料、空间运输、分流和平台接收器；符合“列车供货足够”的条件。生产电路在真实入口接收器后接测试收集器，以逐条检查内容与稳态速率。形状收集器按一条普通带的额定速度持续请求物料。形状校验完整形状码，流体校验 RGB 色码。没有用直接向目标注入的方式计算速率。

`rate_manifest` 指定输入/输出边界，`warmup_seconds` 指定预热时间，`seconds` 指定测量窗口。普通带额定速率由原生 `BeltSpeed` 和 `LaneConstants.ItemSpacing` 计算；本版本流体一包 10 升，流体额定量据此计算。低于额定值的 99% 或出现串料即失败。该测试验收的是整个接入网络的输送能力。

## 完整生产测试与销钉/晶体故障

`cargo run -- batch tests/suites/production.json --timeout 300` 保留完整 CMAM 电路和加工链，持续补充原生卸货站货仓，将目标形状信号送入原有全局接收器的输出导线，按原生信号时钟更新。只监听现有垃圾桶事件；不切断生产带、不替换筛选器。当前 CMAM 的合格品出口是 `(9,4)` 平台，其他位置的销毁记录为中间产物或筛选废品。该出口位置是此设计专用的约定。

参数 `production_manifest` 指定供料卸货站，`target` 指定目标，`seconds` 指定完整模拟时长，`min_produced` 规定合格成品下限。每例输出 `<name>-production.json`，含合格输出计数、全部销毁形状和销毁坐标。达到模拟时长并不等于生产成功，生产验收必须设置 `min_produced`。

2026-11-04 原生游戏实测：

- `WrWr----:CuCu----`：600 秒合格输出 5,746。此配方在交付蓝图上通过完整生产测试；现场停产需继续检查实际存档的信号、供料和改动。
- `WwcbWwcb:P-cwP-cw:WgP---P-`：600 秒合格输出 0，末端丢弃 5,073 个 `WwcbWwcb:cwcwcwcw:WgP---P-`。另一次 900 秒运行丢弃同样错误形状 12,273 个，合格输出仍为 0。
- `WwcbWwcb:cwcwcwcw:WgP---P-`：600 秒合格输出 4,756。
- 单独的 `P-cwP-cw`：600 秒合格输出 0，丢弃 9,145 个 `cwcwcwcw`。

故障是原 CMAM 的销钉/晶体混合层加工：本地游戏 `ShapeOperationCrystallize.ExecuteInternal` 将每层的空位和销钉都替换为晶体；这台机器没有在该路径恢复目标中需要保留的销钉。原目标的第二层因此被做成整层白晶，最终筛选器正常拒收。紧凑接入生成器完整复制生产平台及其电路，未修改此加工逻辑。修复需要改造混合层的生成与组合流程。上述生产套件有意保留 `min_produced: 1`，当前原目标及最小复现用例会报失败；未交付或覆盖所谓修复蓝图。

诊断记录：`%TEMP%/shapez-lab/runs/1793797114261-18528/*-production.json`；900 秒记录：`%TEMP%/shapez-lab/runs/1793796811462-24484/production-report.json`。

半形状配方记录：`%TEMP%/shapez-lab/runs/1793798197194-12116/half-windmill-circle-production.json`。

读取现场快照：设置 `SHAPEZ_LAB_SAVE` 为 `.spz2` 自动存档路径，再运行 `cargo run -- probe --operation inspect-save`。宿主先将文件复制到本次临时目录的 `inspection.spz2`，通过原生存档加载器恢复地图，输出常量/全局目标信号和卸货站存货。此模式不推进地图、不保存回源存档；使用后清除该环境变量即可恢复新地图测试。

## 项目与临时文件

- `crates/shapez-lab`：Rust workspace 中的正式 crate；蓝图读写、实例管理、CLI 和当前设计求解器。
- `interop/HostPrep`：Mono.Cecil 工程，仅修改临时游戏副本。
- `interop/LabMod`：游戏 Mod 工程；原生世界、导入、通道测试及启动桥接。
- `ShapezLab.slnx`：两个 C# 工程的 solution。
- `tests/suites`：批量用例。

本轮将新设计放在独立 `compact` 模块内，仍使用同一个 crate。通用蓝图读写、进程管理和设计代码已分文件；后续可按用户要求拆分 crate，本轮未做跨项目重构。

仓库无 Python、松散 C# 程序或游戏 DLL。旧实验归档在 `%TEMP%/shapez-lab/legacy-20261104-175213`。所有构建产物、生成蓝图、SDK、游戏副本和日志均在 `%TEMP%/shapez-lab`。交付蓝图另复制到用户蓝图库。`.cargo/config.toml` 已将本机编辑器后台构建也定向到系统临时目录；换机器时调整该路径或设置 `CARGO_TARGET_DIR`。

## 独立实例与启动裁剪

首次需要本地游戏安装、Rust 工具链以及 .NET SDK：

```powershell
.\scripts\bootstrap.ps1  # SDK 已安装时跳过
cargo run -- prepare
cargo run -- probe
```

`SHAPEZ_GAME_ROOT` 可覆盖安装目录，`SHAPEZ_LAB_DOTNET` 可指定 SDK 的 `dotnet.exe`。游戏升级或 HostPrep 修改后重新 `prepare`。每次 `probe` / `batch` 自动构建 Mod。

实例使用自己的临时数据目录和 Mod 目录。Rust 校验 readiness PID/路径，通过独占文件租约和 Windows Job Object 管理生命周期；退出、超时及父进程异常时均回收游戏。每个用例重新建立地图、资源地图、模拟器和信号注册表。不会操作个人游戏进程或存档。

临时副本的启动桥接只运行原生设置、玩家基础对象、游戏模式/核心数据初始化；跳过交互控制、音频系统、渲染系统、HUD、成就和调试控制台初始化，禁用摄像机与宿主地图每帧 Tick。保留内容加载所需的图形设备。此版本的资源加载不能直接使用 Unity `-nographics`，因此采用停止渲染而保留资源加载的方式。

本次裁剪实例启动约 **3–4 秒**；新版的 420 模拟秒满速用例（含导入、测量和图导出）实测约 **5.6 秒**。完整六项套件结果在运行目录 `report.json`，各项含 `wall_seconds`，启动数据在 `ready.json`。

Managed DLL 是独立副本，其他只读发行资源使用硬链接。HostPrep 只处理带标记的暂存目录，并输出 `patch-audit.json`；游戏 DLL 不提交或分发。

## 通用命令与旧设计

`inspect` 输出蓝图摘要，`decode` 解压到临时目录。`probe --operation capacities` 查询速度和空间运输类型。批量套件支持 `${LAB}`、`${LIBRARY}` 路径变量，`expected_ok: false` 可测试错误恢复。失败返回非零状态，日志和响应保留在当前运行目录。

旧版 `connect --splitter ... --rgb ...`、`tests/suites/cmam.json` 和原有输出保留供对比；其可达性测试不作为新设计的吞吐验收。
