# 跨单元建筑级合成

`quarter-weave` 使用 Rust 建模，通过已有 `cpsat-rs` 绑定调用原生 CP-SAT。求解变量直接选择原生建筑的位置、朝向、镜像和逐接口整数流量。物料没有所属切割单元，因此不同第一刀产生的半片、四分之一片可以合流、共享旋转器，再重新分流。

## 约束

- 流量单位为 0.5 个/秒：完整切割器容量 1，旋转器容量 2，带子容量 4。
- 形状用四象限位图表示。完整形状 `15`，半片 `3/6/9/12`，四分之一片 `1/2/4/8`；最终目标为 `1`。
- 每个相邻格的有向接口、每种形状分别建立连接数量与流量相等约束。选中的每个接口流量至少为 1，所以合并器的全部输入必须使用。
- 切割器每个输入和输出都是 1；旋转器改变形状；合流、分流保持形状。统一的物料守恒按形状所含象限数加权。
- 原生建筑的全部占格参与互斥，包括切割器第二格、升降机中间层。跳跃带只占发送和接收两格，同时禁止相互误配的收发器组合。
- 原生建筑可以直接相邻，不要求中间额外插入带格。
- 目标首先最少占格，其次最少建筑。`--rotators` 单独限制旋转器数量。

运输支持普通直带、左右转带、升降带和可选跳跃带。生产支持完整切割器及镜像、三种旋转器、两输入/三输入合并器、两输出分流器。当前实验求解两条满速输出；`--whole` 将第一刀到最终合流全部交给求解器，默认模式只重排两组第二刀之后的处理区。

## 已实现的跨单元共享

两组第二刀产生的四条待旋转流，各为 0.5 个/秒。求解器把它们跨组配对，先合流再旋转，使用 **2 台满负荷旋转器** 代替 4 台半负荷旋转器。该处理区有 22 台建筑、25 个占格，使用两层；旧的两个独立处理区合计占 16 格。因此它是已运行的共享结构，密度优化继续由包含切割阶段的完整模型承担。

已保存的解为 [shared-quarters-witness.json](shared-quarters-witness.json)。接回两组真实切割机后，在隔离原生宿主中验证：

- `CuCuCuCu` → 两条 `Cu------`，每条 120 秒 240 个。
- `RrRrRrRr` → 两条 `Rr------`，每条 120 秒 240 个。
- 堵塞后恢复满带。
- 6 台合并器的全部接口均连接，空接接口为 0。

原生报告：`%TEMP%/shapez-lab/runs/1794073599917-29032-1/report.json`。
实验蓝图：`%TEMP%/shapez-lab/runs/1794073599915-29032-0/weave.spz2bp`。这是两输出实验组件，尚未发布为 1×2 整机。

完整建筑模型也独立求出了 5×8 单层结构：6 台切割器、8 台旋转器，共 26 台建筑、32 个占格。它同样通过两种形状与堵塞恢复的原生测试。已保存 [whole-weave-witness.json](whole-weave-witness.json)，可用作后续搜索提示。

扩展到九种形状后的完整模型复验报告：`%TEMP%/shapez-lab/runs/1794074112695-20984-1/report.json`。

## 运行

```powershell
. tasks/quarter-cut/search/environment.ps1
cargo run -p quarter-cut --features search --bin quarter-weave -- `
  "$env:TEMP/shapez-lab/shared-quarters" `
  --rotators 2 --seconds 30 --native

cargo run -p quarter-cut --features search --bin quarter-weave -- `
  "$env:TEMP/shapez-lab/whole-weave" `
  --whole --width 5 --height 8 --layers 1 --rotators 8 `
  --hint tasks/quarter-cut/search/whole-weave-witness.json `
  --seconds 30 --native
```

`--free-ports` 允许两条输入在左边界、两条输出在右边界自由选位；`--jumps` 启用原生跳跃带；`--layers` 可选 1–3。默认接口位置固定。`--native` 在有可行解时自动生成实验蓝图并执行原生验收。

输出 `<prefix>.status.json` 保存范围、状态和占格数；可行解另写 `<prefix>.json` 保存每台建筑及其接口流量，`<prefix>.ports.json` 保存选中的边界接口。跳跃带使用 `companion` 记录配对接收器。搜索提示只是提示，不固定布局。

## 当前搜索范围与结果

固定左右接口、5×7 单层区域，即使释放切割位置并加入跳跃带和两种半片切法，仍为 `INFEASIBLE`。释放边界接口后的同一区域，60 秒搜索为 `UNKNOWN`。4×8 两层、限制 6 台旋转器的早期六形状版本，50 秒搜索为 `UNKNOWN`。这些实例和 1×2 整机是不同的约束问题；后续 [分区联合模型](BANKS.md) 已完成 1×2 整机及 48 路原生验收。
