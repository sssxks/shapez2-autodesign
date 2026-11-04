use anyhow::Result;
use clap::{Parser, Subcommand};
use shapez_runtime::LabPaths;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Build {
        blueprint: PathBuf,
        #[arg(long)]
        rgb: PathBuf,
        #[arg(long)]
        previous: Option<PathBuf>,
    },
    Legacy {
        blueprint: PathBuf,
        #[arg(long)]
        splitter: PathBuf,
        #[arg(long)]
        rgb: PathBuf,
    },
}
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    match Cli::parse().command {
        Command::Build {
            blueprint,
            rgb,
            previous,
        } => cmam_compact::commands::run_compact(&paths, &blueprint, &rgb, previous.as_deref()),
        Command::Legacy {
            blueprint,
            splitter,
            rgb,
        } => cmam_compact::commands::run_legacy(&paths, &blueprint, &splitter, &rgb),
    }
}
