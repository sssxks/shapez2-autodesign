//! One task executable: solve, visualize, export and optionally validate.
mod layout;
mod native;
use anyhow::Result;
use clap::{Parser, Subcommand};
use quarter_cut::design;
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
    /// Generate and validate the legacy three-chunk layout.
    Legacy,
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
        Command::Legacy => {
            use shapez_blueprint::{
                Blueprint, BlueprintContent, BuildingBlueprint, Island, IslandBlueprint, Rotation,
            };
            let mut island = Island::new("Foundation_1x3", 1, 0, 0, Rotation::new(0));
            island.buildings = Some(BuildingBlueprint::new(
                layout::build()?
                    .into_iter()
                    .map(|mut b| {
                        b.x -= 20;
                        b
                    })
                    .collect(),
            ));
            native::validate(Blueprint {
                game_version: 1138,
                content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
                icon: None,
                source_format_version: None,
            })
        }
        Command::Probe { mode } => {
            let paths = shapez_runtime::LabPaths::discover();
            let run = shapez_runtime::RunDirectory::create(&paths.root, "quarter-cut")?;
            native::probe(&paths, &run, &format!("--{mode}"))
        }
    }
}
