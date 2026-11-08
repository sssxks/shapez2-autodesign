use shapez_solver::Solution;
mod geometry;
mod joint;
mod network;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use shapez_solver::Limits;
use std::{fs, path::PathBuf, time::Instant};

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
enum Backend {
    CpSat,
    Scip,
}
#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
enum Ports {
    Original,
    West,
}
#[derive(Parser)]
#[command(about = "Native Rust placement and material-flow search")]
pub struct SearchArgs {
    seed: PathBuf,
    output: PathBuf,
    #[arg(long, default_value_t = 300.0)]
    seconds: f64,
    #[arg(long, default_value_t = 4)]
    workers: i32,
    #[arg(long, default_value_t = 1)]
    radius: i32,
    #[arg(long)]
    half_turn: bool,
    #[arg(long)]
    checkpoint: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "cp-sat")]
    backend: Backend,
    #[arg(long, value_enum, default_value = "original")]
    ports: Ports,
    #[arg(long)]
    quiet: bool,
    /// Maximize completed full-rate outputs, retaining a feasible incumbent.
    #[arg(long)]
    maximize_outputs: bool,
    #[arg(long)]
    resume: Option<PathBuf>,
}
impl SearchArgs {
    fn west(&self) -> bool {
        matches!(self.ports, Ports::West)
    }
}
pub fn run(args: SearchArgs) -> Result<()> {
    anyhow::ensure!(
        args.radius >= 0 && args.workers > 0 && args.seconds > 0.,
        "Invalid search limits"
    );
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let seed = toml::from_str::<SearchSeed>(&fs::read_to_string(&args.seed)?)?.layers;
    let checkpoint: Option<SearchCheckpoint> = args
        .resume
        .as_ref()
        .map(|path| -> Result<_> { crate::checkpoint::load(path, "joint") })
        .transpose()?;
    let witness = checkpoint.as_ref().map(|c| {
        c.flows
            .iter()
            .copied()
            .map(|(commodity, a, b, flow)| ((commodity, a, b), flow))
            .collect::<network::Witness>()
    });
    let start = Instant::now();
    let mut built = joint::build(&seed, &args, witness.as_ref())?;
    if let Some(checkpoint) = &checkpoint {
        let cells = &checkpoint.selected;
        for choice in &built.choices {
            let value = cells.iter().any(|(layer, index, c)| {
                *layer == choice.layer
                    && *index == choice.index
                    && c.x == choice.cell.x
                    && c.y == choice.cell.y
                    && c.r == choice.cell.r
                    && c.mirror == choice.cell.mirror
                    && c.out == choice.cell.out
                    && c.split == choice.cell.split
            });
            built.network.model.hint(choice.enabled, i64::from(value));
        }
        let flows = checkpoint.flows.clone();
        let flows: network::Witness = flows
            .into_iter()
            .map(|(c, a, b, f)| ((c, a, b), f))
            .collect();
        for arc in &built.network.arcs {
            let amount = flows
                .get(&(arc.commodity, arc.from, arc.to))
                .copied()
                .unwrap_or(0);
            built
                .network
                .model
                .hint(arc.selected, i64::from(amount != 0));
            built.network.model.hint(arc.amount, amount);
        }
    }
    let (variables, constraints) = built.network.model.counts();
    eprintln!(
        "Rust model: {variables} variables, {constraints} constraints, {} arcs, built in {:.2}s",
        built.network.arcs.len(),
        start.elapsed().as_secs_f64()
    );
    let limits = Limits {
        seconds: args.seconds,
        workers: args.workers,
        log: !args.quiet,
    };
    let solution = match args.backend {
        Backend::CpSat => built.network.model.solve_cp_sat(&limits)?,
        Backend::Scip => built.network.model.solve_scip(&limits)?,
    };
    let status = solution.status();
    let (Solution::Optimal(solution) | Solution::Feasible(solution)) = solution else {
        anyhow::bail!("No feasible joint solution: {:?}", status);
    };
    if let Some(path) = &args.checkpoint {
        let checkpoint = SearchCheckpoint {
            selected: built
                .choices
                .iter()
                .filter(|c| solution.value(c.enabled) == 1)
                .map(|c| (c.layer, c.index, c.cell.clone()))
                .collect(),
            flows: built
                .network
                .arcs
                .iter()
                .filter(|a| solution.value(a.selected) != 0)
                .map(|a| (a.commodity, a.from, a.to, solution.value(a.amount)))
                .collect(),
        };
        crate::checkpoint::save(path, "joint", &checkpoint)?;
    }
    let completed = built
        .choices
        .iter()
        .filter(|c| solution.value(c.enabled) == 1)
        .count();
    anyhow::ensure!(
        completed == 48,
        "Incomplete joint solution: {completed}/48 cells; checkpoint retained if requested"
    );
    let buildings = built.buildings(&solution)?;
    crate::export_buildings(&buildings, &args.output)
}

/// Explicit solver restart state; normal execution never reads its own export.
#[derive(Serialize, Deserialize)]
struct SearchCheckpoint {
    selected: Vec<(usize, usize, geometry::PackedCell)>,
    flows: Vec<(u8, geometry::State, geometry::State, i64)>,
}

/// Human-authored placement search starting configuration.
#[derive(Deserialize)]
struct SearchSeed {
    layers: Vec<Vec<geometry::PackedCell>>,
}
