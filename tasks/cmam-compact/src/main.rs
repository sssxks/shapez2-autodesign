use anyhow::Result;
use clap::{Parser, Subcommand};
use shapez_blueprint::Blueprint;
use shapez_runtime::{Host, LabPaths, RunDirectory, prepare_host, run_suite};
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
        suite: Option<PathBuf>,
        #[arg(long, default_value_t = 300)]
        timeout: u64,
    },
    Inspect {
        blueprint: PathBuf,
    },
    Decode {
        blueprint: PathBuf,
    },
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
            let suite = suite.unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/compact.json")
            });
            run_suite(&paths, &suite, Duration::from_secs(timeout))
        }
        Command::Inspect { blueprint } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&Blueprint::read(blueprint)?.summary())?
            );
            Ok(())
        }
        Command::Decode { blueprint } => {
            let run = RunDirectory::create(&paths.root, "decode")?;
            let input = run.snapshot(&blueprint, "source.spz2bp")?;
            let output = run.path.join("decoded.json");
            fs::write(
                &output,
                serde_json::to_vec_pretty(&Blueprint::read(input)?)?,
            )?;
            println!("{}", output.display());
            Ok(())
        }
        Command::Build {
            blueprint,
            rgb,
            previous,
        } => cmam_compact::run_compact(&paths, &blueprint, &rgb, previous.as_deref()),
        Command::Legacy {
            blueprint,
            splitter,
            rgb,
        } => cmam_compact::run_legacy(&paths, &blueprint, &splitter, &rgb),
    }
}
