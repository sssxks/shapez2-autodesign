//! Scaffold planning and isolated native recipe execution commands.
use anyhow::Result;
use clap::{Parser, Subcommand};
use shapez_runtime::{Host, LabPaths, prepare_host, run_suite_file};
use std::{path::PathBuf, time::Duration};

/// Command-line selection for planning and native validation.
#[derive(Parser)]
struct Cli {
    /// Operation to perform using the shared isolated host paths.
    #[command(subcommand)]
    command: Command,
}
/// Supported planner and native host operations.
#[derive(Subcommand)]
enum Command {
    /// Prepare the isolated native game host.
    Prepare,
    /// Send an argument-free operation to a native host.
    Probe {
        #[arg(long, default_value = "capabilities")]
        /// Native operation name, such as capabilities.
        operation: String,
        #[arg(long, default_value_t = 180)]
        /// Maximum time in seconds for the native operation.
        timeout: u64,
    },
    /// Execute a supplied suite or the generated recipe cases.
    Test {
        /// Optional TOML suite; omission uses the planner construction cases.
        suite: Option<PathBuf>,
        #[arg(long, default_value_t = 300)]
        /// Maximum time in seconds for the native operation.
        timeout: u64,
    },
    /// Construct and natively validate a recipe for one target.
    Plan {
        /// Target shape in the native quadrant-and-layer notation.
        target: String,
    },
    /// Validate all generated construction cases against the game.
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
            let response = host.request::<_, Box<serde_json::value::RawValue>>(
                &operation,
                shapez_runtime::EmptyArgs {},
                timeout,
            )?;
            println!("{}", serde_json::to_string(&response)?);
            println!("artifacts: {}", host.run.display());
            host.shutdown()?;
            response.into_result()?;
            Ok(())
        }
        Command::Test { suite, timeout } => match suite {
            Some(path) => run_suite_file(&paths, &path, Duration::from_secs(timeout)),
            None => shapez_runtime::run_suite(
                &paths,
                &scaffold_recipe::check_suite()?,
                Duration::from_secs(timeout),
            ),
        },
        Command::Plan { target } => scaffold_recipe::run_recipe(&paths, &target),
        Command::Check => scaffold_recipe::run_check(&paths),
    }
}
