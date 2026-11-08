//! Primitive socket-flow synthesis. No cell identity survives a merge.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use shapez_blueprint::{Building, Rotation};
use shapez_solver::{Expr, FeasibleSolution, Limits, Model, Solution, Var};
use std::{collections::BTreeMap, path::PathBuf};

/// Tile coordinates `(x, y, layer)` in the synthesis volume.
type Point = (i32, i32, i16);
/// Directed connection keyed by its source tile, direction index, and quadrant mask.
type Edge = (Point, usize, u8);
/// Planar offsets in building rotation order: east, south, west, north.
const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
/// Command-line bounds, candidate families, and export options for socket-flow synthesis.
#[derive(Parser)]
pub struct Args {
    /// Destination blueprint for the selected buildings in synthesis coordinates.
    output: PathBuf,
    /// Number of tile columns available to candidate footprints.
    #[arg(long, default_value_t = 2)]
    width: i32,
    /// Number of tile rows; quarter-only synthesis requires eight.
    #[arg(long, default_value_t = 8)]
    height: i32,
    /// Number of usable building layers, from one through three.
    #[arg(long, default_value_t = 2)]
    layers: i16,
    /// Maximum number of selected rotators across all supported shape transitions.
    #[arg(long, default_value_t = 3)]
    rotators: i64,
    /// CP-SAT solve time limit in seconds.
    #[arg(long, default_value_t = 60.)]
    seconds: f64,
    /// Number of CP-SAT search workers.
    #[arg(long, default_value_t = 4)]
    workers: i32,
    /// Validate the exported solution in the native experiment with boundary belts.
    #[arg(long)]
    native: bool,
    /// Also synthesize the cutting stages and their half-shape rotation network.
    #[arg(long)]
    whole: bool,
    /// Include paired jump senders and receivers separated by two through five tiles.
    #[arg(long)]
    jumps: bool,
    /// Allow whole-shape input and quarter output ports on any row of their edges.
    #[arg(long, requires = "whole")]
    free_ports: bool,
    /// Load a compatible checkpoint as placement and socket-flow hints.
    #[arg(long)]
    resume: Option<PathBuf>,
    /// Save selected candidate indices and socket amounts after a feasible solve.
    #[arg(long)]
    checkpoint: Option<PathBuf>,
    /// Number of unit-rate whole inputs and four-unit quarter outputs to select.
    #[arg(long, default_value_t = 2)]
    units: i64,
    /// Extend free boundary port candidates to all four edges on layer zero.
    #[arg(long, requires = "free_ports")]
    all_edges: bool,
}
/// One shape-specific input or output face of a placement.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct Socket {
    /// Tile carrying this face, which may differ from the building anchor.
    position: Point,
    /// Outward-facing direction index after applying the building rotation.
    side: usize,
    /// Occupied-quadrant bit mask; its population count weights material conservation.
    shape: u8,
    /// Whether material enters the placement through this face.
    input: bool,
}
/// Optional placement and the solver variables needed to export or checkpoint it.
struct Choice {
    /// Boolean selecting the entire placement, including any companion.
    enabled: Var,
    /// Primary building at its synthesis position and rotation.
    building: Building,
    /// Resolved socket faces paired with their gated integer flow amounts.
    sockets: Vec<(Socket, Var)>,
    /// Tiles reserved together, including lift layers or both jump endpoints.
    footprint: Vec<Point>,
    /// Receiver emitted alongside a selected jump sender.
    companion: Option<Building>,
}
/// Optional fixed-rate external connection retained for native source and meter placement.
struct Boundary {
    /// Boolean selecting this external connection.
    enabled: Var,
    /// Source tile of the directed edge, outside the volume for supply ports.
    position: Point,
    /// Whether the connection supplies material rather than consumes it.
    supply: bool,
    /// Direction of material travel across the boundary.
    direction: usize,
}
/// Candidate placements and accumulated equations for a socket-connected factory.
#[derive(Default)]
struct Synthesis {
    /// CP-SAT variables, constraints, hints, and placement-cost objective.
    model: Model,
    /// Placement selectors sharing each tile, later constrained to at most one.
    occupied: BTreeMap<Point, Vec<Var>>,
    /// Signed physical-connection and material-flow balances for each directed shape edge.
    edges: BTreeMap<Edge, (Expr, Expr)>,
    /// Placements in deterministic generation order used by checkpoint indices.
    choices: Vec<Choice>,
    /// Rotator selectors summed against the requested machine limit.
    rotators: Vec<Var>,
    /// Jump selectors grouped by sender tile, receiver tile, and travel direction.
    jumps: BTreeMap<(Point, Point, usize), Vec<Var>>,
    /// External connections from which native input and output records are derived.
    boundaries: Vec<Boundary>,
}
/// Step one tile in a direction without changing layers.
fn neighbor(p: Point, d: usize) -> Point {
    (p.0 + DIRECTIONS[d].0, p.1 + DIRECTIONS[d].1, p.2)
}
impl Synthesis {
    /// Add a socket's selector and flow to its canonical directed edge balances.
    /// Inputs subtract from the neighboring source edge; outputs add at their own tile.
    fn socket(&mut self, socket: Socket, enabled: Var, amount: Var) {
        let Socket {
            position,
            side,
            shape,
            input,
        } = socket;
        let (edge, sign) = if input {
            ((neighbor(position, side), (side + 2) % 4, shape), -1)
        } else {
            ((position, side, shape), 1)
        };
        let (physical, flow) = self.edges.entry(edge).or_default();
        physical.term(enabled, sign);
        flow.term(amount, sign);
    }
    /// Register a placement with capacity-gated socket flows and quadrant conservation.
    /// Socket sides are rotated here; positions and footprint tiles must already be resolved.
    /// Every socket shares the placement selector, so selected faces require physical peers.
    fn add(
        &mut self,
        kind: &str,
        p: Point,
        rotation: usize,
        footprint: Vec<Point>,
        sockets: &[(Point, usize, u8, bool)],
        capacity: i64,
    ) {
        let enabled = self.model.boolean();
        let mut conservation = Expr::default();
        let mut resolved = Vec::new();
        for &(position, side, shape, input) in sockets {
            let socket = Socket {
                position,
                side: (side + rotation) % 4,
                shape,
                input,
            };
            let amount = self.model.flow(enabled, capacity);
            self.socket(socket, enabled, amount);
            conservation.term(
                amount,
                i64::from(shape.count_ones()) * if input { 1 } else { -1 },
            );
            resolved.push((socket, amount));
        }
        self.model.equal(conservation, 0);
        for &point in &footprint {
            self.occupied.entry(point).or_default().push(enabled);
        }
        if kind.starts_with("Rotator") {
            self.rotators.push(enabled);
        }
        self.choices.push(Choice {
            enabled,
            building: Building::new(kind, p.0, p.1, p.2, Rotation::new(rotation)),
            sockets: resolved,
            footprint,
            companion: None,
        });
    }
    /// Add an optional external edge carrying `amount` units of the given shape.
    /// Supply adds to both balances; consumption subtracts. Return its selection variable.
    fn boundary(
        &mut self,
        position: Point,
        direction: usize,
        shape: u8,
        amount: i64,
        supply: bool,
    ) -> Var {
        let enabled = self.model.boolean();
        let (physical, flow) = self.edges.entry((position, direction, shape)).or_default();
        let sign = if supply { 1 } else { -1 };
        physical.term(enabled, sign);
        flow.term(enabled, sign * amount);
        self.boundaries.push(Boundary {
            enabled,
            position,
            supply,
            direction,
        });
        enabled
    }
    /// Close edge balances, forbid footprint overlap, and bound selected rotators.
    /// Exclude jump pairs whose senders could see an unintended receiver, then minimize
    /// the weighted cost of 100 per occupied tile plus one per emitted building.
    fn finish(&mut self, rotators: i64) {
        for (_, candidates) in std::mem::take(&mut self.occupied) {
            self.model.at_most_one(candidates);
        }
        for (_, (physical, flow)) in std::mem::take(&mut self.edges) {
            self.model.equal(physical, 0);
            self.model.equal(flow, 0);
        }
        self.model
            .bounded(Expr::sum(self.rotators.iter().copied()), 0, rotators);
        // Native senders inspect all receivers 2..=5 tiles ahead.
        let mut receivers = BTreeMap::<(Point, usize), Vec<(Point, Point, usize)>>::new();
        for &jump in self.jumps.keys() {
            receivers.entry((jump.1, jump.2)).or_default().push(jump);
        }
        let mut conflicts = std::collections::BTreeSet::new();
        for &a in self.jumps.keys() {
            for length in 2..=5 {
                let q = (
                    a.0.0 + DIRECTIONS[a.2].0 * length,
                    a.0.1 + DIRECTIONS[a.2].1 * length,
                    a.0.2,
                );
                if q == a.1 {
                    continue;
                }
                if let Some(others) = receivers.get(&(q, a.2)) {
                    for &b in others {
                        conflicts.insert((a.min(b), a.max(b)));
                    }
                }
            }
        }
        for (a, b) in conflicts {
            self.model
                .at_most_one(self.jumps[&a].iter().chain(&self.jumps[&b]).copied());
        }
        // Occupied tiles first, then prefer fewer machines among equal footprints.
        let mut objective = Expr::default();
        for c in &self.choices {
            objective.term(
                c.enabled,
                100 * c.footprint.len() as i64 + 1 + i64::from(c.companion.is_some()),
            );
        }
        self.model.minimize(objective);
    }
}
/// Enumerate in-bounds transport and processing placements, then finalize their constraints.
/// Quarter-only mode supplies eight fixed quarter streams; whole mode selects whole-shape
/// inputs and exactly three cutters per unit. Each selected output carries four quarters.
fn build(args: &Args) -> Synthesis {
    let mut result = Synthesis::default();
    for z in 0..args.layers {
        for y in 0..args.height {
            for x in 0..args.width {
                let p = (x, y, z);
                for (r, &(dx, dy)) in DIRECTIONS.iter().enumerate() {
                    for &shape in if args.whole {
                        &[1, 2, 4, 8, 3, 6, 9, 12, 15][..]
                    } else {
                        &[1, 8][..]
                    } {
                        if args.jumps {
                            for length in 2..=5 {
                                let q = (p.0 + dx * length, p.1 + dy * length, p.2);
                                if !(0..args.width).contains(&q.0)
                                    || !(0..args.height).contains(&q.1)
                                {
                                    continue;
                                }
                                result.add(
                                    "BeltPortSenderInternalVariant",
                                    p,
                                    r,
                                    vec![p, q],
                                    &[(p, 2, shape, true), (q, 0, shape, false)],
                                    4,
                                );
                                let choice = result.choices.last_mut().unwrap();
                                choice.companion = Some(Building::new(
                                    "BeltPortReceiverInternalVariant",
                                    q.0,
                                    q.1,
                                    q.2,
                                    Rotation::new(r),
                                ));
                                result
                                    .jumps
                                    .entry((p, q, r))
                                    .or_default()
                                    .push(choice.enabled);
                            }
                        }
                        for (kind, side) in [
                            ("BeltDefaultForwardInternalVariant", 0),
                            ("BeltDefaultLeftInternalVariant", 3),
                            ("BeltDefaultLeftInternalVariantMirrored", 1),
                        ] {
                            result.add(
                                kind,
                                p,
                                r,
                                vec![p],
                                &[(p, 2, shape, true), (p, side, shape, false)],
                                4,
                            );
                        }
                        for (kind, sides) in [
                            ("Merger2To1LInternalVariant", &[2, 3][..]),
                            ("Merger2To1LInternalVariantMirrored", &[2, 1][..]),
                            ("Merger3To1InternalVariant", &[2, 3, 1][..]),
                        ] {
                            let mut sockets: Vec<_> =
                                sides.iter().map(|&side| (p, side, shape, true)).collect();
                            sockets.push((p, 0, shape, false));
                            result.add(kind, p, r, vec![p], &sockets, 4);
                        }
                        if args.whole {
                            for (kind, side) in [
                                ("Splitter1To2LInternalVariant", 3),
                                ("Splitter1To2LInternalVariantMirrored", 1),
                            ] {
                                result.add(
                                    kind,
                                    p,
                                    r,
                                    vec![p],
                                    &[
                                        (p, 2, shape, true),
                                        (p, 0, shape, false),
                                        (p, side, shape, false),
                                    ],
                                    4,
                                );
                            }
                        }
                        for dz in [-2_i16, -1, 1, 2] {
                            let target = (x, y, z + dz);
                            if !(0..args.layers).contains(&target.2) {
                                continue;
                            }
                            let footprint = (z.min(target.2)..=z.max(target.2))
                                .map(|layer| (x, y, layer))
                                .collect::<Vec<_>>();
                            for (suffix, side) in [
                                ("Forward", 0),
                                ("Left", 3),
                                ("LeftMirrored", 1),
                                ("Backward", 2),
                            ] {
                                let mirrored = suffix.ends_with("Mirrored");
                                let turn = suffix.trim_end_matches("Mirrored");
                                let kind = format!(
                                    "Lift{}{}{turn}InternalVariant{}",
                                    dz.abs(),
                                    if dz > 0 { "Up" } else { "Down" },
                                    if mirrored { "Mirrored" } else { "" }
                                );
                                result.add(
                                    &kind,
                                    p,
                                    r,
                                    footprint.clone(),
                                    &[(p, 2, shape, true), (target, side, shape, false)],
                                    4,
                                );
                            }
                        }
                    }
                    result.add(
                        "RotatorOneQuadInternalVariant",
                        p,
                        r,
                        vec![p],
                        &[(p, 2, 8, true), (p, 0, 1, false)],
                        2,
                    );
                    if args.whole {
                        for (kind, shape, output) in [
                            ("RotatorOneQuadInternalVariant", 12, 9),
                            ("RotatorOneQuadCCWInternalVariant", 3, 9),
                            ("RotatorOneQuadInternalVariant", 3, 6),
                            ("RotatorOneQuadCCWInternalVariant", 12, 6),
                            ("RotatorOneQuadCCWInternalVariant", 2, 1),
                            ("RotatorHalfInternalVariant", 4, 1),
                        ] {
                            result.add(
                                kind,
                                p,
                                r,
                                vec![p],
                                &[(p, 2, shape, true), (p, 0, output, false)],
                                2,
                            );
                        }
                        for (kind, side) in [
                            ("CutterDefaultInternalVariant", 3),
                            ("CutterDefaultInternalVariantMirrored", 1),
                        ] {
                            let other = neighbor(p, (side + r) % 4);
                            if !(0..args.width).contains(&other.0)
                                || !(0..args.height).contains(&other.1)
                            {
                                continue;
                            }
                            for (input, left, right) in [(15, 12, 3), (9, 8, 1), (6, 4, 2)] {
                                result.add(
                                    kind,
                                    p,
                                    r,
                                    vec![p, other],
                                    &[
                                        (p, 2, input, true),
                                        (other, 0, left, false),
                                        (p, 0, right, false),
                                    ],
                                    1,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // Two adjacent groups of second-stage cutters, each delivering four quarters.
    if args.whole {
        let rows = if args.free_ports {
            (0..args.height).collect::<Vec<_>>()
        } else {
            vec![2, args.height - 2]
        };
        let mut ports: Vec<_> = rows
            .into_iter()
            .map(|y| result.boundary((-1, y, 0), 0, 15, 1, true))
            .collect();
        if args.all_edges {
            for y in 0..args.height {
                ports.push(result.boundary((args.width, y, 0), 2, 15, 1, true));
            }
            for x in 0..args.width {
                ports.push(result.boundary((x, -1, 0), 1, 15, 1, true));
                ports.push(result.boundary((x, args.height, 0), 3, 15, 1, true));
            }
        }
        result.model.equal(Expr::sum(ports), args.units);
        let cutters = result
            .choices
            .iter()
            .filter(|c| c.building.definition.starts_with("Cutter"))
            .map(|c| c.enabled);
        result.model.equal(Expr::sum(cutters), 3 * args.units);
    } else {
        for y in 0..8 {
            let port = result.boundary(
                (-1, y, 0),
                0,
                if y % 4 == 0 || y % 4 == 3 { 8 } else { 1 },
                1,
                true,
            );
            result.model.equal(port, 1);
        }
    }
    let rows = if args.free_ports {
        (0..args.height).collect::<Vec<_>>()
    } else {
        vec![1, args.height - 3]
    };
    let mut ports: Vec<_> = rows
        .into_iter()
        .map(|y| result.boundary((args.width - 1, y, 0), 0, 1, 4, false))
        .collect();
    if args.all_edges {
        for y in 0..args.height {
            ports.push(result.boundary((0, y, 0), 2, 1, 4, false));
        }
        for x in 0..args.width {
            ports.push(result.boundary((x, 0, 0), 3, 1, 4, false));
            ports.push(result.boundary((x, args.height - 1, 0), 1, 1, 4, false));
        }
    }
    result.model.equal(Expr::sum(ports), args.units);
    result.finish(args.rotators);
    result
}
/// Validate options, solve the placement model, and export its selected buildings.
/// Optionally load checkpoint hints, save the feasible assignment, and run native validation.
/// Return an error for invalid bounds, incompatible checkpoints, infeasible search outcomes,
/// solver or file failures, or failed native validation.
pub fn run(args: Args) -> Result<()> {
    ensure!(
        args.width > 0
            && args.height >= 2
            && args.units > 0
            && (args.units == 2 || args.whole && args.free_ports)
            && (args.whole || args.height == 8)
            && (1..=3).contains(&args.layers)
            && args.rotators >= 0
            && args.seconds > 0.,
        "Invalid synthesis dimensions or limits"
    );
    let mut model = build(&args);
    if let Some(path) = &args.resume {
        let checkpoint: WeaveCheckpoint = crate::checkpoint::load(path, "weave")?;
        ensure!(
            checkpoint.spec == specification(&args) && checkpoint.choices == model.choices.len(),
            "Checkpoint belongs to another weave model"
        );
        for (index, candidate) in model.choices.iter().enumerate() {
            let amounts = checkpoint
                .selected
                .iter()
                .find(|(id, _)| *id == index)
                .map(|(_, amounts)| amounts);
            model
                .model
                .hint(candidate.enabled, i64::from(amounts.is_some()));
            if let Some(amounts) = amounts {
                ensure!(
                    amounts.len() == candidate.sockets.len(),
                    "Checkpoint socket count differs"
                );
                for (&value, (_, variable)) in amounts.iter().zip(&candidate.sockets) {
                    model.model.hint(*variable, value);
                }
            }
        }
    }
    let (variables, constraints) = model.model.counts();
    eprintln!(
        "Primitive model: {variables} variables, {constraints} constraints, {} placements",
        model.choices.len()
    );
    let solution = model.model.solve_cp_sat(&Limits {
        seconds: args.seconds,
        workers: args.workers,
        log: false,
    })?;
    let status = solution.status();
    let (Solution::Optimal(solution) | Solution::Feasible(solution)) = solution else {
        anyhow::bail!("No feasible weave solution: {:?}", status);
    };
    let selected = || {
        model
            .choices
            .iter()
            .filter(|c| solution.value(c.enabled) == 1)
    };
    let rotators = selected()
        .filter(|c| c.building.definition.starts_with("Rotator"))
        .count();
    let occupied: usize = selected().map(|c| c.footprint.len()).sum();
    if let Some(path) = &args.checkpoint {
        crate::checkpoint::save(
            path,
            "weave",
            &WeaveCheckpoint {
                spec: specification(&args),
                choices: model.choices.len(),
                selected: model
                    .choices
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| solution.value(c.enabled) == 1)
                    .map(|(index, c)| {
                        (
                            index,
                            c.sockets
                                .iter()
                                .map(|(_, amount)| solution.value(*amount))
                                .collect(),
                        )
                    })
                    .collect(),
            },
        )?;
    }
    let buildings = selected()
        .flat_map(|s| std::iter::once(s.building.clone()).chain(s.companion.clone()))
        .collect::<Vec<_>>();
    crate::export_buildings(&buildings, &args.output)?;
    if args.native {
        native(&args, &buildings, &model, &solution)?;
    }
    let building_count = buildings.len();
    eprintln!(
        "{:?}: {building_count} buildings, {occupied} occupied tiles, {rotators} rotators, {:.2}s",
        status,
        solution.metadata().seconds()
    );
    Ok(())
}

/// Translate selected buildings into the native test area and attach source and meter belts.
/// Quarter-only solutions receive fixed upstream cutting stages; whole solutions use their
/// selected supply boundaries. Feed whole shapes at 0.5 per second and validate quarter outputs.
fn native(
    args: &Args,
    selected: &[Building],
    model: &Synthesis,
    solution: &FeasibleSolution,
) -> Result<()> {
    let mut buildings: Vec<_> = selected
        .iter()
        .map(|building| {
            let mut b = building.clone();
            b.x += if args.whole { 8 } else { 11 };
            b.y += 4;
            b
        })
        .collect();
    if !args.whole {
        for y in [4, 8] {
            for (dx, dy, kind) in [
                (0, 2, "CutterDefaultInternalVariant"),
                (1, 1, "RotatorOneQuadInternalVariant"),
                (1, 2, "RotatorOneQuadCCWInternalVariant"),
                (2, 1, "CutterDefaultInternalVariant"),
                (2, 2, "CutterDefaultInternalVariantMirrored"),
            ] {
                buildings.push(Building::new(kind, 8 + dx, y + dy, 0, Rotation::new(0)));
            }
        }
    }
    let inputs = if args.whole {
        model
            .boundaries
            .iter()
            .filter(|b| b.supply && solution.value(b.enabled) == 1)
            .map(|b| {
                (
                    (8 + b.position.0, 4 + b.position.1, b.position.2),
                    b.direction,
                )
            })
            .collect::<Vec<_>>()
    } else {
        vec![((7, 6, 0), 0), ((7, args.height + 2, 0), 0)]
    };
    let offset = if args.whole { 8 } else { 11 };
    let outputs: Vec<_> = model
        .boundaries
        .iter()
        .filter(|b| !b.supply && solution.value(b.enabled) == 1)
        .map(|b| {
            let q = neighbor(b.position, b.direction);
            ((offset + q.0, 4 + q.1, q.2), b.direction)
        })
        .collect();
    for &((x, y, z), direction) in inputs.iter().chain(&outputs) {
        buildings.push(Building::new(
            "BeltDefaultForwardInternalVariant",
            x,
            y,
            z,
            Rotation::new(direction),
        ));
    }
    let inputs = inputs
        .into_iter()
        .map(|(position, _)| crate::experiment::Input {
            position,
            rate: 0.5,
        })
        .collect::<Vec<_>>();
    let outputs = outputs
        .into_iter()
        .map(|(position, _)| crate::experiment::Output {
            position,
            full: false,
        })
        .collect::<Vec<_>>();
    crate::experiment::validate("weave", buildings, None, &inputs, &outputs)
}

/// Feasible placement and flow assignment persisted for a compatible model's search hints.
#[derive(Serialize, Deserialize)]
struct WeaveCheckpoint {
    /// Model-defining dimensions, limits, and candidate-family switches in specification order.
    spec: [i64; 9],
    /// Total candidate count checked before interpreting saved placement indices.
    choices: usize,
    /// Selected candidate indices with integer flow amounts in each candidate's socket order.
    selected: Vec<(usize, Vec<i64>)>,
}
/// Parameters that determine candidate ordering and flow constraints.
fn specification(args: &Args) -> [i64; 9] {
    [
        i64::from(args.width),
        i64::from(args.height),
        i64::from(args.layers),
        args.rotators,
        args.units,
        i64::from(args.whole),
        i64::from(args.jumps),
        i64::from(args.free_ports),
        i64::from(args.all_edges),
    ]
}
