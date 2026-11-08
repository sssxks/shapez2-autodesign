//! Adapter build commands and native-host diagnostics.
use anyhow::Result;
use clap::{Parser, Subcommand};
use shapez_blueprint::Blueprint;
use shapez_runtime::{Host, LabPaths, RunDirectory, prepare_host, run_suite_file};
use std::{path::PathBuf, time::Duration};

#[derive(Parser)]
/// Top-level command-line dispatch for adapter construction and native diagnostics.
struct Cli {
    #[command(subcommand)]
    /// Requested build or diagnostic operation.
    command: Command,
}
#[derive(Subcommand)]
/// Supported adapter workflows and host inspection commands.
enum Command {
    /// Build and prepare the native game host.
    Prepare,
    /// Run one host operation and print its response and artifact location.
    Probe {
        #[arg(long, default_value = "capabilities")]
        /// Host protocol operation to invoke with empty arguments.
        operation: String,
        #[arg(long, default_value_t = 180)]
        /// Maximum wait in seconds for the host operation or suite.
        timeout: u64,
    },
    /// Execute a native acceptance suite from disk.
    Test {
        /// Suite specification to execute.
        suite: PathBuf,
        #[arg(long, default_value_t = 300)]
        /// Maximum wait in seconds for the host operation or suite.
        timeout: u64,
    },
    /// Print a typed summary of an existing blueprint.
    Inspect {
        /// Source blueprint read by this operation.
        blueprint: PathBuf,
    },

    /// Rewrite an existing blueprint using the current format and typed model.
    Reencode {
        /// Source blueprint read by this operation.
        blueprint: PathBuf,
    },
    /// Construct the compact adapter and optionally run native acceptance.
    Build {
        /// Source blueprint read by this operation.
        blueprint: PathBuf,
        #[arg(long)]
        /// RGB blueprint used as a construction input.
        rgb: PathBuf,
        #[arg(long)]
        /// Optional previous adapter blueprint for comparison.
        previous: Option<PathBuf>,
        #[arg(long)]
        /// Run native validation after constructing the adapter.
        native: bool,
        #[arg(long = "target", requires = "native")]
        /// Native validation targets selected by the caller.
        targets: Vec<String>,
    },
    /// Construct the legacy adapter from explicit component blueprints.
    Legacy {
        /// Source blueprint read by this operation.
        blueprint: PathBuf,
        #[arg(long)]
        /// Splitter blueprint used by legacy construction.
        splitter: PathBuf,
        #[arg(long)]
        /// RGB blueprint used as a construction input.
        rgb: PathBuf,
    },
}
/// Dispatch the requested operation and propagate filesystem or host failures.
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
        Command::Test { suite, timeout } => {
            run_suite_file(&paths, &suite, Duration::from_secs(timeout))
        }
        Command::Inspect { blueprint } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&Blueprint::read(blueprint)?.summary())?
            );
            Ok(())
        }
        Command::Reencode { blueprint } => {
            let run = RunDirectory::create(&paths.root, "blueprint-reencode")?;
            let input = run.snapshot(&blueprint, "source.spz2bp")?;
            let model = Blueprint::read(input)?;
            let output = run.path.join(
                blueprint
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Blueprint filename required"))?,
            );
            model.write(&output)?;
            println!("{}", output.display());
            Ok(())
        }
        Command::Build {
            blueprint,
            rgb,
            previous,
            native,
            targets,
        } => cmam_adapter::run_compact(
            &paths,
            &blueprint,
            &rgb,
            previous.as_deref(),
            native,
            &targets,
        ),
        Command::Legacy {
            blueprint,
            splitter,
            rgb,
        } => cmam_adapter::run_legacy(&paths, &blueprint, &splitter, &rgb),
    }
}
