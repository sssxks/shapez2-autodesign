//! Primitive socket-flow synthesis. No cell identity survives a merge.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use shapez_blueprint::{Building, Rotation};
use shapez_solver::{Expr, Limits, Model, Solution, Var};
use std::{collections::BTreeMap, fs, path::PathBuf};

type Point = (i32, i32, i16);
type Edge = (Point, usize, u8);
const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
#[derive(Parser)]
struct Args {
    output: PathBuf,
    #[arg(long, default_value_t = 2)]
    width: i32,
    #[arg(long, default_value_t = 8)]
    height: i32,
    #[arg(long, default_value_t = 2)]
    layers: i16,
    #[arg(long, default_value_t = 3)]
    rotators: i64,
    #[arg(long, default_value_t = 60.)]
    seconds: f64,
    #[arg(long, default_value_t = 4)]
    workers: i32,
    #[arg(long)]
    native: bool,
    /// Also synthesize the cutting stages and their half-shape rotation network.
    #[arg(long)]
    whole: bool,
    #[arg(long)]
    jumps: bool,
    #[arg(long, requires = "whole")]
    free_ports: bool,
    #[arg(long)]
    hint: Option<PathBuf>,
    #[arg(long, default_value_t = 2)]
    units: i64,
    #[arg(long, requires = "free_ports")]
    all_edges: bool,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
struct Socket {
    position: Point,
    side: usize,
    shape: u8,
    input: bool,
}
struct Choice {
    enabled: Var,
    building: Building,
    sockets: Vec<(Socket, Var)>,
    footprint: Vec<Point>,
    companion: Option<Building>,
}
struct Boundary {
    enabled: Var,
    position: Point,
    supply: bool,
    direction: usize,
}
#[derive(Default)]
struct Synthesis {
    model: Model,
    occupied: BTreeMap<Point, Vec<Var>>,
    // Each directed edge has both physical-connection and material-flow equations.
    edges: BTreeMap<Edge, (Expr, Expr)>,
    choices: Vec<Choice>,
    rotators: Vec<Var>,
    jumps: BTreeMap<(Point, Point, usize), Vec<Var>>,
    boundaries: Vec<Boundary>,
}
fn neighbor(p: Point, d: usize) -> Point {
    (p.0 + DIRECTIONS[d].0, p.1 + DIRECTIONS[d].1, p.2)
}
impl Synthesis {
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
#[derive(Serialize, Deserialize)]
struct ResolvedSocket {
    #[serde(flatten)]
    socket: Socket,
    half_items_per_second: i64,
}
#[derive(Serialize, Deserialize)]
struct ResolvedBuilding {
    building: Building,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    companion: Option<Building>,
    sockets: Vec<ResolvedSocket>,
}
fn export(model: &Synthesis, solution: &Solution) -> Vec<ResolvedBuilding> {
    model
        .choices
        .iter()
        .filter(|c| solution.value(c.enabled) == 1)
        .map(|c| ResolvedBuilding {
            building: c.building.clone(),
            companion: c.companion.clone(),
            sockets: c
                .sockets
                .iter()
                .map(|&(socket, amount)| ResolvedSocket {
                    socket,
                    half_items_per_second: solution.value(amount),
                })
                .collect(),
        })
        .collect()
}
pub fn run() -> Result<()> {
    let args = Args::parse();
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
    if let Some(path) = &args.hint {
        let hint: Vec<ResolvedBuilding> = serde_json::from_slice(&fs::read(path)?)?;
        for candidate in &model.choices {
            let matched = hint.iter().find(|h| {
                h.building == candidate.building
                    && h.companion == candidate.companion
                    && h.sockets.len() == candidate.sockets.len()
                    && h.sockets
                        .iter()
                        .zip(&candidate.sockets)
                        .all(|(a, (b, _))| a.socket == *b)
            });
            model
                .model
                .hint(candidate.enabled, i64::from(matched.is_some()));
            if let Some(h) = matched {
                for (a, (_, amount)) in h.sockets.iter().zip(&candidate.sockets) {
                    model.model.hint(*amount, a.half_items_per_second);
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
    let path = |suffix: &str| PathBuf::from(format!("{}{suffix}", args.output.display()));
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let selected = if solution.feasible() {
        export(&model, &solution)
    } else {
        Vec::new()
    };
    let rotators = selected
        .iter()
        .filter(|b| b.building.definition.starts_with("Rotator"))
        .count();
    let building_count: usize = selected
        .iter()
        .map(|b| 1 + usize::from(b.companion.is_some()))
        .sum();
    let occupied: usize = model
        .choices
        .iter()
        .filter(|c| solution.feasible() && solution.value(c.enabled) == 1)
        .map(|c| c.footprint.len())
        .sum();
    fs::write(
        path(".status.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"status":solution.status,"seconds":solution.seconds,"whole":args.whole,"jumps":args.jumps,"free_ports":args.free_ports,"width":args.width,"height":args.height,"layers":args.layers,"rotators":rotators,"rotator_limit":args.rotators,"buildings":building_count,"occupied_tiles":occupied,"variables":variables,"constraints":constraints}),
        )?,
    )?;
    if solution.feasible() {
        fs::write(path(".json"), serde_json::to_vec_pretty(&selected)?)?;
        let ports: Vec<_> = model
            .boundaries
            .iter()
            .filter(|b| solution.value(b.enabled) == 1)
            .map(|b| serde_json::json!({"position":b.position,"supply":b.supply,"direction":b.direction}))
            .collect();
        fs::write(path(".ports.json"), serde_json::to_vec_pretty(&ports)?)?;
        if args.native {
            native(&args, &selected, &model, &solution)?;
        }
    }
    eprintln!(
        "{:?}: {building_count} buildings, {occupied} occupied tiles, {rotators} rotators, {:.2}s",
        solution.status, solution.seconds
    );
    Ok(())
}

fn native(
    args: &Args,
    selected: &[ResolvedBuilding],
    model: &Synthesis,
    solution: &Solution,
) -> Result<()> {
    use shapez_blueprint::{
        Blueprint, BlueprintContent, BuildingBlueprint, Island, IslandBlueprint, encode_blueprint,
    };
    use shapez_runtime::{LabPaths, RunDirectory, run_suite};
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-weave")?;
    let mut buildings: Vec<_> = selected
        .iter()
        .flat_map(|s| std::iter::once(&s.building).chain(s.companion.iter()))
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
    let count = buildings.len();
    let mut island = Island::new("Foundation_2x2", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let bp = Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: None,
        source_format_version: None,
    };
    let blueprint = run.path.join("weave.spz2bp");
    fs::write(&blueprint, encode_blueprint(&bp)?)?;
    let inputs: Vec<_> = inputs.into_iter().map(|(p, _)| p).collect();
    let outputs: Vec<_> = outputs.into_iter().map(|(p, _)| p).collect();
    let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":"weave","blueprint":blueprint,"expected_buildings":count,"inputs":inputs,"outputs":outputs,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterWeaveExperiment"}}]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    eprintln!("Native weave: {}", run.path.display());
    run_suite(&paths, &suite_path, std::time::Duration::from_secs(180))
}
