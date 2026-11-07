# shapez-viz

Rust library for self-contained, interactive factory grid views. It has no game, solver, native SDK, server, or JavaScript package-manager dependency.

Tasks adapt their own source data into a typed `Scene`: occupied `Tile`s, building owners, `Category` legend entries, directed material `Connection`s, rectangular `Region`s and boundary `Port`s. A multi-tile building has one owner and multiple tiles. Connections carry shape labels, rates, jump flags and optional highlighting.

`shapez_viz::render_html(&scene)` returns one offline HTML document. The viewer supports layer selection (or all layers), region boundaries, material-flow overlays, category filtering, zoom, keyboard tile selection, and building/connection inspection. All data and scripts are embedded; it makes no network requests. JSON labels are escaped against script-element termination and displayed using DOM text nodes.

The renderer owns visualization only. Game-specific footprints, rotation semantics, solver-token decoding, and artifact generation remain in tasks. The complete adapter example is `tasks/quarter-cut/src/visualization.rs`; its `scene` function combines the actual assembled blueprint and versioned flow witness. Each active flow tile and destination must resolve to an actual occupied building tile.

The caller supplies coordinates with x increasing right, y increasing down, and zero-based layers. Region bounds are half-open. Ports may occupy the perimeter outside the active floor. Rendering adds a two-tile margin for labels. The HTML uses host theme variables, with fallback colors for direct offline viewing.

```rust,ignore
let scene: shapez_viz::Scene = task_scene(&blueprint)?;
let html = shapez_viz::render_html(&scene)?;
std::fs::write("factory.html", html)?;
```
