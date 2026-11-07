# Reproducing the 1×2 quarter cutter

There are three distinct operations: deterministic reconstruction of the delivered design, native game acceptance, and searching for a new design.

## Deterministic reconstruction — Rust only

From the repository root:

```powershell
cargo run --locked --release -p quarter-cut
```

Or change to `tasks/quarter-cut` and run `cargo run --locked --release`.

This rebuilds both native-verified interface variants. The original north-input factory uses `search/original/left.buildings.json` and `right.buildings.json`: two 17×14 regions, each copied across three layers. The earlier west-input factory uses `search/bank-left.buildings.json` and `search/bank-right.buildings.json`, with reflection into four 17×7 banks. Both material-flow views use their corresponding committed `*.witness.json` files.

Outputs, relative to the working directory:

- `artifacts/quarter-cut/quarter-cut-1x2-original.spz2bp` — the user's original north-input interfaces
- `artifacts/quarter-cut/quarter-cut-1x2-original.html`
- `artifacts/quarter-cut/quarter-cut-1x2.spz2bp`
- `artifacts/quarter-cut/quarter-cut-1x2.html`

An explicit destination is supported: `cargo run --release -p quarter-cut -- --output path/to/output`.

All eight design inputs are tracked and embedded at compile time. No `%TEMP%/shapez-lab` design data, game installation, .NET SDK, solver SDK, Python, or prepared native host is read by this operation. Cargo uses its normal target directory; `CARGO_TARGET_DIR` is optional. Rust and its platform linker plus Cargo dependencies are needed for compilation.

`cargo test -p quarter-cut --lib` compares both reconstructed building lists and icons against their native-verified templates and constructs both flow scenes. The original-interface blueprint SHA-256 is `babaf25a81fe1df3b1ccf9d485535f49e9feeaa5520a3509bb18df93620ea6d9`. The earlier west-input blueprint SHA-256 remains `d029d294a1136ec591f4e91899e78eadd5d6a057485a1f15ab12e9137dbc0085`.

## Native game acceptance — Windows and a local game

```powershell
./scripts/bootstrap.ps1
cargo run -p cmam-compact -- prepare
cargo run --release -p quarter-cut -- --native
```

This requires a local, compatible Shapez 2 installation (the delivered design was verified on 1.2.0-rc3) and .NET SDK 10. `bootstrap.ps1` installs the pinned SDK in the lab tools directory. `prepare` creates and patches an isolated game copy. Default game path: `C:/Program Files (x86)/Steam/steamapps/common/shapez 2`. Override with `SHAPEZ_GAME_ROOT`; override the SDK executable with `SHAPEZ_LAB_DOTNET`.

`--native` now selects the original north-input template. Pass the earlier west-input template's path explicitly to test that variant; its physical input port selects the corresponding harness.

Native binaries and game assets are not part of Git. Acceptance feeds twelve full belts, measures all forty-eight outputs for two shape types, tests blocked-output recovery, validates every merger socket and the library icon, then publishes `latest.json`. Native tests run serially because the host staging directory has an exclusive lease.

To rebuild arbitrary solved bank files and immediately run native acceptance, use `quarter-assemble` with two file paths. The default portable reconstruction and this native executable share the same assembly implementation.

## Searching again — native solver SDK

See [original interface model](original/README.md) or [earlier west-input banks](BANKS.md). Dot-source `environment.ps1` to download/configure the pinned official OR-Tools Windows SDK. The search feature uses existing Rust bindings to CP-SAT and SCIP and needs their native build toolchain. Saved witnesses are warm-start hints; omit `--hint` for a cold solve. Time-limited, parallel solving can return different layouts or hit the time limit; it is not the deterministic reconstruction command above.

The west-input variant uses four 17×7 regions with fixed cutting cores. The original north-input variant uses two 17×14 regions; left-side core origin ranges and directions are in `original/left-core-domains.json`, and CP-SAT selects their positions and mirrors jointly with routing. These regional boundaries are design inputs rather than outputs of a global partition optimizer.

## Verified fresh-clone run

On 2026-11-08, commit `facc273` was cloned with `git clone --no-local` into a new directory. The build used the clone's new `target` directory, with `CARGO_TARGET_DIR`, `ORTOOLS_PREFIX`, and `PROTOC` unset. Game and .NET paths were deliberately set to nonexistent locations. Running `cargo run --locked --release` inside `tasks/quarter-cut` produced the blueprint and interactive HTML successfully with Rust 1.98.1 on Windows.

The generated blueprint was byte-identical to the native-verified template (SHA-256 above). Cargo's normal dependency cache was available. The [machine-readable result](../results/fresh-clone-reconstruction.json) records the checkout, command, paths and hash. This checks reconstruction from tracked solutions; cold solving and native acceptance have the separate prerequisites described above.
