# Reproducing the 1×2 quarter cutter

There are three distinct operations: deterministic reconstruction of the delivered design, native game acceptance, and searching for a new design.

## Deterministic reconstruction — Rust only

From the repository root:

```powershell
cargo run --locked --release -p quarter-cut
```

Or change to `tasks/quarter-cut` and run `cargo run --locked --release`.

This builds the factory again from `search/bank-left.buildings.json` and `search/bank-right.buildings.json`, reflects and assembles the banks, adds the physical interfaces, and encodes a fresh blueprint. It also reconstructs the three-layer material-flow view using `search/bank-left.witness.json` and `search/bank-right.witness.json`.

Outputs, relative to the working directory:

- `artifacts/quarter-cut/quarter-cut-1x2.spz2bp`
- `artifacts/quarter-cut/quarter-cut-1x2.html`

An explicit destination is supported: `cargo run --release -p quarter-cut -- --output path/to/output`.

All four design inputs are tracked and embedded at compile time. No `%TEMP%/shapez-lab` design data, game installation, .NET SDK, solver SDK, Python, or prepared native host is read by this operation. Cargo uses its normal target directory; `CARGO_TARGET_DIR` is optional. Rust and its platform linker plus Cargo dependencies are needed for compilation.

`cargo test -p quarter-cut --lib` compares all assembled buildings and the icon against the already native-verified template. The expected blueprint SHA-256 is `d029d294a1136ec591f4e91899e78eadd5d6a057485a1f15ab12e9137dbc0085`.

## Native game acceptance — Windows and a local game

```powershell
./scripts/bootstrap.ps1
cargo run -p cmam-compact -- prepare
cargo run --release -p quarter-cut -- --native
```

This requires a local, compatible Shapez 2 installation (the delivered design was verified on 1.2.0-rc3) and .NET SDK 10. `bootstrap.ps1` installs the pinned SDK in the lab tools directory. `prepare` creates and patches an isolated game copy. Default game path: `C:/Program Files (x86)/Steam/steamapps/common/shapez 2`. Override with `SHAPEZ_GAME_ROOT`; override the SDK executable with `SHAPEZ_LAB_DOTNET`.

Native binaries and game assets are not part of Git. Acceptance feeds twelve full belts, measures all forty-eight outputs for two shape types, tests blocked-output recovery, validates every merger socket and the library icon, then publishes `latest.json`. Native tests run serially because the host staging directory has an exclusive lease.

To rebuild arbitrary solved bank files and immediately run native acceptance, use `quarter-assemble` with two file paths. The default portable reconstruction and this native executable share the same assembly implementation.

## Searching again — native solver SDK

See [BANKS.md](BANKS.md). Dot-source `environment.ps1` to download/configure the pinned official OR-Tools Windows SDK. The search feature uses existing Rust bindings to CP-SAT and SCIP and needs their native build toolchain. Saved witnesses are warm-start hints; omit `--hint` for a cold solve. Time-limited, parallel solving can return different layouts or hit the time limit; it is not the deterministic reconstruction command above.

The four 17×7 regions and fixed cutting cores are explicit design inputs in the Rust formulation. The original bank dimensions came from geometry and successful regional trials, not from an outer global partition optimizer.

## Verified fresh-clone run

On 2026-11-08, commit `facc273` was cloned with `git clone --no-local` into a new directory. The build used the clone's new `target` directory, with `CARGO_TARGET_DIR`, `ORTOOLS_PREFIX`, and `PROTOC` unset. Game and .NET paths were deliberately set to nonexistent locations. Running `cargo run --locked --release` inside `tasks/quarter-cut` produced the blueprint and interactive HTML successfully with Rust 1.98.1 on Windows.

The generated blueprint was byte-identical to the native-verified template (SHA-256 above). Cargo's normal dependency cache was available. The [machine-readable result](../results/fresh-clone-reconstruction.json) records the checkout, command, paths and hash. This checks reconstruction from tracked solutions; cold solving and native acceptance have the separate prerequisites described above.