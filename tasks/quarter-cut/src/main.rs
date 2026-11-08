//! One task executable: solve, visualize, export and optionally validate.
mod native;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Solve both halves, assemble the factory and render its material flow in memory.
    Factory(quarter_cut::grid::FactoryArgs),
    /// Solve a configurable finite-domain region and export its blueprint.
    Grid(quarter_cut::grid::Args),
    /// Solve rigid-cell placement and shared routing.
    Search(quarter_cut::search::SearchArgs),
    /// Solve a primitive socket-flow region.
    Weave(quarter_cut::weave::Args),
    /// Validate an explicitly supplied game blueprint.
    Native { blueprint: PathBuf },
    /// Run one native cell or definition probe.
    Probe {
        #[arg(value_parser = ["cell", "mirror-cell", "middle-cell", "middle-cell-2", "jump-cell", "definitions", "capacity"])]
        mode: String,
    },
}
fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Factory(args) => {
            let solution = quarter_cut::grid::factory(&args)?;
            solution.export(&args.output)?;
            if args.native {
                native::validate(solution.blueprint()?)?;
            }
            Ok(())
        }
        Command::Grid(args) => quarter_cut::grid::run(args),
        Command::Search(args) => quarter_cut::search::run(args),
        Command::Weave(args) => quarter_cut::weave::run(args),
        Command::Native { blueprint } => {
            native::validate(shapez_blueprint::Blueprint::read(blueprint)?)
        }
        Command::Probe { mode } => {
            let paths = shapez_runtime::LabPaths::discover();
            let run = shapez_runtime::RunDirectory::create(&paths.root, "quarter-cut")?;
            native::probe(&paths, &run, &format!("--{mode}"))
        }
    }
}
