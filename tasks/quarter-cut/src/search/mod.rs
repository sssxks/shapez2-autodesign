mod geometry;
mod joint;
mod network;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::Serialize;
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
    #[arg(long, default_value_t = 2)]
    chunks: i32,
    #[arg(long)]
    witness: Option<PathBuf>,
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
    warm_start: Option<PathBuf>,
}
impl SearchArgs {
    fn west(&self) -> bool {
        matches!(self.ports, Ports::West)
    }
}
pub fn run() -> Result<()> {
    let args = SearchArgs::parse();
    anyhow::ensure!(
        (2..=3).contains(&args.chunks) && args.radius >= 0 && args.workers > 0 && args.seconds > 0.,
        "Invalid search limits"
    );
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    let seed: Vec<Vec<geometry::PackedCell>> = serde_json::from_slice(&fs::read(&args.seed)?)?;
    let witness = if let Some(path) = &args.witness {
        let rows: Vec<(u8, geometry::State, geometry::State, i64)> =
            serde_json::from_slice(&fs::read(path)?)?;
        Some(
            rows.into_iter()
                .map(|(c, a, b, f)| ((c, a, b), f))
                .collect::<network::Witness>(),
        )
    } else {
        None
    };
    let start = Instant::now();
    let mut built = joint::build(&seed, &args, witness.as_ref())?;
    if let Some(prefix) = &args.warm_start {
        let cells: Vec<(usize, usize, geometry::PackedCell)> =
            serde_json::from_slice(&fs::read(format!("{}.selected.json", prefix.display()))?)?;
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
        let flows: Vec<(u8, geometry::State, geometry::State, i64)> =
            serde_json::from_slice(&fs::read(format!("{}.flows.json", prefix.display()))?)?;
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
    let output = |suffix: &str| PathBuf::from(format!("{}{suffix}", args.output.display()));
    let completed = if solution.feasible() {
        built
            .choices
            .iter()
            .filter(|c| solution.value(c.enabled) == 1)
            .count()
    } else {
        0
    };
    fs::write(
        output(".status.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"status":solution.status,"seconds":solution.seconds,"completed_outputs":completed,"backend":solution.backend,"variables":variables,"constraints":constraints,"seed":args.seed,"radius":args.radius,"half_turn":args.half_turn,"chunks":args.chunks,"ports":args.ports}),
        )?,
    )?;
    eprintln!("Result: {:?} in {:.3}s", solution.status, solution.seconds);
    if solution.feasible() {
        let flows: Vec<_> = built
            .network
            .arcs
            .iter()
            .filter(|a| solution.value(a.selected) != 0)
            .map(|a| (a.commodity, a.from, a.to, solution.value(a.amount)))
            .collect();
        fs::write(output(".flows.json"), serde_json::to_vec(&flows)?)?;
        let partial: Vec<_> = built
            .choices
            .iter()
            .filter(|c| solution.value(c.enabled) == 1)
            .map(|c| (c.layer, c.index, &c.cell))
            .collect();
        fs::write(
            output(".selected.json"),
            serde_json::to_vec_pretty(&partial)?,
        )?;
        eprintln!("Completed full-rate outputs: {completed}/48");
        fs::write(
            output(".buildings.json"),
            serde_json::to_vec_pretty(&built.buildings(&solution)?)?,
        )?;
        if completed != 48 {
            return Ok(());
        }
        let (cells, routes) = built.export(&solution)?;
        fs::write(output(".json"), serde_json::to_vec_pretty(&cells)?)?;
        fs::write(output(".routes.json"), serde_json::to_vec(&routes)?)?;
        eprintln!("Exported {} complete routes", routes.len());
    }
    Ok(())
}
