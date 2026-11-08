# CMAM 接入与压缩

本 task 只有 `cmam-compact` 一个 binary。构建结果、路由、供料接口和生产参数均使用 Rust 结构体；可视化及原生验收直接消费内存结果。

```powershell
cargo run --release -p cmam-compact -- build <source.spz2bp> --rgb <rgb.spz2bp>
cargo run --release -p cmam-compact -- build <source.spz2bp> --rgb <rgb.spz2bp> --native
cargo run --release -p cmam-compact -- build <source.spz2bp> --rgb <rgb.spz2bp> --native --target 'WrWr----:CuCu----'
```

`--previous <blueprint>` 添加旧版对照；`--target` 可以重复指定多个生产目标，要求同时使用 `--native`。

`build` 导出最终蓝图、HTML 和诊断报告。`--native` 使用当前构建结果检查导入、图库格式和所有供料通道的满速输送；`--target` 继续执行 600 模拟秒的生产验收，并要求至少产出一件目标形状。供料和生产配置直接放进管道请求，不再读取 `manifest.json`。报告只作为输出保存。

其他子命令：

- `legacy <source> --splitter <blueprint> --rgb <blueprint>`：旧接入方案。
- `inspect <blueprint>`：输出类型化蓝图摘要。
- `reencode <blueprint>`：按当前游戏格式重编码。
- `prepare`：准备隔离宿主。
- `probe --operation capacities`：读取原生能力。
- `test <suite.toml>`：执行人工编写的 TOML 验收配置。普通 `build --native` 无需配置文件。

旧方案验收示例为 `tests/legacy.toml`。原来的 compact/production JSON 套件已并入 `build --native`，不再依赖前一次运行生成的 manifest。`decode` 全量展开蓝图的命令已移除，检查蓝图使用 `inspect`。

构建产物保存在 `%TEMP%/shapez-lab/runs`；只有流程成功完成后才发布该 task 的 latest 记录。原生运行保持隔离宿主和串行 staging lease。