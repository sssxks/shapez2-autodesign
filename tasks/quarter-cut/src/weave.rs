//! Primitive socket-flow synthesis. No cell identity survives a merge.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::Serialize;
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
}
#[derive(Clone, Copy, Serialize)]
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
}
#[derive(Default)]
struct Synthesis {
    model: Model,
    occupied: BTreeMap<Point, Vec<Var>>,
    // Each directed edge has both physical-connection and material-flow equations.
    edges: BTreeMap<Edge, (Expr, Expr)>,
    choices: Vec<Choice>,
    rotators: Vec<Var>,
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
            conservation.term(amount, if input { 1 } else { -1 });
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
        });
    }
    fn boundary(
        &mut self,
        position: Point,
        direction: usize,
        shape: u8,
        amount: i64,
        supply: bool,
    ) {
        let (physical, flow) = self.edges.entry((position, direction, shape)).or_default();
        let sign = if supply { 1 } else { -1 };
        physical.constant += sign;
        flow.constant += sign * amount;
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
        // Occupied tiles first, then prefer fewer machines among equal footprints.
        let mut objective = Expr::default();
        for c in &self.choices {
            objective.term(c.enabled, 100 * c.footprint.len() as i64 + 1);
        }
        self.model.minimize(objective);
    }
}
fn build(args: &Args) -> Synthesis {
    let mut result = Synthesis::default();
    for z in 0..args.layers {
        for y in 0..8 {
            for x in 0..args.width {
                let p = (x, y, z);
                for r in 0..4 {
                    for shape in [1, 8] {
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
                }
            }
        }
    }
    // Two adjacent groups of second-stage cutters, each delivering four quarters.
    for y in 0..8 {
        result.boundary(
            (-1, y, 0),
            0,
            if y % 4 == 0 || y % 4 == 3 { 8 } else { 1 },
            1,
            true,
        );
    }
    for y in [1, 5] {
        result.boundary((args.width - 1, y, 0), 0, 1, 4, false);
    }
    result.finish(args.rotators);
    result
}
#[derive(Serialize)]
struct ResolvedSocket {
    #[serde(flatten)]
    socket: Socket,
    half_items_per_second: i64,
}
#[derive(Serialize)]
struct ResolvedBuilding {
    building: Building,
    sockets: Vec<ResolvedSocket>,
}
fn export(model: &Synthesis, solution: &Solution) -> Vec<ResolvedBuilding> {
    model
        .choices
        .iter()
        .filter(|c| solution.value(c.enabled) == 1)
        .map(|c| ResolvedBuilding {
            building: c.building.clone(),
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
        args.width > 0 && (1..=3).contains(&args.layers) && args.rotators >= 0 && args.seconds > 0.,
        "Invalid synthesis dimensions or limits"
    );
    let model = build(&args);
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
    fs::write(
        path(".status.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"status":solution.status,"seconds":solution.seconds,"width":args.width,"layers":args.layers,"rotators":rotators,"rotator_limit":args.rotators,"buildings":selected.len(),"variables":variables,"constraints":constraints}),
        )?,
    )?;
    if solution.feasible() {
        fs::write(path(".json"), serde_json::to_vec_pretty(&selected)?)?;
        if args.native {
            native(&args, &selected)?;
        }
    }
    eprintln!(
        "{:?}: {} buildings, {rotators} quarter rotators, {:.2}s",
        solution.status,
        selected.len(),
        solution.seconds
    );
    Ok(())
}

fn native(args: &Args, selected: &[ResolvedBuilding]) -> Result<()> {
    use shapez_blueprint::{
        Blueprint, BlueprintContent, BuildingBlueprint, Island, IslandBlueprint, encode_blueprint,
    };
    use shapez_runtime::{LabPaths, RunDirectory, run_suite};
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-weave")?;
    let mut buildings: Vec<_> = selected
        .iter()
        .map(|s| {
            let mut b = s.building.clone();
            b.x += 11;
            b.y += 4;
            b
        })
        .collect();
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
    let inputs = [(7, 6, 0), (7, 10, 0)];
    let outputs = [(11 + args.width, 5, 0), (11 + args.width, 9, 0)];
    for (x, y, z) in inputs.into_iter().chain(outputs) {
        buildings.push(Building::new(
            "BeltDefaultForwardInternalVariant",
            x,
            y,
            z,
            Rotation::new(0),
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
    let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":"weave","blueprint":blueprint,"expected_buildings":count,"inputs":inputs,"outputs":outputs,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterWeaveExperiment"}}]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    eprintln!("Native weave: {}", run.path.display());
    run_suite(&paths, &suite_path, std::time::Duration::from_secs(180))
}
