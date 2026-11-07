# Quarter-cut — implementation in progress

Target: horizontal Foundation_1x2, top-left 12-belt input CuCuCuCu, four 12-belt outputs (left, right, bottom-left, bottom-right), all Cu------. Three floors, four input lanes per floor. Preserve all four quadrants using two successive full cuts and rotations.

Currently implemented: task-owned Rust entry point and C# runtime definition probe. Run `cargo run -p quarter-cut`. This creates an empty foundation only for probing; it is NOT the requested factory blueprint. No latest pointer is published.

The probe uses resolved GameBindings.Mode.Buildings definitions and native blueprint placement in an isolated host. Native occupancy checks remain enabled. It excludes virtual buildings and serializes bounded primitive data, avoiding recursive serialization of native configuration objects.

Verified against installed 1.2.0-rc3:

- CutterDefaultInternalVariant occupies (0,0,0), (0,-1,0).
- Mirrored full cutter occupies (0,0,0), (0,1,0).
- Both full cutters report belt speed 1 units/second and processing delay 1.5 seconds.
- Half cutter occupies one tile and reports delay 1 second.
- RotatorHalfInternalVariant, RotatorOneQuadCCWInternalVariant, RotatorOneQuadInternalVariant occupy one tile each.

Successful discovery report: `%TEMP%/shapez-lab/runs/1794051900937-27708-1/report.json`; resolved definitions are beside it in `quarter-cut-definitions.json`. Rust and C# compile successfully; the native definitions case passes.

Remaining implementation: extract directed input/output connectors and cutter simulation timing, generate and route the four independent processing cells per floor, orient the two-chunk foundation correctly, add all 48 native output meters and 12 full-rate sources, assert Cu------ and full belt utilization, and publish the final importable blueprint only after those checks pass. The current probe does not perform throughput verification.
