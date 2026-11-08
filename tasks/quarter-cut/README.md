# 四象限切割器

## 求解与导出

```powershell
# 小型切割单元；输出可导入的建筑蓝图
cargo run -p quarter-cut -- grid artifacts/quarter-cut/cell.spz2bp --cell --seconds 10 --quiet

# 原图接口：两区求解 → 装配 → 蓝图和交互 HTML
cargo run --release -p quarter-cut -- factory artifacts/quarter-cut --seconds 120

# 西侧输入接口，按同样的内存流程执行
cargo run --release -p quarter-cut -- factory artifacts/quarter-cut-west --west --seconds 120

# factory 加 --native 时，将组装结果直接交给原生验收
cargo run --release -p quarter-cut -- factory artifacts/quarter-cut --native
```

`factory` 输出 `factory.spz2bp` 和 `factory.html`。每区求解的时间上限由 `--seconds` 控制；只导出满足全部输出要求的结果。原图接口为北侧 12 带输入，西、东、南侧两个接口共 48 带输出；西侧输入版在上、下各有两个输出接口。

`RegionSolution` 同时持有该次求解的建筑和物料流，`FactorySolution` 持有两个分区。装配与可视化使用同一个结果，不依赖已保存的 `buildings.json` 或 witness 文件。

## 一个入口的子命令

- `factory`：完整工厂的联合流水线。
- `grid`：有限域建筑、形状和流量求解；`--native` 验收当前区域。
- `weave`：原始端口流模型，直接输出蓝图。
- `search <seed.toml> <output.spz2bp>`：刚性切割单元与共享布线的联合求解。种子是人工配置的 `layers` 数组。
- `native <blueprint.spz2bp>`：验收指定的最终游戏蓝图。
- `legacy`：生成并验收原有 1×3 布局。
- `probe <cell|mirror-cell|middle-cell|middle-cell-2|jump-cell|definitions|capacity>`：原生实验。

旧的 `quarter-grid`、`quarter-weave`、`quarter-search`、`quarter-assemble` 独立 binary 已合并。旧 compact 的环境变量、网络导出、反馈文件及路线导入入口已由联合求解流程取代。

## 显式续算

`grid`、`weave`、`search` 支持 `--checkpoint <file>` 和 `--resume <file>`。checkpoint 是带求解器类型及版本头的二进制快照；只有显式请求才保存或恢复，正常流程不经过它。不同求解器的 checkpoint 不混用。

```powershell
cargo run -p quarter-cut -- grid artifacts/quarter-cut/cell.spz2bp --cell --quiet --checkpoint artifacts/quarter-cut/cell.checkpoint
cargo run -p quarter-cut -- grid artifacts/quarter-cut/cell-resumed.spz2bp --cell --quiet --resume artifacts/quarter-cut/cell.checkpoint
```

`grid --buildings-hint` 接受游戏蓝图，`--core-domains` 接受人工编写的 TOML（`domains` 数组，每项含 `bounds` 和 `rotation`）。Grid checkpoint 将类型编号与完整建筑语法一起保存，恢复时核对语法。

## 构建与原生验收

搜索工具始终编译，无 `search` feature gate。需要 OR-Tools 9.15、`protoc` 和 SCIP 后端；设置 `ORTOOLS_PREFIX`、`PROTOC`，将 OR-Tools 的 `bin` 加入 `PATH`。原生验收另外需要本地游戏、.NET SDK 和已准备的隔离宿主。

验收以 typed suite 直接调用 runtime，JSON 仅用于原生通信及最终诊断报告。不会先写 suite 文件再读回执行。