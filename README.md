# Shapez 2: 36-lane 1x3 transpose

One Foundation_1x3 platform, 1,053 buildings. North inputs left to right: R, G, B, 12 belts each. South: three outputs, each with R on level 3, G on level 2, B on level 1. The four lanes in each group retain their order.

The source level chooses the output: input level 3 -> left output, level 2 -> middle output, level 1 -> right output. The source port chooses output level: R -> 3, G -> 2, B -> 1.

`uv run python solve.py` uses negotiated congestion routing with actual lift column occupancy and sender/catcher jumps of 2-5 tiles. `uv run python export.py` writes the blueprint and checks actual building definitions, occupied cells, ordinary connectors, and farthest-sender jump matching. `uv run python draw.py` creates an interactive three-floor wiring drawing.

Verified: 36 independent paths, zero occupied-cell overlap, every ordinary output connected to its intended lane, all 47 jump pairs, and native game import with all 1,053 buildings retained. No live item-throughput measurement was performed. The existing factory was not modified.

The platform envelope and serialization structure came from the user's existing 1x3 belt-swapper blueprint. All routing was newly solved. The blueprint is saved using the existing library's SHAPEZ2-1 format, accepted by the running 1.2.0-rc3 game.

## Fluid version

Run uv run python fluid.py to convert the solved routes using actual fluid connector geometry. Downward connections are represented by bidirectional up-pipe variants with the correct lower anchor and rotation. Verified 36 isolated fluid networks, all source/destination pairs, 47 jump pairs, and zero overlapping occupied cells or open pipe connectors. In-game validation is handed to the user; native fluid import and live throughput are not claimed. Output: fluid-transpose-1x3.spz2bp.

