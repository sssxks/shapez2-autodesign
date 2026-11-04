use anyhow::Result;
use clap::{Parser, Subcommand};
use shapez_runtime::{Host, LabPaths, RunDirectory, prepare_host, run_suite};
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Prepare,
    Probe {
        #[arg(long, default_value = "capabilities")]
        operation: String,
        #[arg(long, default_value_t = 180)]
        timeout: u64,
    },
    Test {
        suite: Option<PathBuf>,
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    Plan {
        target: String,
    },
    Check,
}
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    match Cli::parse().command {
        Command::Prepare => prepare_host(&paths),
        Command::Probe { operation, timeout } => {
            paths.build("LabMod")?;
            let timeout = Duration::from_secs(timeout);
            let mut host = Host::start(&paths, timeout)?;
            let response =
                host.request::<_, serde_json::Value>(&operation, serde_json::json!({}), timeout)?;
            println!("{}", serde_json::to_string(&response)?);
            println!("artifacts: {}", host.run.display());
            host.shutdown()?;
            response.into_result()?;
            Ok(())
        }
        Command::Test { suite, timeout } => {
            let suite = match suite {
                Some(path) => path,
                None => RunDirectory::latest(&paths.root, "scaffold-recipe-check")?
                    .path
                    .join("suite.json"),
            };
            run_suite(&paths, &suite, Duration::from_secs(timeout))
        }
        Command::Plan { target } => scaffold_recipe::run_recipe(&paths, &target),
        Command::Check => scaffold_recipe::run_check(&paths),
    }
}
