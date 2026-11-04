# Shapez Lab

Rust 蓝图工具与独立 Shapez 2 原生测试实例。针对本机 **1.2.0-rc3**。

## 项目结构

- `crates/shapez-lab`：蓝图解码、CMAM 接线生成、实例生命周期、JSON 批量测试。
- `interop/HostPrep`：Mono.Cecil 工程，只修改临时游戏副本的数据目录和自动启动入口。
- `interop/LabMod`：游戏 Mod 工程，原生导入蓝图、创建独立模拟世界、连接图导出与带料测试。
- `tests/suites`：可重复执行的测试定义。
- `scripts`：PowerShell 构建入口与本地 SDK 安装入口。

仓库不含 Python、松散 C# 程序、游戏 DLL、生成蓝图或运行日志。旧实验文件已迁至 `%TEMP%/shapez-lab/legacy-20261104-175213`。生成内容统一写入 `%TEMP%/shapez-lab`：Rust `target`、C# `bin/obj`、游戏副本 `host`、每次运行的 `runs`、蓝图 `adapter`。

## 启动与测试

需要 Rust 工具链和本地 Shapez 2 安装。首次准备 .NET SDK（已安装时可跳过）：

```powershell
.\scripts\bootstrap.ps1
.\scripts\dev.ps1 run -- prepare
.\scripts\dev.ps1 run -- probe
```

`SHAPEZ_GAME_ROOT` 可指定游戏安装目录，`SHAPEZ_LAB_DOTNET` 可指定 SDK 的 `dotnet.exe`。`.cargo/config.toml` 已将本机编辑器后台构建也定向到系统临时目录；换机器时调整该路径或设置 `CARGO_TARGET_DIR`。游戏升级后重新执行 `prepare`。每次 `probe` / `batch` 都先重新构建 Mod。

```powershell
.\scripts\dev.ps1 run -- batch tests/suites/cmam.json --timeout 300
.\scripts\dev.ps1 test
.\scripts\dev.ps1 @('clippy', '--all-targets', '--', '-D', 'warnings')
```

测试套件支持 `${LAB}`、`${LIBRARY}` 路径变量。每项默认 `operation: blueprint`；可指定 `expected_buildings`、模拟 `seconds`。`expected_ok: false` 用于错误恢复用例。失败返回非零退出码，详细结果保存在当前运行目录的 `report.json`，另有 `player.log`、逐项响应和模拟连接图。

Rust 管理子进程与 Windows Job Object。实例启动后验证 PID 和临时数据目录；正常结束先请求退出，10 秒后强制回收；超时和错误也会回收进程。单个暂存游戏目录只允许一个实例。测试进程通过自己的临时 `data/mods/ShapezLab` 加载 Mod，使用新的地图和模拟系统；每个蓝图用例有单独的世界及信号注册表。用户运行中的游戏和存档不参与测试。

游戏的 Managed DLL 是独立副本；其他只读发行资源用硬链接节省空间。`HostPrep` 要求临时目录标记，并写出 `patch-audit.json`。游戏 DLL 不提交或分发。

## 生成 CMAM 接线

输入为用户提供的 108 平台 CMAM，以及蓝图库现有分离器和已完成的 RGB 转置器：

```powershell
$library=Join-Path $env:USERPROFILE 'AppData\LocalLow\tobspr Games\shapez 2\blueprints'
.\scripts\dev.ps1 run -- connect "$library\CMAM not connected.spz2bp" `
  --splitter "$library\personal\08 形状运输 Shape Transfer\带分离器 Belt Spliter.spz2bp" `
  --rgb "$library\personal\09 流体运输 Fluid Transfer\RBG三口转三层 RGB Transposer.spz2bp"
```

输出 `%TEMP%/shapez-lab/adapter/CMAM connected.spz2bp` 和 `adapter.json`。后者包含逐平台输入、输出方向与总线编号。方向 `0=东, 1=南, 2=西, 3=北`。

接线按原蓝图的卸货站标记：**销钉、圆形、方形、星形、风车、R、G、B**，从左到右。铁路保持原坐标，CMAM 和输出部分整体向南移动四个空间格。CMAM 的输入标记平台被走线替代，内部生产建筑保持原样。生成版共 **500 平台、142,208 建筑**，其中新增走线平台 431 个，另放置一个 RGB 转置平台。此版选择宽松总线布局，未以占地最小化为目标。

每个 CMAM 层模块对应以下接口；五组模块水平重复：

| 位置 | 左到右输入 |
|---|---|
| 北面 | 星形、销钉、风车、RGB、RGB |
| 南面 | 圆形、RGB、方形 |

每个形状口有 3 层 × 4 通道，每个 RGB 口同样有 12 通道，颜色自上而下为 R、G、B。北面总线直接向南分配；圆、方和第三个 RGB 从右侧绕至南面。交叉使用跨度 5 的跳带/跳管，分支使用库中的分离器布局。

一个卸货口分配到多个模块，分流不会增加总供给能力。带料用例验证所有目标口的内容与可达性，不测最终 CMAM 配方产率。

## 原生带料测试边界

`adapter_probe: true` 专用于上述 CMAM 布局。测试从八个真实卸货站的 96 条已连接输出通道注入五种形状和三种流体；将生产电路截断在 CMAM 的物理接收器处，收集全部 **300 条形状通道 + 180 条流体通道**。每条形状通道校验完整形状码，每条流体通道校验色码；连接缺失、串料或规定时间内未到达都会失败。

本机已完成一次 760 模拟秒的带料测试，480 条通道全部通过。整张蓝图另以全部 142,208 建筑原生导入并运行。测试套件同时覆盖原始 CMAM、错误请求恢复和世界释放后的继续运行。

RGB 转置器作为用户蓝图库中的输入资源读取；本仓库保留使用和测试入口，不依赖已归档的旧 Python 求解器。
