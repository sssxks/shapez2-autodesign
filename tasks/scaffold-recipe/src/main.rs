use anyhow::Result;
use clap::{Parser, Subcommand};
use shapez_runtime::LabPaths;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Plan { target: String },
    Check,
}
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    match Cli::parse().command {
        Command::Plan { target } => scaffold_recipe::commands::run_recipe(&paths, &target),
        Command::Check => scaffold_recipe::commands::run_check(&paths),
    }
}
