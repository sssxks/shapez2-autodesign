# shapez-solver

Task-independent integer-linear model ownership and native solver adapters.
Variables are indexed handles; constraints are normalized before submission.
Every returned feasible integer solution is checked against the owned model.

Backends (always enabled):

- `cp-sat`: `cpsat-rs` 0.1.2, OR-Tools 9.15 official serialized C API.
- `scip`: `russcip` 0.11 with its bundled native SCIP distribution.

Set `ORTOOLS_PREFIX` to an OR-Tools 9.15 installation and `PROTOC` to its compiler. On Windows, add the installation's `bin` to `PATH` for native DLL loading.

```powershell
cargo test -p shapez-solver
```