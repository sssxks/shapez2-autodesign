# shapez-solver

Task-independent integer-linear model ownership and native solver adapters.
Variables are indexed handles; constraints are normalized before submission.
Every returned feasible integer solution is checked against the owned model.

Optional backends:

- `cp-sat`: `cpsat-rs` 0.1.2, OR-Tools 9.15 official serialized C API.
- `scip`: `russcip` 0.11 with its bundled native SCIP distribution.

No native dependency is enabled by default. On Windows, the quarter-cut task's
`search/environment.ps1` sets up the pinned official OR-Tools binary package,
including `protoc`, and sets this shell's DLL search path. Other platforms can
set `ORTOOLS_PREFIX` and `PROTOC` to their native installations.

```powershell
. tasks/quarter-cut/search/environment.ps1
cargo test -p shapez-solver --features cp-sat,scip
```
