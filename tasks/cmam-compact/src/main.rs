use anyhow::Result;
use clap::{Parser, Subcommand};
use shapez_blueprint::{Blueprint, encode_blueprint};
use shapez_runtime::{Host, LabPaths, RunDirectory, prepare_host, run_suite_file};
use std::{fs, path::PathBuf, time::Duration};

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
        suite: PathBuf,
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    Inspect {
        blueprint: PathBuf,
    },

    /// Rewrite an existing blueprint using the current format and typed model.
    Reencode {
        blueprint: PathBuf,
    },
    Build {
        blueprint: PathBuf,
        #[arg(long)]
        rgb: PathBuf,
        #[arg(long)]
        previous: Option<PathBuf>,
        #[arg(long)]
        native: bool,
        #[arg(long = "target", requires = "native")]
        targets: Vec<String>,
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
            fs::write(&output, encode_blueprint(&model)?)?;
            println!("{}", output.display());
            Ok(())
        }
        Command::Build {
            blueprint,
            rgb,
            previous,
            native,
            targets,
        } => cmam_compact::run_compact(
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
        } => cmam_compact::run_legacy(&paths, &blueprint, &splitter, &rgb),
    }
}
