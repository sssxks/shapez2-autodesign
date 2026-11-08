//! Command-line joint search, restart persistence and complete-blueprint export.
use shapez_solver::Solution;
/// Tile coordinates, directed states and rigid-cell terminal geometry.
mod geometry;
/// Joint placement and routing constraint construction.
mod joint;
/// Shared multi-commodity transport constraints.
mod network;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use shapez_solver::Limits;
use std::{fs, path::PathBuf, time::Instant};

#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
/// Optimization engine used to solve the joint model.
enum Backend {
    /// OR-Tools constraint programming with integer variables.
    CpSat,
    /// SCIP mixed-integer optimization.
    Scip,
}
#[derive(Clone, Copy, Debug, ValueEnum, Serialize)]
/// External socket arrangement used by search geometry.
enum Ports {
    /// North input and west, east and south outputs from the original layout.
    Original,
    /// West-facing input arrangement.
    West,
}
#[derive(Parser)]
#[command(about = "Native Rust placement and material-flow search")]
/// Command-line inputs and bounds for rigid-cell placement search.
pub struct SearchArgs {
    /// TOML file containing three layers of sixteen initial cells.
    seed: PathBuf,
    /// Destination for the completed building blueprint.
    output: PathBuf,
    #[arg(long, default_value_t = 300.0)]
    /// Positive solver time limit in seconds.
    seconds: f64,
    #[arg(long, default_value_t = 4)]
    /// Positive number of solver workers.
    workers: i32,
    #[arg(long, default_value_t = 1)]
    /// Maximum translation in either horizontal axis from each seed pose.
    radius: i32,
    #[arg(long)]
    /// Include poses rotated by 180 degrees around the seed footprint.
    half_turn: bool,
    #[arg(long)]
    /// Optional restart file written for a feasible solution before completeness checking.
    checkpoint: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "cp-sat")]
    /// Solver implementation used for this invocation.
    backend: Backend,
    #[arg(long, value_enum, default_value = "original")]
    /// Physical external interfaces required by the model.
    ports: Ports,
    #[arg(long)]
    /// Disable solver progress logging.
    quiet: bool,
    /// Maximize completed full-rate outputs, retaining a feasible incumbent.
    #[arg(long)]
    maximize_outputs: bool,
    #[arg(long)]
    /// Restart file whose flows are fixed and whose cell selections become hints.
    resume: Option<PathBuf>,
}
impl SearchArgs {
    /// Whether geometry should use the west-input socket arrangement.
    fn west(&self) -> bool {
        matches!(self.ports, Ports::West)
    }
}
/// Build and solve the model, optionally checkpoint it, and export only a complete 48-cell solution.
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
    /// Selected layer, seed-cell index and complete cell configuration.
    selected: Vec<(usize, usize, geometry::PackedCell)>,
    /// Material class, departure state, arrival state and integer flow for selected moves.
    flows: Vec<(u8, geometry::State, geometry::State, i64)>,
}

/// Human-authored placement search starting configuration.
#[derive(Deserialize)]
struct SearchSeed {
    /// Initial cell configurations grouped by factory layer.
    layers: Vec<Vec<geometry::PackedCell>>,
}
