# 蓝图数据模型与图库兼容

`shapez-blueprint` 负责结构化蓝图数据和游戏格式编解码。任务只操作类型化字段，不读写 JSON 路径；模型不保留原始 JSON、通用 JSON 值或未知字段映射。

## 模型

- `Blueprint`：游戏版本、岛屿/建筑蓝图内容、图库图标、来源封装版本。
- `BlueprintContent`：明确区分 `IslandBlueprint` 和 `BuildingBlueprint`。
- `Island`：i32 平面坐标、i16 高度、旋转、定义 ID、配置字节、旧版 C 配置及可选建筑蓝图。
- `Building`：i32 平面坐标、i16 层数、旋转、定义 ID 与配置字节。
- `Rotation`：解析时限制为 0–3；生成器用 `Rotation::new` 按四分之一圈归一化。
- `Configuration`：解码后的字节，编码时转 base64；空字节配置与无配置保持区别。
- `BlueprintIcon`：四个可选槽，每槽为命名图标或形状码。

旧格式的 `BinaryVersion` 和嵌套蓝图 `Icon` 有独立类型化字段。顶层旧版内嵌图标在读入时迁到 `Blueprint.icon`，编码时写入新版元数据尾部。Newtonsoft 的类型记录、数组 `$values` 和字节 `$value` 包装在输入边界解包；输出使用游戏原生导入器支持的普通数组和 base64 字符串，并保留 `Island`/`Building` 多态类型标记。

游戏格式允许省略的坐标、层数和旋转默认 0。错误字段类型、越界坐标、非法旋转、无效 base64、缺失定义 ID、未知蓝图类型以及未建模字段均报错。新增游戏或 Mod 字段需要先扩充模型；不会在重写时静默丢弃。元数据目前支持 `BlueprintIcon`，其他元数据类型明确拒绝。

`decode` 输出的 JSON 是工具的完整类型化模型，包括 `game_version`、`content`、`icon`、`source_format_version`；`Blueprint::read` 同时支持此模型 JSON、游戏载荷 JSON 和 `.spz2bp` 文本。游戏载荷使用 V/BP 字段，封装版本和图标分别放在外层头部与尾部。

## 图库旧版警告

2026-11-05 检查本机 1.2.0-rc3 的游戏程序集确认：

- `Game.Core.Blueprint.Serialization.BlueprintFormat.Version = 5`，`RecommendedVersion = 4`。
- `Game.Hud` 的 `HUDBlueprintLibraryNavEntry`、`HUDBlueprintLibrarySlot` 用 `entry.Version < 4` 控制警告图标。
- `Game.Hud.View.Model.BlueprintEntryToolbarElement.ShouldDisplayWarning` 使用相同条件，显示 `blueprint-library.legacy-blueprints-warning`。
- `BlueprintImporter.TryImport` 返回的是 `SHAPEZ2-<版本>-` 的封装版本；载荷中的 V 是另一项游戏/存档版本。

原编码器固定输出 `SHAPEZ2-1-`，因此生成文件被图库识别为旧版。原解码器还会截掉 gzip/base64 载荷后的图标元数据。新版编码统一输出：

```text
SHAPEZ2-5-<base64(gzip(游戏载荷))><图标元数据 JSON>_<元数据 UTF-16 长度>$
```

元数据与游戏 `CustomDataConverter` 一致，为带 Type/Content 的数组；Content 是序列化后的图标数据。长度按 .NET `String.Length` 的 UTF-16 单元计数。无自定义图标时使用游戏默认的岛屿/建筑图标。载荷中的游戏版本原样保留。

## 命令与验证

```powershell
cargo run -p cmam-adapter -- reencode '<既有蓝图.spz2bp>'
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo test -p shapez-blueprint local_library -- --ignored --nocapture
cargo run -p cmam-adapter -- test
```

`reencode` 先保存输入快照，再将新版文件写到 `%TEMP%/shapez-lab/runs/<run-id>/<原文件名>`，不覆盖蓝图库文件。

原生操作 `inspect-blueprint` 使用游戏自己的分解器、元数据解码器和导入器，建立 `BlueprintLibraryEntry`，报告封装版本、图库版本、旧版警告状态及图标。支持 `expected_format_version`、`expected_library_warning`、`expected_icon`；任一不匹配使请求失败。原生重新导出后再次导入，核对完整规范化载荷和图标，并保存原生导出文件。gzip 头包含时间戳，因此比较解压后的规范化内容。

本机验证记录：

- 本地蓝图库 85 份文件完成类型化往返，布局、配置字节、游戏版本和图标保持一致。
- 原生格式套件 5 项通过：旧生成文件版本 1 且警告为真；其重编码文件与重新生成的 CMAM 均版本 5 且警告为假；游戏来源版本 5 和旧版版本 4 文件正常导入与再次导出。
- 新 CMAM 保持 422 岛屿、54,906 建筑、375 星际运输段。默认紧凑套件增加了 `compact-library-format` 用例。
- 默认紧凑套件七项全部通过；预热 300 模拟秒、测量 120 模拟秒，300 条形状通道与 180 条流体通道的最慢额定速率比例均为 1.0。

本次格式套件报告：`%TEMP%/shapez-lab/runs/1793829315759-20508-0/report.json`。源码检查来自本地安装程序集，反编译文本位于 `%TEMP%/shapez-lab/blueprint-source`，不提交游戏代码或程序集。

本次默认套件报告：`%TEMP%/shapez-lab/runs/1793829347634-27100-0/report.json`；重新生成的蓝图位于 `runs/1793829244511-21048-0`；既有交付文件的重编码副本位于 `runs/1793829253094-19844-0`。
