//! Finite-domain production and routing on one complete platform layer.
use anyhow::{Result, ensure};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use shapez_blueprint::{Building, Rotation};
use shapez_solver::{Expr, FeasibleSolution, Limits, Model, Solution, Var};
use std::{collections::BTreeMap, fs, path::PathBuf};
/// Local tile coordinates (x, y, layer), with zero-based layers and x east/y south.
type Point = (i32, i32, i16);
/// Direction offsets indexed east, south, west, north, up, down (0 through 5).
const D: [Point; 6] = [
    (1, 0, 0),
    (0, 1, 0),
    (-1, 0, 0),
    (0, -1, 0),
    (0, 0, 1),
    (0, 0, -1),
];
/// Return the reverse of a valid D direction index (0 through 5).
fn opposite(d: usize) -> usize {
    if d < 4 { (d + 2) % 4 } else { 9 - d }
}
/// Allowed quadrant occupancy masks: single quarters, adjacent halves, and full shapes.
const SHAPES: [i64; 9] = [1, 2, 4, 8, 3, 6, 9, 12, 15];
/// Two vertical regions matching the original north-input interface layout.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OriginalHalf {
    /// Western 17 by 14 region receiving the north inputs and forwarding two full-shape lanes.
    Left,
    /// Eastern 17 by 14 region receiving the two forwarded full-shape lanes.
    Right,
}
/// CLI options for synthesizing and exporting a regional or full-platform grid.
#[derive(Clone, Parser)]
pub struct Args {
    /// Destination blueprint path supplied as a positional CLI argument.
    pub output: PathBuf,
    /// Solve the 5 by 4 tile unit with one input and one quarter output.
    #[arg(long)]
    cell: bool,
    /// Solve one 17x14 half of the original five-port layout.
    #[arg(long, conflicts_with_all = ["cell", "bank"])]
    original_half: Option<OriginalHalf>,
    /// The two complete-shape lanes crossing between the original-layout regions.
    #[arg(long, num_args = 2, default_values_t = [5,8])]
    through_rows: Vec<i32>,
    /// Point the upper cutting cores inward to receive the original northern input.
    #[arg(long, requires = "original_half")]
    inward_top: bool,
    /// Reflect fixed core geometry horizontally while preserving shape operations.
    #[arg(long, requires = "original_half")]
    mirror_cores: bool,
    /// Stagger four west-facing cores above four south-facing cores in the left region.
    #[arg(long, requires_all = ["original_half", "cores"], conflicts_with_all = ["mirror_cores", "inward_top"])]
    corner_cores: bool,
    /// Jointly choose a small set of cutting-core poses and all remaining transport.
    #[arg(long, requires = "original_half", conflicts_with = "cores")]
    flex_cores: bool,
    /// Pack anonymous cutting cores anywhere; their layer counts may differ.
    #[arg(long, requires = "original_half", conflicts_with_all = ["cores", "flex_cores"])]
    free_core_pack: bool,
    /// Pack cores in the left half of a full platform while allowing transport anywhere.
    #[arg(long, conflicts_with_all = ["original_half", "cores", "flex_cores", "free_core_pack"])]
    pack_left: bool,
    /// Synthesize a four-output bank with one full input per layer.
    #[arg(long, conflicts_with = "cell")]
    bank: bool,
    /// Bank width in tiles; at least 4, or 16 with fixed cores.
    #[arg(long, default_value_t = 17)]
    bank_width: i32,
    /// Bank height in tiles; at least 3, or 6 with fixed cores.
    #[arg(long, default_value_t = 6)]
    bank_height: i32,
    /// Zero-based x coordinate of the first of four consecutive north-edge outputs.
    #[arg(long, default_value_t = 5)]
    bank_output: i32,
    /// Carry an additional full input through the bank to the east edge.
    #[arg(long, requires = "bank")]
    bank_through: bool,
    /// West input y coordinate; defaults to height minus 1 and must differ from the through lane.
    #[arg(long, requires = "bank")]
    bank_input_row: Option<i32>,
    /// Fix only cutting/half-rotation cores; synthesize the entire shared quarter network.
    #[arg(long)]
    cores: bool,
    /// Number of modeled layers, restricted to 1 through 3; layer indices start at zero.
    #[arg(long, default_value_t = 1)]
    layers: i16,
    /// Flattened CP-SAT budget, logging, and objective controls.
    #[command(flatten)]
    search: SearchSettings,
    /// Flattened checkpoint, hint, and native-validation options.
    #[command(flatten)]
    execution: Execution,
}
/// Mutually exclusive physical domains accepted by the grid builder.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Domain {
    /// Complete 34 by 14 platform interior.
    Platform,
    /// Compact 5 by 4 production unit.
    Cell,
    /// One 17 by 14 half with original-interface boundary ports.
    Half(OriginalHalf),
    /// Configurable rectangular four-output bank.
    Bank,
}

/// In-memory region geometry and core placement rules, independent of CLI and files.
struct Region {
    /// Physical boundary layout determining dimensions and required ports.
    domain: Domain,
    /// Two local y coordinates for full-shape lanes from the left half into the right half.
    through_rows: [i32; 2],
    /// Orient the upper fixed cores toward the northern input lanes.
    inward_top: bool,
    /// Reflect fixed core positions horizontally across the region while preserving shape transformations.
    mirror_cores: bool,
    /// Use the staggered left-region arrangement of four west-facing and four south-facing cores.
    corner_cores: bool,
    /// Choose core poses from small per-core domains jointly with transport routing.
    flex_cores: bool,
    /// Choose anonymous nonoverlapping core poses without per-layer production quotas.
    free_core_pack: bool,
    /// Restrict packed cores to x below 17 while allowing transport throughout the platform.
    pack_left: bool,
    /// Bank width in tiles; at least 4, or 16 with fixed cores.
    bank_width: i32,
    /// Bank height in tiles; at least 3, or 6 with fixed cores.
    bank_height: i32,
    /// Zero-based x coordinate of the first of four consecutive north-edge outputs.
    bank_output: i32,
    /// Require a full-rate complete-shape lane across the bank at y = height minus 2.
    bank_through: bool,
    /// West input y coordinate; defaults to height minus 1 and must differ from the through lane.
    bank_input_row: Option<i32>,
    /// Restrict production to fixed cutting and rotation footprints while solving shared transport.
    cores: bool,
    /// Number of modeled layers, restricted to 1 through 3; layer indices start at zero.
    layers: i16,
}
impl Region {
    /// Standard geometry shared by CLI regions and the factory pipeline.
    fn new(domain: Domain) -> Self {
        Self {
            domain,
            through_rows: [5, 8],
            inward_top: false,
            mirror_cores: false,
            corner_cores: false,
            flex_cores: false,
            free_core_pack: false,
            pack_left: false,
            bank_width: 17,
            bank_height: 6,
            bank_output: 5,
            bank_through: false,
            bank_input_row: None,
            cores: false,
            layers: 1,
        }
    }
    /// Half identity exists only for original-interface regions.
    fn half(&self) -> Option<OriginalHalf> {
        match self.domain {
            Domain::Half(half) => Some(half),
            _ => None,
        }
    }
    /// Regional experiments use a surrounding test platform instead of factory sockets.
    fn regional(&self) -> bool {
        self.domain != Domain::Platform
    }
}

/// Optimization controls; shared by direct region solves and factory assembly.
#[derive(Clone, clap::Args)]
struct SearchSettings {
    /// CP-SAT time budget in seconds for each regional solve; defaults to 120.
    #[arg(long, default_value_t = 120.)]
    seconds: f64,
    /// Number of parallel CP-SAT search workers; defaults to 8.
    #[arg(long, default_value_t = 8)]
    workers: i32,
    /// Suppress CP-SAT progress logging; summary diagnostics remain enabled.
    #[arg(long)]
    quiet: bool,
    /// Stop at the first complete feasible layout instead of minimizing occupied tiles.
    #[arg(long)]
    feasibility: bool,
    /// Lower bound on served quarter outputs; zero leaves the default objective in control.
    #[arg(long, default_value_t = 0)]
    minimum: i64,
}

/// Optional CLI file operations and native inspection around an in-memory solve.
#[derive(Clone, Default, clap::Args)]
struct Execution {
    /// Optional disjoint pose domains, one selected core per domain and layer.
    #[arg(long, requires = "free_core_pack")]
    core_domains: Option<PathBuf>,
    /// Request native game validation of the resulting blueprint.
    #[arg(long)]
    native: bool,
    /// Load a grid checkpoint with a matching building grammar as solver hints.
    #[arg(long)]
    resume: Option<PathBuf>,
    /// Save an explicit restart checkpoint.
    #[arg(long)]
    checkpoint: Option<PathBuf>,
    /// Embed a verified 5x4 unit at the west output bank as a feasible seed.
    #[arg(long)]
    unit_hint: Option<PathBuf>,
    /// Infer jump kinds from checkpoint port pairs instead of trusting saved kind IDs.
    #[arg(long)]
    infer_hint: bool,
    /// Four resume coordinates x0 y0 x1 y1; freeze tile kinds outside this half-open rectangle on every layer.
    #[arg(long, num_args = 4, requires = "resume")]
    window: Vec<i32>,
    /// Blueprint supplying building-kind hints; island buildings have the three-tile border offset removed.
    #[arg(long)]
    buildings_hint: Option<PathBuf>,
    /// Fix every hinted tile kind, including empty cells absent from the supplied blueprint.
    #[arg(long, requires = "buildings_hint")]
    lock_layout: bool,
    /// Fix supplied buildings while leaving all other tiles available.
    #[arg(long, requires = "buildings_hint")]
    fix_known_layout: bool,
}
/// Physical building pose or reserved footprint cell; its index is the solver kind ID.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Kind {
    /// Native building definition; absent for empty tiles and auxiliary footprint cells.
    definition: Option<String>,
    /// Physical planar direction index: east 0, south 1, west 2, north 3.
    rotation: usize,
    /// Marks a reserved cutter or lift footprint cell that exports no separate building.
    auxiliary: bool,
    /// Optional jump code: direction * 4 + distance - 2 for senders, plus 16 for receivers; distance is 2 through 5 tiles.
    jump: Option<usize>,
    /// Distinguishes lift footprints from cutter footprints, including auxiliary cells.
    #[serde(default)]
    lift: bool,
}
/// Inclusive origin bounds and physical direction for one interchangeable pose group.
#[derive(Deserialize)]
struct CoreDomain {
    /// Inclusive local origin bounds [xmin, ymin, xmax, ymax] in tiles.
    bounds: [i32; 4],
    /// Physical planar direction index: east 0, south 1, west 2, north 3.
    rotation: usize,
}
/// Legal building states, initially encoded as flow tokens and reduced to wire classes during grid construction.
struct Grammar {
    /// Building descriptors indexed by solver kind ID; index zero represents an empty tile.
    kinds: Vec<Kind>,
    /// Allowed rows: kind ID, six input ports, six output ports in D order; build converts flow tokens to wire classes.
    rows: Vec<Vec<i64>>,
}
/// Move n tile or layer units along direction d; d must index D and layer arithmetic must fit i16.
fn step(p: Point, d: usize, n: i32) -> Point {
    (p.0 + D[d].0 * n, p.1 + D[d].1 * n, p.2 + D[d].2 * n as i16)
}
/// Encode an ordinary flow as 4 * shape + q, where shape is a quadrant mask and q is 1 through 4 half-items per second.
fn token(shape: i64, q: i64) -> i64 {
    4 * shape + q
}
impl Grammar {
    /// Append a physical pose and return its stable index; infer main-lift status from the native definition.
    fn kind(
        &mut self,
        definition: Option<&str>,
        rotation: usize,
        auxiliary: bool,
        jump: Option<usize>,
    ) -> usize {
        let id = self.kinds.len();
        self.kinds.push(Kind {
            definition: definition.map(str::to_owned),
            rotation,
            auxiliary,
            jump,
            lift: definition.is_some_and(|d| d.starts_with("Lift")),
        });
        id
    }
    /// Append a 13-column state with zero for unspecified ports; port indices must be in 0 through 5.
    fn row(&mut self, kind: usize, inputs: &[(usize, i64)], outputs: &[(usize, i64)]) {
        let mut row = vec![0; 13];
        row[0] = kind as i64;
        for &(d, t) in inputs {
            row[1 + d] = t;
        }
        for &(d, t) in outputs {
            row[7 + d] = t;
        }
        self.rows.push(row);
    }
    /// Construct legal production and transport states, including cutter transfers, paired jumps, and one/two-layer lifts.
    fn build() -> Self {
        let mut g = Self {
            kinds: Vec::new(),
            rows: Vec::new(),
        };
        g.kind(None, 0, false, None);
        g.row(0, &[], &[]);
        for r in 0..4 {
            let dir = |d| (d + r) % 4;
            for (name, out) in [
                ("BeltDefaultForwardInternalVariant", 0),
                ("BeltDefaultLeftInternalVariant", 3),
                ("BeltDefaultLeftInternalVariantMirrored", 1),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for shape in SHAPES {
                    for q in 1..=4 {
                        g.row(
                            k,
                            &[(dir(2), token(shape, q))],
                            &[(dir(out), token(shape, q))],
                        );
                    }
                }
            }
            for (name, a, b) in [
                ("RotatorOneQuadInternalVariant", 12, 9),
                ("RotatorOneQuadCCWInternalVariant", 3, 9),
                ("RotatorOneQuadInternalVariant", 3, 6),
                ("RotatorOneQuadCCWInternalVariant", 12, 6),
                ("RotatorOneQuadInternalVariant", 8, 1),
                ("RotatorOneQuadCCWInternalVariant", 2, 1),
                ("RotatorHalfInternalVariant", 4, 1),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for q in 1..=2 {
                    g.row(k, &[(dir(2), token(a, q))], &[(dir(0), token(b, q))]);
                }
            }
            for (name, sides) in [
                ("Merger2To1LInternalVariant", &[2, 3][..]),
                ("Merger2To1LInternalVariantMirrored", &[2, 1][..]),
                ("Merger3To1InternalVariant", &[2, 3, 1][..]),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for shape in SHAPES {
                    for a in 1..=3 {
                        for b in 1..=3 {
                            for c in if sides.len() == 3 { 1..=2 } else { 0..=0 } {
                                let q = a + b + c;
                                if q > 4 {
                                    continue;
                                }
                                let rates = [a, b, c];
                                let input = sides
                                    .iter()
                                    .enumerate()
                                    .map(|(i, &d)| (dir(d), token(shape, rates[i])))
                                    .collect::<Vec<_>>();
                                g.row(k, &input, &[(dir(0), token(shape, q))]);
                            }
                        }
                    }
                }
            }
            for (name, side) in [
                ("Splitter1To2LInternalVariant", 3),
                ("Splitter1To2LInternalVariantMirrored", 1),
                ("SplitterTShapeInternalVariant", 4),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for shape in SHAPES {
                    for a in 1..=3 {
                        for b in 1..=4 - a {
                            let ds = if side == 4 { [3, 1] } else { [0, side] };
                            g.row(
                                k,
                                &[(dir(2), token(shape, a + b))],
                                &[(dir(ds[0]), token(shape, a)), (dir(ds[1]), token(shape, b))],
                            );
                        }
                    }
                }
            }
            for (mirror, (name, side)) in [
                ("CutterDefaultInternalVariant", 3),
                ("CutterDefaultInternalVariantMirrored", 1),
            ]
            .into_iter()
            .enumerate()
            {
                for (recipe, (a, l, b)) in
                    [(15, 12, 3), (9, 8, 1), (6, 4, 2)].into_iter().enumerate()
                {
                    let transfer = 80 + ((r * 2 + mirror) * 3 + recipe) as i64;
                    let main = g.kind(Some(name), r, false, None);
                    g.row(
                        main,
                        &[(dir(2), token(a, 1))],
                        &[(dir(0), token(b, 1)), (dir(side), transfer)],
                    );
                    let aux = g.kind(None, r, true, None);
                    g.row(
                        aux,
                        &[((dir(side) + 2) % 4, transfer)],
                        &[(dir(0), token(l, 1))],
                    );
                }
            }
            for n in 2..=5 {
                let j = r * 4 + n - 2;
                let sender = g.kind(Some("BeltPortSenderInternalVariant"), r, false, Some(j));
                let receiver = g.kind(
                    Some("BeltPortReceiverInternalVariant"),
                    r,
                    false,
                    Some(16 + j),
                );
                for shape in SHAPES {
                    for q in 1..=4 {
                        g.row(sender, &[(dir(2), token(shape, q))], &[]);
                        g.row(receiver, &[], &[(dir(0), token(shape, q))]);
                    }
                }
            }
            for (turn, (suffix, out)) in [
                ("Forward", 0),
                ("Left", 3),
                ("LeftMirrored", 1),
                ("Backward", 2),
            ]
            .into_iter()
            .enumerate()
            {
                for (up, axis) in [(true, 4), (false, 5)] {
                    let code = r * 8 + turn * 2 + usize::from(up);
                    for length in 1..=2 {
                        let name = format!(
                            "Lift{length}{}{}InternalVariant{}",
                            if up { "Up" } else { "Down" },
                            suffix.trim_end_matches("Mirrored"),
                            if suffix.ends_with("Mirrored") {
                                "Mirrored"
                            } else {
                                ""
                            }
                        );
                        let main = g.kind(Some(&name), r, false, None);
                        let end = g.kind(None, r, true, None);
                        g.kinds[end].lift = true;
                        let middle = (length == 2).then(|| {
                            let k = g.kind(None, r, true, None);
                            g.kinds[k].lift = true;
                            k
                        });
                        for shape in SHAPES {
                            for q in 1..=4 {
                                let a = 1000
                                    + ((if length == 1 { 10 } else { 42 }) + code as i64) * 128
                                    + token(shape, q);
                                let b = 1000 + (74 + code as i64) * 128 + token(shape, q);
                                g.row(main, &[(dir(2), token(shape, q))], &[(axis, a)]);
                                g.row(
                                    end,
                                    &[(opposite(axis), if length == 1 { a } else { b })],
                                    &[(dir(out), token(shape, q))],
                                );
                                if let Some(m) = middle {
                                    g.row(m, &[(opposite(axis), a)], &[(axis, b)]);
                                }
                            }
                        }
                    }
                }
            }
        }
        g
    }
}
/// Directed connection split into wire class, quadrant mask, half-item-per-second rate, and class predicates.
#[derive(Clone, Copy)]
struct Edge {
    /// Wire class: 0 absent, 1 ordinary, 2 through 9 cutter transfer, 10 and above lift transfer.
    wire: Var,
    /// Quadrant occupancy mask, or zero for an inactive connection.
    shape: Var,
    /// Flow in units of 0.5 items per second, from zero through four.
    rate: Var,
    /// Boolean equivalent to a nonzero wire class.
    active: Var,
    /// Boolean selecting ordinary belt flow (wire class 1).
    normal: Var,
    /// Boolean selecting cutter or lift transfers (wire class at least 2).
    internal: Var,
}
/// Solver variables for one occupied or empty tile, including reserved multi-tile footprints.
#[derive(Clone)]
struct Tile {
    /// Index into the associated grammar kind table; zero denotes an empty tile.
    kind: Var,
    /// Boolean identifying a main cutter cell, excluding its auxiliary footprint.
    cut: Var,
    /// Six inputs followed by six outputs, each ordered east, south, west, north, up, down.
    ports: [Edge; 12],
    /// One-hot jump predicates: 16 senders then 16 receivers, indexed by direction * 4 + distance - 2.
    jumps: [Var; 32],
    /// Quadrant masks for input, ordinary output, and internal-transfer output, respectively.
    shapes: [Var; 3],
}
/// Finite regional model with boundary flows and an objective favoring served outputs then fewer occupied tiles.
struct Grid {
    /// CP-SAT variables, constraints, hints, and occupied-tile/output objective.
    model: Model,
    /// Deduplicated building kinds and allowed wire-class tuples used by this model.
    grammar: Grammar,
    /// Tile variables keyed by local x, y, and zero-based layer coordinates.
    tiles: BTreeMap<Point, Tile>,
    /// Boolean flags for full-rate quarter outputs; mandatory full-shape through lanes are excluded.
    served: Vec<Var>,
    /// Boundary source triples: exterior tile position, inward direction index, and directed flow variables.
    sources: Vec<(Point, usize, Edge)>,
    /// Boundary target triples: interior tile position, outward direction index, and directed flow variables.
    targets: Vec<(Point, usize, Edge)>,
    /// Extent along x in tiles, with local coordinates 0 through width minus 1.
    width: i32,
    /// Extent along y in tiles, with local coordinates 0 through height minus 1.
    height: i32,
    /// Number of modeled layers, restricted to 1 through 3; layer indices start at zero.
    layers: i16,
}
/// Map native definitions to transform codes: 0 transport, 1 clockwise, 2 counterclockwise, 3 half-turn, 4 cutter.
fn operation(k: &Kind) -> i64 {
    match k.definition.as_deref() {
        Some("RotatorOneQuadInternalVariant") => 1,
        Some("RotatorOneQuadCCWInternalVariant") => 2,
        Some("RotatorHalfInternalVariant") => 3,
        Some(s) if s.starts_with("Cutter") => 4,
        _ => 0,
    }
}
/// Return allowed [operation, input, ordinary output, internal output] quadrant-mask tuples, including the empty state.
fn shape_relations() -> Vec<Vec<i64>> {
    let mut rows = vec![vec![0, 0, 0, 0]];
    rows.extend(SHAPES.into_iter().map(|s| vec![0, s, s, s]));
    for (op, a, b, c) in [
        (1, 12, 9, 9),
        (1, 3, 6, 6),
        (1, 8, 1, 1),
        (2, 3, 9, 9),
        (2, 12, 6, 6),
        (2, 2, 1, 1),
        (3, 4, 1, 1),
        (4, 15, 3, 12),
        (4, 9, 1, 8),
        (4, 6, 2, 4),
    ] {
        rows.push(vec![op, a, b, c]);
    }
    rows
}
/// Enforce a == b when Boolean on is set; limit must bound the absolute difference when on is unset.
fn equal_if(model: &mut Model, on: Var, a: Var, b: Var, limit: i64) {
    model.bounded(
        Expr {
            terms: vec![(a, 1), (b, -1), (on, limit)],
            constant: 0,
        },
        i64::MIN,
        limit,
    );
    model.bounded(
        Expr {
            terms: vec![(a, 1), (b, -1), (on, -limit)],
            constant: 0,
        },
        -limit,
        i64::MAX,
    );
}
/// Encode solved flow for checkpoints: zero absent, ordinary token below 80, cutter transfer from 80, lift transfer from 1000.
fn edge_value(e: Edge, s: &FeasibleSolution) -> i64 {
    let wire = s.value(e.wire);
    let shape = s.value(e.shape);
    let rate = s.value(e.rate);
    if wire == 0 {
        0
    } else if wire == 1 {
        token(shape, rate)
    } else if wire >= 10 {
        1000 + wire * 128 + token(shape, rate)
    } else {
        80 + (wire - 2) * 3
            + match shape {
                12 => 0,
                8 => 1,
                4 => 2,
                _ => unreachable!("Native cutter side shape"),
            }
    }
}
/// Choose each core's translation and vertical orientation together with its transport.
fn flexible_cores(
    model: &mut Model,
    grammar: &Grammar,
    tiles: &BTreeMap<Point, Tile>,
    layers: i16,
) {
    let mut occupied = BTreeMap::<Point, Vec<Var>>::new();
    let mut cutters = BTreeMap::<Point, Vec<Var>>::new();
    let mut constants = BTreeMap::new();
    for z in 0..layers {
        for group in 0..8 {
            let poses = if group < 4 {
                vec![(false, 4), (false, 5), (true, 1), (true, 2)]
            } else {
                vec![(true, 8), (true, 9)]
            };
            let mut choices = Vec::new();
            for (reflected, row) in poses {
                for (shift, mirror_x) in
                    (0..=1).flat_map(|shift| [false, true].map(move |mirror| (shift, mirror)))
                {
                    let on = model.boolean();
                    choices.push(on);
                    let r = if reflected { 1 } else { 3 };
                    for (x, y, definition, mask, aux) in [
                        (0, 2, "CutterDefaultInternalVariant", 15, false),
                        (0, 1, "", 0, true),
                        (1, 1, "RotatorOneQuadInternalVariant", 12, false),
                        (1, 2, "RotatorOneQuadCCWInternalVariant", 3, false),
                        (2, 1, "CutterDefaultInternalVariant", 9, false),
                        (2, 0, "", 0, true),
                        (2, 2, "CutterDefaultInternalVariantMirrored", 9, false),
                        (2, 3, "", 0, true),
                    ] {
                        let p = (
                            4 * (group % 4) + shift + if mirror_x { 3 - y } else { y },
                            row + if reflected { x } else { -x },
                            z,
                        );
                        let definition =
                            if reflected != mirror_x && definition.starts_with("Cutter") {
                                if definition.ends_with("Mirrored") {
                                    "CutterDefaultInternalVariant"
                                } else {
                                    "CutterDefaultInternalVariantMirrored"
                                }
                            } else {
                                definition
                            };
                        let k = grammar
                            .kinds
                            .iter()
                            .position(|k| {
                                k.rotation == r
                                    && k.auxiliary == aux
                                    && !k.lift
                                    && if aux {
                                        k.definition.is_none()
                                    } else {
                                        k.definition.as_deref() == Some(definition)
                                    }
                            })
                            .unwrap() as i64;
                        let wanted = *constants.entry(k).or_insert_with(|| model.variable(k, k));
                        equal_if(
                            model,
                            on,
                            tiles[&p].kind,
                            wanted,
                            grammar.kinds.len() as i64,
                        );
                        if mask != 0 {
                            let value = model.variable(mask, mask);
                            equal_if(model, on, tiles[&p].shapes[0], value, 15);
                        }
                        occupied.entry(p).or_default().push(on);
                        if definition.starts_with("Cutter") {
                            cutters.entry(p).or_default().push(on);
                        }
                    }
                }
            }
            model.exactly_one(choices);
        }
    }
    for choices in occupied.into_values() {
        model.at_most_one(choices);
    }
    for (&p, tile) in tiles {
        let mut count = Expr::from(tile.cut);
        for &on in cutters.get(&p).into_iter().flatten() {
            count.term(on, -1);
        }
        model.equal(count, 0);
    }
}

/// Set packing avoids assigning interchangeable core identities or per-layer production quotas.
fn pack_cores(
    model: &mut Model,
    grammar: &Grammar,
    tiles: &BTreeMap<Point, Tile>,
    width: i32,
    height: i32,
    layers: i16,
    domains: &[CoreDomain],
) {
    let mut occupied = BTreeMap::<Point, Vec<Var>>::new();
    let mut cutters = BTreeMap::<Point, Vec<Var>>::new();
    let mut constants = BTreeMap::new();
    let mut choices = Vec::new();
    let mut groups = vec![Vec::new(); domains.len() * layers as usize];
    for z in 0..layers {
        for y0 in 0..height {
            for x0 in 0..width {
                for r in 0..4 {
                    let group = domains.iter().position(|d| {
                        d.rotation == r
                            && (d.bounds[0]..=d.bounds[2]).contains(&x0)
                            && (d.bounds[1]..=d.bounds[3]).contains(&y0)
                    });
                    if !domains.is_empty() && group.is_none() {
                        continue;
                    }
                    for mirror in [false, true] {
                        let mut cells = Vec::new();
                        for (x, y, name, mask, aux) in [
                            (0, 2, "CutterDefaultInternalVariant", 15, false),
                            (0, 1, "", 0, true),
                            (1, 1, "RotatorOneQuadInternalVariant", 12, false),
                            (1, 2, "RotatorOneQuadCCWInternalVariant", 3, false),
                            (2, 1, "CutterDefaultInternalVariant", 9, false),
                            (2, 0, "", 0, true),
                            (2, 2, "CutterDefaultInternalVariantMirrored", 9, false),
                            (2, 3, "", 0, true),
                        ] {
                            let y = if mirror { 3 - y } else { y };
                            let (dx, dy) = [(x, y), (-y, x), (-x, -y), (y, -x)][r];
                            let p = (x0 + dx, y0 + dy, z);
                            if !tiles.contains_key(&p) {
                                break;
                            }
                            let name = if mirror && name.starts_with("Cutter") {
                                if name.ends_with("Mirrored") {
                                    "CutterDefaultInternalVariant"
                                } else {
                                    "CutterDefaultInternalVariantMirrored"
                                }
                            } else {
                                name
                            };
                            let k = grammar
                                .kinds
                                .iter()
                                .position(|k| {
                                    k.rotation == r
                                        && k.auxiliary == aux
                                        && !k.lift
                                        && if aux {
                                            k.definition.is_none()
                                        } else {
                                            k.definition.as_deref() == Some(name)
                                        }
                                })
                                .unwrap() as i64;
                            cells.push((p, k, mask, name.starts_with("Cutter")));
                        }
                        if cells.len() != 8 {
                            continue;
                        }
                        let on = model.boolean();
                        choices.push(on);
                        if let Some(group) = group {
                            groups[z as usize * domains.len() + group].push(on);
                        }
                        for (p, k, mask, cut) in cells {
                            let wanted =
                                *constants.entry(k).or_insert_with(|| model.variable(k, k));
                            equal_if(
                                model,
                                on,
                                tiles[&p].kind,
                                wanted,
                                grammar.kinds.len() as i64,
                            );
                            if mask != 0 {
                                let value = *constants
                                    .entry(mask)
                                    .or_insert_with(|| model.variable(mask, mask));
                                equal_if(model, on, tiles[&p].shapes[0], value, 15);
                            }
                            occupied.entry(p).or_default().push(on);
                            if cut {
                                cutters.entry(p).or_default().push(on);
                            }
                        }
                    }
                }
            }
        }
    }
    model.bounded(Expr::sum(choices), 0, 8 * i64::from(layers));
    for group in groups {
        model.exactly_one(group);
    }
    for choices in occupied.into_values() {
        model.at_most_one(choices);
    }
    for (&p, t) in tiles {
        let mut eq = Expr::from(t.cut);
        for &on in cutters.get(&p).into_iter().flatten() {
            eq.term(on, -1);
        }
        model.equal(eq, 0);
    }
}

/// Construct constraints from region geometry; file and CLI concerns stay outside.
fn build(args: &Region, domains: &[CoreDomain]) -> Grid {
    let (width, height) = if args.domain == Domain::Cell {
        (5, 4)
    } else if args.half().is_some() {
        (17, 14)
    } else if args.domain == Domain::Bank {
        (args.bank_width, args.bank_height)
    } else {
        (34, 14)
    };
    let layers = args.layers;
    let mut grammar = Grammar::build();
    let mut kinds = Vec::<Kind>::new();
    let remap: Vec<_> = grammar
        .kinds
        .iter()
        .map(|k| {
            if let Some(i) = kinds.iter().position(|other| {
                other.definition == k.definition
                    && other.rotation == k.rotation
                    && other.auxiliary == k.auxiliary
                    && other.jump == k.jump
                    && other.lift == k.lift
            }) {
                i
            } else {
                let i = kinds.len();
                kinds.push(k.clone());
                i
            }
        })
        .collect();
    let rows = grammar
        .rows
        .into_iter()
        .map(|mut row| {
            row[0] = remap[row[0] as usize] as i64;
            for v in &mut row[1..] {
                *v = if *v == 0 {
                    0
                } else if *v >= 1000 {
                    (*v - 1000) / 128
                } else if *v < 80 {
                    1
                } else {
                    2 + (*v - 80) / 3
                };
            }
            row
        })
        .collect::<std::collections::BTreeSet<_>>();
    grammar.rows = rows.into_iter().collect();
    grammar.kinds = kinds;
    let mut model = Model::default();
    let wiring = model.relation(grammar.rows.clone());
    let transforms = model.relation(shape_relations());
    let edge_types = model.relation(
        (0..=105)
            .map(|k| {
                vec![
                    k,
                    i64::from(k != 0),
                    i64::from(k == 1),
                    i64::from(k >= 2),
                    i64::from((2..=9).contains(&k)),
                ]
            })
            .collect(),
    );
    let mut shape_rows = vec![vec![0, 0]];
    shape_rows.extend(SHAPES.into_iter().map(|s| vec![1, s]));
    let edge_shapes = model.relation(shape_rows);
    let classes = model.relation(
        grammar
            .kinds
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let op = operation(k);
                let def = k.definition.as_deref().unwrap_or("");
                let capacity = if i == 0 {
                    0
                } else if k.auxiliary && !k.lift || op == 4 {
                    1
                } else if op != 0 {
                    2
                } else {
                    4
                };
                let delta = if def.starts_with("Splitter") {
                    1
                } else if def.starts_with("Merger3") {
                    -2
                } else if def.starts_with("Merger") {
                    -1
                } else {
                    0
                };
                let mut row = vec![0; 38];
                row[..6].copy_from_slice(&[
                    i as i64,
                    i64::from(i != 0),
                    i64::from(op == 4),
                    capacity,
                    op,
                    delta,
                ]);
                if let Some(j) = k.jump {
                    row[6 + j] = 1;
                }
                row
            })
            .collect(),
    );
    let mut cores = BTreeMap::<Point, (Vec<i64>, i64)>::new();
    if args.cores {
        for i in 0..if args.half().is_some() {
            8
        } else if args.domain == Domain::Bank {
            4
        } else {
            16
        } {
            let reflected = args.half().is_some() && (i >= 4 || args.inward_top);
            let r = if args.corner_cores && i < 4 {
                2
            } else if args.half().is_some() {
                if reflected { 1 } else { 3 }
            } else if args.domain == Domain::Bank || i < 8 {
                3
            } else {
                1
            };
            let origin = if args.corner_cores {
                [
                    (4, 3),
                    (4, 7),
                    (4, 11),
                    (12, 3),
                    (12, 4),
                    (8, 9),
                    (12, 9),
                    (16, 9),
                ][i as usize]
            } else if args.half().is_some() {
                (
                    4 * (i % 4) + if reflected { 3 } else { 0 },
                    if i >= 4 {
                        8
                    } else if args.inward_top {
                        1
                    } else {
                        5
                    },
                )
            } else if args.domain == Domain::Bank {
                (4 * i, height - 2)
            } else if i < 8 {
                (1 + 4 * (i % 8), 5)
            } else {
                (4 + 4 * (i % 8), 8)
            };
            for (x, y, definition, mask, aux, mirrored) in [
                (0, 2, "CutterDefaultInternalVariant", 15, false, false),
                (0, 1, "", 0, true, false),
                (1, 1, "RotatorOneQuadInternalVariant", 12, false, false),
                (1, 2, "RotatorOneQuadCCWInternalVariant", 3, false, false),
                (2, 1, "CutterDefaultInternalVariant", 9, false, false),
                (2, 0, "", 0, true, false),
                (2, 2, "CutterDefaultInternalVariantMirrored", 9, false, true),
                (2, 3, "", 0, true, true),
            ] {
                let y = if reflected { 3 - y } else { y };
                let mirrored = if reflected != args.mirror_cores {
                    !mirrored
                } else {
                    mirrored
                };
                let definition =
                    if reflected != args.mirror_cores && definition.starts_with("Cutter") {
                        if definition.ends_with("Mirrored") {
                            "CutterDefaultInternalVariant"
                        } else {
                            "CutterDefaultInternalVariantMirrored"
                        }
                    } else {
                        definition
                    };
                let (dx, dy) = [(x, y), (-y, x), (-x, -y), (y, -x)][r];
                let p = (
                    if args.mirror_cores {
                        width - 1 - origin.0 - dx
                    } else {
                        origin.0 + dx
                    },
                    origin.1 + dy,
                );
                let mut allowed = vec![0];
                for row in &grammar.rows {
                    let k = &grammar.kinds[row[0] as usize];
                    if k.rotation != r || k.auxiliary != aux {
                        continue;
                    }
                    if aux {
                        if row[1..5].contains(&(2 + (r * 2 + usize::from(mirrored)) as i64)) {
                            allowed.push(row[0]);
                        }
                    } else if k.definition.as_deref() == Some(definition) {
                        allowed.push(row[0]);
                    }
                }
                allowed.sort_unstable();
                allowed.dedup();
                for z in 0..layers {
                    cores.insert((p.0, p.1, z), (allowed.clone(), mask));
                }
            }
        }
    }
    let open_kinds: Vec<_> = grammar
        .kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| (!k.auxiliary || k.lift) && operation(k) != 4)
        .map(|(i, _)| vec![i as i64])
        .collect();
    let open_table = model.relation(open_kinds);
    let mut edges = BTreeMap::<(Point, usize), Edge>::new();
    let mut tiles = BTreeMap::new();
    let mut objective = Expr::default();
    let mut cuts = Expr::default();
    let mut stream_balance = Expr::default();
    let mut source_flow = Expr::default();
    for z in 0..layers {
        for y in 0..height {
            for x in 0..width {
                let p = (x, y, z);
                let kind = model.variable(0, grammar.kinds.len() as i64 - 1);
                let ports = std::array::from_fn(|i| {
                    let d = i % 6;
                    let key = if i < 6 {
                        (step(p, d, 1), opposite(d))
                    } else {
                        (p, d)
                    };
                    *edges.entry(key).or_insert_with(|| {
                        let wire = model.variable(0, 105);
                        let active = model.boolean();
                        let normal = model.boolean();
                        let internal = model.boolean();
                        let shape = model.variable(0, 15);
                        let rate = model.flow(active, 4);
                        let narrow = model.boolean();
                        model.allow(edge_types, vec![wire, active, normal, internal, narrow]);
                        model.allow(edge_shapes, vec![active, shape]);
                        model.bounded(
                            Expr {
                                terms: vec![(rate, 1), (narrow, 3)],
                                constant: 0,
                            },
                            i64::MIN,
                            4,
                        );
                        Edge {
                            wire,
                            shape,
                            rate,
                            active,
                            normal,
                            internal,
                        }
                    })
                });
                let used = model.boolean();
                let cut = model.boolean();
                let cap = model.variable(0, 4);
                let op = model.variable(0, 4);
                let delta = model.variable(-2, 1);
                let jumps = std::array::from_fn(|_| model.boolean());
                let shapes = std::array::from_fn(|_| model.variable(0, 15));
                let mut cls = vec![kind, used, cut, cap, op, delta];
                cls.extend(jumps);
                model.allow(classes, cls);
                let mut state = vec![kind];
                state.extend(ports.map(|e| e.wire));
                model.allow(wiring, state);
                model.allow(transforms, vec![op, shapes[0], shapes[1], shapes[2]]);
                let mut balance = Expr::from(cut);
                for (i, e) in ports.iter().enumerate() {
                    model.bounded(
                        Expr {
                            terms: vec![(e.rate, 1), (cap, -1)],
                            constant: 0,
                        },
                        i64::MIN,
                        0,
                    );
                    if i < 6 {
                        equal_if(&mut model, e.active, e.shape, shapes[0], 15);
                        balance.term(e.rate, 1);
                    } else {
                        equal_if(&mut model, e.normal, e.shape, shapes[1], 15);
                        equal_if(&mut model, e.internal, e.shape, shapes[2], 15);
                        balance.term(e.rate, -1);
                    }
                }
                let mut upper = balance.clone();
                let mut lower = balance;
                for j in jumps {
                    upper.term(j, -4);
                    lower.term(j, 4);
                }
                model.bounded(upper, i64::MIN, 0);
                model.bounded(lower, 0, i64::MAX);
                if args.cores {
                    if let Some((allowed, mask)) = cores.get(&p) {
                        let rel = model.relation(allowed.iter().map(|&k| vec![k]).collect());
                        model.allow(rel, vec![kind]);
                        if *mask != 0 {
                            let wanted = model.variable(*mask, *mask);
                            equal_if(&mut model, used, shapes[0], wanted, 15);
                        }
                    } else {
                        model.allow(open_table, vec![kind]);
                    }
                }
                objective.term(used, 1);
                cuts.term(cut, 1);
                stream_balance.term(delta, 1);
                model.hint(kind, 0);
                tiles.insert(
                    p,
                    Tile {
                        kind,
                        cut,
                        ports,
                        jumps,
                        shapes,
                    },
                );
            }
        }
    }
    if args.flex_cores {
        flexible_cores(&mut model, &grammar, &tiles, layers);
    }
    if args.free_core_pack {
        pack_cores(&mut model, &grammar, &tiles, width, height, layers, domains);
    }
    if args.pack_left {
        let left = tiles
            .iter()
            .filter(|(p, _)| p.0 < 17)
            .map(|(&p, t)| (p, t.clone()))
            .collect();
        pack_cores(&mut model, &grammar, &left, 17, height, layers, &[]);
    }
    for (&p, t) in &tiles {
        for r in 0..4 {
            let mut nearby = Expr::default();
            let mut count = 0;
            for n in 2..=5 {
                let j = r * 4 + (n - 2) as usize;
                let sender = t.jumps[j];
                let receiver = t.jumps[16 + j];
                if let Some(other) = tiles.get(&step(p, r, n)) {
                    model.equal(
                        Expr {
                            terms: vec![(sender, 1), (other.jumps[16 + j], -1)],
                            constant: 0,
                        },
                        0,
                    );
                    equal_if(
                        &mut model,
                        sender,
                        t.ports[(r + 2) % 4].shape,
                        other.ports[6 + r].shape,
                        15,
                    );
                    equal_if(
                        &mut model,
                        sender,
                        t.ports[(r + 2) % 4].rate,
                        other.ports[6 + r].rate,
                        4,
                    );
                    for k in 0..4 {
                        nearby.term(other.jumps[16 + r * 4 + k], 1);
                    }
                    count += 1;
                } else {
                    model.equal(sender, 0);
                }
                if !tiles.contains_key(&step(p, (r + 2) % 4, n)) {
                    model.equal(receiver, 0);
                }
            }
            if count > 0 {
                for k in 0..4 {
                    nearby.term(t.jumps[r * 4 + k], count - 1);
                }
                model.bounded(nearby, i64::MIN, count);
            }
        }
    }
    let source_ports = if args.domain == Domain::Cell {
        vec![((-1, 2, layers - 1), 0)]
    } else if let Some(half) = args.half() {
        (0..layers)
            .flat_map(|z| {
                if half == OriginalHalf::Left {
                    (5..9).map(|x| ((x, -1, z), 1)).collect::<Vec<_>>()
                } else {
                    args.through_rows.iter().map(|&y| ((-1, y, z), 0)).collect()
                }
            })
            .collect()
    } else if args.domain == Domain::Bank {
        (0..layers)
            .flat_map(|z| {
                let mut ports = vec![((-1, args.bank_input_row.unwrap_or(height - 1), z), 0)];
                if args.bank_through {
                    ports.push(((-1, height - 2, z), 0));
                }
                ports
            })
            .collect()
    } else {
        (0..layers)
            .flat_map(|z| (5..9).map(move |x| ((x, -1, z), 1)))
            .collect()
    };
    let target_ports = if args.domain == Domain::Cell {
        vec![((4, 3, 0), 0)]
    } else if let Some(half) = args.half() {
        (0..layers)
            .flat_map(|z| {
                let (x, d, bottom) = if half == OriginalHalf::Left {
                    (0, 2, 5)
                } else {
                    (16, 0, 8)
                };
                (5..9)
                    .map(move |y| ((x, y, z), d))
                    .chain((bottom..bottom + 4).map(move |x| ((x, 13, z), 1)))
            })
            .collect()
    } else if args.domain == Domain::Bank {
        (0..layers)
            .flat_map(|z| (args.bank_output..args.bank_output + 4).map(move |x| ((x, 0, z), 3)))
            .collect()
    } else {
        (0..layers)
            .flat_map(|z| {
                (5..9)
                    .flat_map(move |y| [((0, y, z), 2), ((33, y, z), 0)])
                    .chain(
                        [5, 25]
                            .into_iter()
                            .flat_map(move |x| (x..x + 4).map(move |x| ((x, 13, z), 1))),
                    )
            })
            .collect::<Vec<_>>()
    };
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    let mut served = Vec::new();
    for (p, d) in source_ports {
        let e = edges[&(p, d)];
        model.equal(e.internal, 0);
        model.equal(
            Expr {
                terms: vec![(e.shape, 1), (e.active, -15)],
                constant: 0,
            },
            0,
        );
        if args.domain == Domain::Cell {
            model.bounded(e.rate.into(), 0, 1);
        }
        source_flow.term(e.rate, 1);
        stream_balance.term(e.active, 1);
        sources.push((p, d, e));
    }
    for (p, d) in target_ports {
        let e = edges[&(p, d)];
        model.equal(e.internal, 0);
        model.equal(
            Expr {
                terms: vec![(e.shape, 1), (e.active, -1)],
                constant: 0,
            },
            0,
        );
        model.equal(
            Expr {
                terms: vec![(e.rate, 1), (e.active, -4)],
                constant: 0,
            },
            0,
        );
        let yes = e.active;
        objective.term(yes, -10000);
        cuts.term(yes, -3);
        stream_balance.term(yes, 2);
        source_flow.term(yes, -1);
        served.push(yes);
        targets.push((p, d, e));
    }
    if args.bank_through || args.half() == Some(OriginalHalf::Left) {
        for z in 0..layers {
            for y in if args.bank_through {
                vec![height - 2]
            } else {
                args.through_rows.to_vec()
            } {
                let p = (width - 1, y, z);
                let e = edges[&(p, 0)];
                model.equal(e.wire, 1);
                model.equal(e.shape, 15);
                model.equal(e.rate, 4);
                source_flow.term(e.rate, -1);
                stream_balance.term(e.active, -1);
                targets.push((p, 0, e));
            }
        }
    }
    for (&(p, d), e) in &edges {
        let q = step(p, d, 1);
        if (!tiles.contains_key(&p) || !tiles.contains_key(&q))
            && !sources
                .iter()
                .chain(&targets)
                .any(|&(a, b, _)| a == p && b == d)
        {
            model.equal(e.wire, 0);
        }
    }
    model.equal(cuts, 0);
    model.equal(stream_balance, 0);
    model.equal(source_flow, 0);
    model.minimize(objective);
    Grid {
        model,
        grammar,
        tiles,
        served,
        sources,
        targets,
        width,
        height,
        layers,
    }
}
/// Checkpoint representation of an occupied tile and its twelve encoded flow ports.
#[derive(Clone, Serialize, Deserialize)]
struct CellState {
    /// Local tile x, y, and zero-based layer of this occupied checkpoint cell.
    position: Point,
    /// Index into the associated grammar kind table; zero denotes an empty tile.
    kind: usize,
    /// Encoded flow tokens: six inputs then six outputs in D order; zero means no flow.
    ports: [i64; 12],
}
/// Extract occupied cells in coordinate order, retaining auxiliary footprints and encoded ports for restart.
fn selected(grid: &Grid, solution: &FeasibleSolution) -> Vec<CellState> {
    grid.tiles
        .iter()
        .filter_map(|(&p, t)| {
            let kind = solution.value(t.kind) as usize;
            (kind != 0).then(|| CellState {
                position: p,
                kind,
                ports: t.ports.map(|e| edge_value(e, solution)),
            })
        })
        .collect()
}
/// Decode twelve port tokens, match a legal kind and shape transform, and hint an existing tile; fail if no grammar state matches.
fn apply_hint(grid: &mut Grid, p: Point, ports: [i64; 12], kind: Option<usize>) -> Result<usize> {
    let decoded = ports.map(|value| {
        if value == 0 {
            (0, 0, 0)
        } else if value < 80 {
            (1, (value - 1) / 4, (value - 1) % 4 + 1)
        } else if value >= 1000 {
            let normal = (value - 1000) % 128;
            ((value - 1000) / 128, (normal - 1) / 4, (normal - 1) % 4 + 1)
        } else {
            (
                2 + (value - 80) / 3,
                [12, 8, 4][((value - 80) % 3) as usize],
                1,
            )
        }
    });
    let wires = decoded.map(|(w, _, _)| w);
    let shapes = shape_relations();
    let found = grid
        .grammar
        .rows
        .iter()
        .find_map(|row| {
            if row[1..] != wires || kind.is_some_and(|k| k != row[0] as usize) {
                return None;
            }
            let op = operation(&grid.grammar.kinds[row[0] as usize]);
            shapes
                .iter()
                .find(|s| {
                    s[0] == op
                        && decoded.iter().enumerate().all(|(i, &(w, shape, _))| {
                            w == 0
                                || shape
                                    == s[if i < 6 {
                                        1
                                    } else if w == 1 {
                                        2
                                    } else {
                                        3
                                    }]
                        })
                })
                .map(|s| (row[0], s.clone()))
        })
        .ok_or_else(|| anyhow::anyhow!("No matching native grammar state for hint at {p:?}"))?;
    let t = &grid.tiles[&p];
    grid.model.hint(t.kind, found.0);
    for (&v, &value) in t.shapes.iter().zip(&found.1[1..]) {
        grid.model.hint(v, value);
    }
    for (e, (wire, shape, rate)) in t.ports.iter().zip(decoded) {
        for (v, value) in [
            (e.wire, wire),
            (e.shape, shape),
            (e.rate, rate),
            (e.active, i64::from(wire != 0)),
            (e.normal, i64::from(wire == 1)),
            (e.internal, i64::from(wire >= 2)),
        ] {
            grid.model.hint(v, value);
        }
    }
    Ok(found.0 as usize)
}
/// Apply optional hints, solve the region and retain its physical buildings and flow.
fn solve(
    args: &Region,
    search: &SearchSettings,
    execution: &Execution,
) -> Result<crate::RegionSolution> {
    ensure!(
        (1..=3).contains(&args.layers),
        "One to three native layers are supported"
    );
    if args.domain == Domain::Bank {
        ensure!(
            args.bank_width >= 4
                && args.bank_height >= 3
                && args.bank_output >= 0
                && args.bank_output + 4 <= args.bank_width,
            "Bank outputs must fit its dimensions"
        );
        if let Some(row) = args.bank_input_row {
            ensure!(
                (0..args.bank_height).contains(&row)
                    && (!args.bank_through || row != args.bank_height - 2),
                "Bank input must be a distinct boundary port"
            );
        }
        ensure!(
            !args.cores || args.bank_width >= 16 && args.bank_height >= 6,
            "Fixed bank cores need at least 16x6"
        );
    }
    let domains: Vec<CoreDomain> = execution
        .core_domains
        .as_ref()
        .map(|p| -> Result<_> {
            Ok(toml::from_str::<CoreDomains>(&fs::read_to_string(p)?)?.domains)
        })
        .transpose()?
        .unwrap_or_default();
    ensure!(
        domains.is_empty() || domains.len() == 8,
        "Expected eight core domains"
    );
    for d in &domains {
        ensure!(
            d.rotation < 4 && d.bounds[0] <= d.bounds[2] && d.bounds[1] <= d.bounds[3],
            "Invalid core pose domain"
        );
    }
    for (i, a) in domains.iter().enumerate() {
        for b in &domains[..i] {
            ensure!(
                a.rotation != b.rotation
                    || a.bounds[0] > b.bounds[2]
                    || b.bounds[0] > a.bounds[2]
                    || a.bounds[1] > b.bounds[3]
                    || b.bounds[1] > a.bounds[3],
                "Core pose domains must be disjoint"
            );
        }
    }
    let mut grid = build(args, &domains);
    grid.model.bounded(
        Expr::sum(grid.served.iter().copied()),
        search.minimum,
        grid.served.len() as i64,
    );
    if search.feasibility {
        ensure!(
            search.minimum == grid.served.len() as i64,
            "Feasibility search requires all outputs"
        );
        grid.model.minimize(Expr::default());
    }
    if let Some(path) = &execution.buildings_hint {
        let blueprint = shapez_blueprint::Blueprint::read(path)?;
        let buildings = match &blueprint.content {
            shapez_blueprint::BlueprintContent::Buildings(b) => &b.entries,
            shapez_blueprint::BlueprintContent::Islands(_) => blueprint
                .islands()
                .first()
                .ok_or_else(|| anyhow::anyhow!("Empty hint blueprint"))?
                .buildings(),
        };
        let offset = if matches!(
            blueprint.content,
            shapez_blueprint::BlueprintContent::Buildings(_)
        ) {
            0
        } else {
            3
        };
        let lookup: BTreeMap<_, _> = buildings
            .iter()
            .map(|b| ((b.x - offset, b.y - offset, b.layer), b))
            .collect();
        let mut known = BTreeMap::new();
        for (&p, b) in &lookup {
            if !grid.tiles.contains_key(&p) {
                continue;
            }
            let r = b.rotation.index();
            let jump = if b.definition.starts_with("BeltPort") {
                let sender = b.definition.contains("Sender");
                let d = if sender { r } else { opposite(r) };
                let distance = (2..=5)
                    .find(|&n| {
                        lookup.get(&step(p, d, n)).is_some_and(|other| {
                            other.rotation.index() == r
                                && other.definition.contains(if sender {
                                    "Receiver"
                                } else {
                                    "Sender"
                                })
                        })
                    })
                    .ok_or_else(|| anyhow::anyhow!("Unpaired native jump at {p:?}"))?;
                Some(if sender { 0 } else { 16 } + r * 4 + (distance - 2) as usize)
            } else {
                None
            };
            let kind = grid
                .grammar
                .kinds
                .iter()
                .position(|k| {
                    k.definition.as_deref() == Some(&b.definition)
                        && k.rotation == r
                        && k.jump == jump
                })
                .ok_or_else(|| anyhow::anyhow!("Unsupported native hint {}", b.definition))?;
            known.insert(p, kind);
            if b.definition.starts_with("Cutter") {
                let side = (r + if b.definition.ends_with("Mirrored") {
                    1
                } else {
                    3
                }) % 4;
                let aux = grid
                    .grammar
                    .kinds
                    .iter()
                    .position(|k| k.auxiliary && !k.lift && k.rotation == r)
                    .unwrap();
                known.insert(step(p, side, 1), aux);
            } else if b.definition.starts_with("Lift") {
                let length = b.definition.as_bytes()[4] - b'0';
                let axis = if b.definition.contains("Up") { 4 } else { 5 };
                let aux = grid
                    .grammar
                    .kinds
                    .iter()
                    .position(|k| k.auxiliary && k.lift && k.rotation == r)
                    .unwrap();
                for n in 1..=length {
                    known.insert(step(p, axis, i32::from(n)), aux);
                }
            }
        }
        for (&p, t) in &grid.tiles {
            let kind = known.get(&p).copied().unwrap_or(0) as i64;
            grid.model.hint(t.kind, kind);
            if execution.lock_layout || execution.fix_known_layout && known.contains_key(&p) {
                grid.model.equal(t.kind, kind);
            }
        }
    }
    if let Some(path) = &execution.unit_hint {
        ensure!(
            !(args.domain == Domain::Cell),
            "Unit placement hint is for the platform"
        );
        let checkpoint: GridCheckpoint = crate::checkpoint::load(path, "grid")?;
        ensure!(
            checkpoint.kinds == grid.grammar.kinds,
            "Checkpoint uses a different building grammar"
        );
        let states = checkpoint.states;
        let mut placed = Vec::new();
        for state in states {
            let mut ports = [0; 12];
            for (i, value) in state.ports.into_iter().enumerate() {
                ports[(if i % 6 < 4 { (i % 6 + 2) % 4 } else { i % 6 }) + 6 * (i / 6)] =
                    if (80..104).contains(&value) {
                        80 + (value - 80 + 12) % 24
                    } else {
                        value
                    };
            }
            for z in 0..grid.layers {
                placed.push(((4 - state.position.0, 9 - state.position.1, z), ports));
            }
        }
        for y in 0..=7 {
            let mut ports = [0; 12];
            ports[3] = 61;
            ports[6 + if y == 7 { 2 } else { 1 }] = 61;
            for z in 0..grid.layers {
                placed.push(((5, y, z), ports));
            }
        }
        for (p, ports) in placed {
            apply_hint(&mut grid, p, ports, None)?;
        }
    }
    if let Some(path) = &execution.resume {
        let checkpoint: GridCheckpoint = crate::checkpoint::load(path, "grid")?;
        ensure!(
            checkpoint.kinds == grid.grammar.kinds,
            "Checkpoint uses a different building grammar"
        );
        let states = checkpoint.states;
        let mut known = BTreeMap::new();
        for s in &states {
            let kind = if execution.infer_hint {
                let direction = s.ports[..4].iter().position(|&v| v > 0 && v < 80);
                let receiver = s.ports[6..10].iter().position(|&v| v > 0 && v < 80);
                let jump = if let Some(input) =
                    direction.filter(|_| s.ports[6..].iter().all(|&v| v == 0))
                {
                    let d = opposite(input);
                    (2..=5)
                        .find(|&n| {
                            states.iter().any(|b| {
                                b.position == step(s.position, d, n)
                                    && b.ports[..6].iter().all(|&v| v == 0)
                                    && b.ports[6 + d] == s.ports[input]
                            })
                        })
                        .map(|n| d * 4 + (n - 2) as usize)
                } else if let Some(d) = receiver.filter(|_| s.ports[..6].iter().all(|&v| v == 0)) {
                    (2..=5)
                        .find(|&n| {
                            states.iter().any(|b| {
                                b.position == step(s.position, opposite(d), n)
                                    && b.ports[6..].iter().all(|&v| v == 0)
                                    && b.ports[opposite(d)] == s.ports[6 + d]
                            })
                        })
                        .map(|n| 16 + d * 4 + (n - 2) as usize)
                } else {
                    None
                };
                jump.and_then(|j| grid.grammar.kinds.iter().position(|k| k.jump == Some(j)))
            } else {
                Some(s.kind)
            };
            let kind = apply_hint(&mut grid, s.position, s.ports, kind)?;
            known.insert(s.position, kind);
        }
        if let [x0, y0, x1, y1] = execution.window[..] {
            for (&p, t) in &grid.tiles {
                if p.0 < x0 || p.0 >= x1 || p.1 < y0 || p.1 >= y1 {
                    grid.model
                        .equal(t.kind, known.get(&p).copied().unwrap_or(0) as i64);
                }
            }
        }
    }
    let (vars, rows) = grid.model.counts();
    eprintln!(
        "Finite grid: {vars} variables, {rows} constraints, {} legal states",
        grid.grammar.rows.len()
    );
    let solution = grid.model.solve_cp_sat(&Limits {
        seconds: search.seconds,
        workers: search.workers,
        log: !search.quiet,
    })?;
    let complete = match &solution {
        Solution::Optimal(s) | Solution::Feasible(s) => {
            grid.served.iter().map(|&v| s.value(v)).sum::<i64>()
        }
        _ => 0,
    };
    eprintln!(
        "{:?}: {complete}/{} outputs in {:.2}s",
        solution.status(),
        grid.served.len(),
        solution.metadata().seconds()
    );
    let status = solution.status();
    let (Solution::Optimal(solution) | Solution::Feasible(solution)) = solution else {
        anyhow::bail!("No complete grid solution: {:?}", status);
    };
    let state = selected(&grid, &solution);
    if let Some(path) = &execution.checkpoint {
        crate::checkpoint::save(
            path,
            "grid",
            &GridCheckpoint {
                kinds: grid.grammar.kinds.clone(),
                states: state.clone(),
            },
        )?;
    }
    ensure!(
        complete == grid.served.len() as i64,
        "Incomplete grid solution: {complete}/{} outputs; checkpoint retained if requested",
        grid.served.len()
    );
    let buildings = state
        .iter()
        .filter_map(|s| {
            let kind = &grid.grammar.kinds[s.kind];
            kind.definition.as_ref().map(|d| {
                Building::new(
                    d,
                    s.position.0,
                    s.position.1,
                    s.position.2,
                    Rotation::new(kind.rotation),
                )
            })
        })
        .collect::<Vec<_>>();
    if execution.native {
        native(args, &grid, &solution, buildings.clone())?;
    }
    Ok(crate::RegionSolution {
        buildings,
        flow: state
            .into_iter()
            .map(|s| crate::FlowState {
                position: [s.position.0, s.position.1, i32::from(s.position.2)],
                ports: s.ports,
            })
            .collect(),
    })
}

/// End-to-end factory search options, independent of regional debug switches.
#[derive(clap::Args)]
pub struct FactoryArgs {
    /// Destination blueprint path supplied as a positional CLI argument.
    pub output: PathBuf,
    /// CP-SAT time budget in seconds for each regional solve; defaults to 120.
    #[arg(long, default_value_t = 120.)]
    pub seconds: f64,
    /// Number of parallel CP-SAT search workers; defaults to 8.
    #[arg(long, default_value_t = 8)]
    pub workers: i32,
    /// Solve the west-input bank layout instead of the original northern-input halves.
    #[arg(long)]
    pub west: bool,
    /// Request native game validation of the resulting blueprint.
    #[arg(long)]
    pub native: bool,
    /// Suppress CP-SAT progress logging; summary diagnostics remain enabled.
    #[arg(long)]
    pub quiet: bool,
}
/// Solve both regions, retaining the actual buildings and flow in a single result.
pub fn factory(options: &FactoryArgs) -> Result<crate::FactorySolution> {
    let search = SearchSettings {
        seconds: options.seconds,
        workers: options.workers,
        quiet: options.quiet,
        feasibility: false,
        minimum: 0,
    };
    let execution = Execution::default();
    let mut region = Region::new(if options.west {
        Domain::Bank
    } else {
        Domain::Half(OriginalHalf::Left)
    });
    region.bank_height = 7;
    region.flex_cores = !options.west;
    region.cores = options.west;
    region.bank_through = options.west;
    region.layers = if options.west { 3 } else { 1 };
    let left = solve(&region, &search, &execution)?;
    if options.west {
        region.bank_through = false;
        region.layers = 1;
        region.bank_input_row = Some(5);
        region.bank_output = 8;
    } else {
        region.domain = Domain::Half(OriginalHalf::Right);
    }
    let right = solve(&region, &search, &execution)?;
    Ok(crate::FactorySolution {
        layout: if options.west {
            crate::Layout::WestInput
        } else {
            crate::Layout::Original
        },
        left,
        right,
    })
}
/// Export the final regional blueprint; no intermediate result is required by another command.
pub fn run(args: Args) -> Result<()> {
    let domain = if args.cell {
        Domain::Cell
    } else if let Some(half) = args.original_half {
        Domain::Half(half)
    } else if args.bank {
        Domain::Bank
    } else {
        Domain::Platform
    };
    let region = Region {
        domain,
        through_rows: args
            .through_rows
            .try_into()
            .map_err(|_| anyhow::anyhow!("Expected two through rows"))?,
        inward_top: args.inward_top,
        mirror_cores: args.mirror_cores,
        corner_cores: args.corner_cores,
        flex_cores: args.flex_cores,
        free_core_pack: args.free_core_pack,
        pack_left: args.pack_left,
        bank_width: args.bank_width,
        bank_height: args.bank_height,
        bank_output: args.bank_output,
        bank_through: args.bank_through,
        bank_input_row: args.bank_input_row,
        cores: args.cores,
        layers: args.layers,
    };
    let result = solve(&region, &args.search, &args.execution)?;
    crate::export_buildings(&result.buildings, &args.output)
}
/// Place solved buildings and boundary ports in a native test platform and run throughput/icon acceptance; replicate single-layer full grids three times.
fn native(
    args: &Region,
    grid: &Grid,
    solution: &FeasibleSolution,
    base: Vec<Building>,
) -> Result<()> {
    use shapez_blueprint::{Blueprint, BuildingBlueprint, Island};
    use shapez_runtime::{LabPaths, RunDirectory, run_suite};
    let (offset, layers) = if args.regional() {
        ((8, 4), 1)
    } else {
        ((3, 3), if grid.layers == 1 { 3 } else { 1 })
    };
    let mut buildings = Vec::new();
    for z in 0..layers {
        for b in &base {
            let mut b = b.clone();
            b.x += offset.0;
            b.y += offset.1;
            b.layer += z;
            buildings.push(b);
        }
    }
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for z in 0..layers {
        for &(p, d, v) in &grid.sources {
            ensure!(solution.value(v.rate) > 0, "Unused required source");
            let p = (p.0 + offset.0, p.1 + offset.1, p.2 + z);
            inputs.push(crate::experiment::Input {
                position: p,
                rate: solution.value(v.rate) as f64 * 0.5,
            });
            buildings.push(Building::new(
                if args.regional() {
                    "BeltDefaultForwardInternalVariant"
                } else {
                    "BeltPortReceiverInternalVariant"
                },
                p.0,
                p.1,
                p.2,
                Rotation::new(d),
            ));
        }
        for &(p, d, e) in &grid.targets {
            let p = step(p, d, 1);
            let p = (p.0 + offset.0, p.1 + offset.1, p.2 + z);
            outputs.push(crate::experiment::Output {
                position: p,
                full: solution.value(e.shape) == 15,
            });
            buildings.push(Building::new(
                if args.regional() {
                    "BeltDefaultForwardInternalVariant"
                } else {
                    "BeltPortSenderInternalVariant"
                },
                p.0,
                p.1,
                p.2,
                Rotation::new(d),
            ));
        }
    }
    if args.regional() {
        return crate::experiment::validate(
            "grid",
            buildings,
            Some(crate::design::icon()),
            &inputs,
            &outputs,
        );
    }
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    let count = buildings.len();
    let mergers = buildings
        .iter()
        .filter(|b| b.definition.starts_with("Merger"))
        .count();
    let mut island = Island::new("Foundation_1x2", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let bp = Blueprint::platform(island, Some(crate::design::icon()));
    let blueprint = run.path.join("quarter-cut-1x2.spz2bp");
    bp.write(&blueprint)?;
    eprintln!(
        "Blueprint {} ({}x{})",
        blueprint.display(),
        grid.width,
        grid.height
    );
    {
        use shapez_model::FoundationGeometry;
        ensure!(
            grid.layers == 1 || grid.layers == 3,
            "Factory acceptance requires three resulting layers"
        );
        let geometry = FoundationGeometry::strip(2, 3);
        let sockets = crate::original_sockets();
        let harness = bp.with_harness(&geometry, &sockets)?;
        let test = run.path.join("harness.spz2bp");
        harness.write(&test)?;
        let suite = shapez_runtime::Suite {
            projects: vec!["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj".into()],
            cases: vec![
                shapez_runtime::case! {name:"factory",blueprint:test,ports:geometry.ports(&sockets),expected_buildings:count,expected_mergers:mergers,export_graph:true,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll".into(), type_name: ("QuarterFactoryExperiment").into() }},
                shapez_runtime::case! {name:"library-icon",operation:"inspect-blueprint",blueprint:blueprint.clone(),expected_format_version:5,expected_library_warning:false,expected_icon:bp.icon.as_ref().unwrap().components.clone()},
            ],
        };
        run_suite(&paths, &suite, std::time::Duration::from_secs(180))?;
        run.publish(&paths.root)?;
    }
    Ok(())
}

/// A restart checkpoint keeps solver kind IDs paired with their exact grammar.
#[derive(Serialize, Deserialize)]
struct GridCheckpoint {
    /// Building descriptors indexed by solver kind ID; index zero represents an empty tile.
    kinds: Vec<Kind>,
    /// Occupied tile states whose kind indices refer to the checkpoint grammar.
    states: Vec<CellState>,
}

/// Human-authored search constraints, separate from solver output.
#[derive(Deserialize)]
struct CoreDomains {
    /// Pose groups selecting one core per group and layer; an empty list allows unrestricted packing.
    domains: Vec<CoreDomain>,
}

/// CLI parsing regression checks for mode conflicts and flattened option dependencies.
#[cfg(test)]
mod tests {
    use super::*;
    /// Flattening execution/search options must preserve cross-group CLI requirements.
    #[test]
    fn cli_keeps_mode_conflicts_and_hint_dependencies() {
        assert!(Args::try_parse_from(["grid", "out.spz2bp", "--cell", "--bank"]).is_err());
        assert!(
            Args::try_parse_from(["grid", "out.spz2bp", "--window", "0", "0", "4", "4"]).is_err()
        );
        let args = Args::try_parse_from([
            "grid",
            "out.spz2bp",
            "--cell",
            "--seconds",
            "5",
            "--checkpoint",
            "state.bin",
        ])
        .unwrap();
        assert!(args.cell);
        assert_eq!(args.search.seconds, 5.);
        assert_eq!(
            args.execution.checkpoint.as_deref(),
            Some(std::path::Path::new("state.bin"))
        );
    }
}
