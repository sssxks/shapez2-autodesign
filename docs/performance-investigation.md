# Live CPU performance investigation — 2026-11-05

Game PID: 28180, installed Steam shapez 2. Foreground capture at the existing dense factory view. No game settings, save, or installed assemblies changed.

## Capture

PresentMon 2.5.1, 15 seconds, 572 frames. Raw capture: `%TEMP%/shapez-lab/performance/live.csv`.

- Average present interval: 25.988 ms (38.48 FPS); median 25.915 ms; p95 27.998 ms.
- PresentMon CPU busy interval: average 25.774 ms. This is frame-side elapsed time, including internal synchronization, not aggregated CPU execution time.
- Present API wait: average 0.215 ms.
- GPU busy: average 15.709 ms; p95 16.722 ms.
- Earlier background process sampling: 2.37 core equivalents, 5.48 GiB working set. Do not mix background CPU utilization with foreground frame timing.
- Configured simulation: Twelve threads, prediction enabled, Low simulation detail. Display: 3840x2160.

## Current installed architecture

Inspected the installed `Game.Core.Map.Simulation.dll` with the existing ilspycmd tool.

`SimulationGraph.Update(IReadOnlyList<SimulationLOD>)` copies the cluster set into a scoped hash set, processes visible chunk sets, then traverses remaining clusters. It queues work into concurrent bags. For each update, it starts `NumThreads` tasks using `Task.Factory.StartNew(UpdateClusters)` and calls `Task.WaitAll`.

`UpdateClusters()` consumes whole clusters. `SimulationCluster.Update()` traverses ordered simulation nodes sequentially. Parallelism is between clusters; increasing the configured thread count cannot divide one large cluster.

`AsynchronousUpdate()` already dispatches the graph update through `Task.Run`. A proposed asynchronous redesign must therefore address ownership and the consumer's synchronization boundary, not simply put the existing update in another Task.

## Architectural direction

1. Maintain topology and chunk membership indexes on edits. Replace repeated set construction with reusable traversal marks and contiguous work lists. Preserve LOD priority and optional-update time budgets.
2. Batch cluster work into a persistent worker scheduler, with work stealing and cost-aware ordering. Preserve cluster-local ordering and update completion semantics. Measure actual scheduler time before selecting this as the implementation target.
3. If a few large clusters dominate, establish explicit transfer boundaries and deterministic handover phases before partitioning. Naive parallel node updates change item acceptance and machine ordering.
4. If presentation waits on mutable simulation state, publish compact immutable render snapshots and interpolate from the last completed state. The current asynchronous task alone does not provide that ownership boundary.

## Implementation status and next decisive capture

The foreground frame capture establishes CPU-side frame pacing as the first target. No optimization patch has been applied. Next capture needs CPU stacks or in-game timings for `PrepareClustersByLOD`, `PrepareRemainingClusters`, `UpdateClusters`, render preparation and simulation completion waits, together with cluster size/cost distribution. Choose the dominant stage and implement that architectural change; do not optimize every listed candidate.

The old FactoryProbe source provides a command-loading bridge, but its status file is dated November 4 and the current Player.log does not report that probe. A live bridge is not established for this process. Use an external CPU trace or an explicitly loaded performance mod for the next capture.
