# CMAM 紧凑接入任务

任务拥有原始蓝图约定、接入策略、旧版对比、报告模板和生产验收插件。设计参数在 `src/design.rs`；公共库无需了解 CMAM。

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

每次生成写入 `%TEMP%/shapez-lab/runs/<run-id>`，包含源蓝图与 RGB 模板快照、完整蓝图、`manifest.json` 和任务自己的 `review.html`。成功后更新 `tasks/cmam-compact/latest.json` 指针。测试套件使用 `${TASK:cmam-compact}` 解析已发布的产物。

使用 `--previous <blueprint>` 显式提供旧版对比输入。旧版 `connect` 输出发布为 `cmam-legacy`；对应验收套件为 `tasks/cmam-compact/tests/legacy.json`。

原生生产和旧版接入实验由本目录的 `interop/CmamTask.csproj` 提供。套件声明构建项目及实验类型，公共 LabMod 通过 `IBlueprintExperiment` 加载。速率测试直接使用公共端口测量能力。

求解器在有障碍的三维网格中搜索共享运输树，支持同层转弯、星际分流、上下升降及升降转弯；保留终端进入空间。固定枚举 80 种布线顺序，选择星际段数量最少的可行解。此次解为 375 段；这是有界搜索结果，未声明全局最优。

## 满速验收

```powershell
cargo run -- batch tasks/cmam-compact/tests/compact.json --timeout 300
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
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

`cargo run -- batch tasks/cmam-compact/tests/production.json --timeout 300` 保留完整 CMAM 电路和加工链，持续补充原生卸货站货仓，将目标形状信号送入原有全局接收器的输出导线，按原生信号时钟更新。只监听现有垃圾桶事件；不切断生产带、不替换筛选器。当前 CMAM 的合格品出口是 `(9,4)` 平台，其他位置的销毁记录为中间产物或筛选废品。该出口位置是此设计专用的约定。

参数 `production_manifest` 指定供料卸货站，`target` 指定目标，`seconds` 指定完整模拟时长，`min_produced` 规定合格成品下限。每例输出 `<name>-production.json`，含合格输出计数、全部销毁形状和销毁坐标。达到模拟时长并不等于生产成功，生产验收必须设置 `min_produced`。

2026-11-04 原生游戏实测：

- `WrWr----:CuCu----`：600 秒合格输出 5,746。此配方在交付蓝图上通过完整生产测试；现场停产需继续检查实际存档的信号、供料和改动。
- `WwcbWwcb:P-cwP-cw:WgP---P-`：600 秒合格输出 0，末端丢弃 5,073 个 `WwcbWwcb:cwcwcwcw:WgP---P-`。另一次 900 秒运行丢弃同样错误形状 12,273 个，合格输出仍为 0。
- `WwcbWwcb:cwcwcwcw:WgP---P-`：600 秒合格输出 4,756。
- 单独的 `P-cwP-cw`：600 秒合格输出 0，丢弃 9,145 个 `cwcwcwcw`。

故障是原 CMAM 的销钉/晶体混合层加工：本地游戏 `ShapeOperationCrystallize.ExecuteInternal` 将每层的空位和销钉都替换为晶体；这台机器没有在该路径恢复目标中需要保留的销钉。原目标的第二层因此被做成整层白晶，最终筛选器正常拒收。紧凑接入生成器完整复制生产平台及其电路，未修改此加工逻辑。修复需要改造混合层的生成与组合流程。上述生产套件有意保留 `min_produced: 1`，当前原目标及最小复现用例会报失败；未交付或覆盖所谓修复蓝图。

诊断记录：`%TEMP%/shapez-lab/runs/1793797114261-18528/*-production.json`；900 秒记录：`%TEMP%/shapez-lab/runs/1793796811462-24484/production-report.json`。

半形状配方记录：`%TEMP%/shapez-lab/runs/1793798197194-12116/half-windmill-circle-production.json`。

读取现场快照：设置 `SHAPEZ_LAB_SAVE` 为 `.spz2` 自动存档路径，再运行 `cargo run -- probe --operation inspect-save`。宿主先将文件复制到本次临时目录的 `inspection.spz2`，通过原生存档加载器恢复地图，输出常量/全局/可控无线目标信号、卸货站存货和模拟节点类型计数。默认不推进地图；批量用例可通过 `seconds` 推进副本以恢复无线信号传播。始终不保存回源存档；使用后清除该环境变量即可恢复新地图测试。

21:22 自动存档副本推进 5 秒后，四台 CMAM 的可控无线接收器分别读取频道 5250–5253，目标均不是 `WrWr----:CuCu----`。现场记录在 `%TEMP%/shapez-lab/runs/1793798827562-19796/result-1.json`；需取得故障目标正在运行的存档，才能继续沿该现场追踪。


## 独立入口

```powershell
cargo run -p cmam-compact -- build <source.spz2bp> --rgb <rgb.spz2bp>
cargo run -- batch tasks/cmam-compact/tests/compact.json --timeout 300
```

`src/compact.rs` 暴露 `build_compact(&Blueprint, &Blueprint, Option<&Blueprint>, &CmamDesign)`，返回内存中的 `CompactBuild`；调用 `.write(output)` 才写文件。CLI 编排与输入快照在任务自己的 `src/commands.rs`。

## 2026-11-05 重构验收

- 独立任务入口重新生成：54,906 建筑、422 空间实体、375 运输段。
- `tests/compact.json` 六项通过，300 条形状通道与 180 条流体通道的最低额定速率比均为 1.0。
- `tests/production-smoke.json` 两项通过，600 模拟秒产出 5,746 件 `WrWr----:CuCu----`，随后宿主继续响应。
- 迁移后的旧版实验插件在已有旧蓝图上通过 480 入口连通性测试及世界释放后继续响应测试。
- 完整生产套件仍有两个既有失败：`pin-crystal-target` 和 `isolated-pin-crystal` 产量为 0。保留 `min_produced: 1` 与非零退出状态。

验证报告分别位于 `%TEMP%/shapez-lab/runs/1793824872171-25928-0`（满速）、`1793825012233-27136-0`（生产冒烟）、`1793825069516-19576-0`（旧版插件）和 `1793824542364-23936-0`（完整生产套件）。
