use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
use shapez_core::Blueprint;
use shapez_runtime::{Host, LabPaths, prepare_host};
use shapez_runtime::{artifacts::RunDirectory, suite::run_suite};
use std::{fs, path::PathBuf, time::Duration};
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}
#[derive(Subcommand)]
enum Cmd {
    RecipeCheck,
    Recipe {
        target: String,
    },
    Inspect {
        blueprint: PathBuf,
    },
    Decode {
        blueprint: PathBuf,
    },
    Prepare,
    Compact {
        blueprint: PathBuf,
        #[arg(long)]
        rgb: PathBuf,
        #[arg(long)]
        previous: Option<PathBuf>,
    },
    Connect {
        blueprint: PathBuf,
        #[arg(long)]
        splitter: PathBuf,
        #[arg(long)]
        rgb: PathBuf,
    },
    Batch {
        suite: PathBuf,
        #[arg(long, default_value_t = 180)]
        timeout: u64,
    },
    Probe {
        #[arg(long, default_value = "capabilities")]
        operation: String,
        #[arg(long, default_value_t = 180)]
        timeout: u64,
    },
}
fn main() -> Result<()> {
    let p = LabPaths::discover();
    fs::create_dir_all(&p.root)?;
    match Cli::parse().command {
        Cmd::RecipeCheck => scaffold_recipe::commands::run_check(&p)?,
        Cmd::Recipe { target } => scaffold_recipe::commands::run_recipe(&p, &target)?,
        Cmd::Compact {
            blueprint,
            rgb,
            previous,
        } => cmam_compact::commands::run_compact(&p, &blueprint, &rgb, previous.as_deref())?,
        Cmd::Connect {
            blueprint,
            splitter,
            rgb,
        } => cmam_compact::commands::run_legacy(&p, &blueprint, &splitter, &rgb)?,
        Cmd::Inspect { blueprint } => println!(
            "{}",
            serde_json::to_string_pretty(&Blueprint::read(blueprint)?.summary())?
        ),
        Cmd::Decode { blueprint } => {
            let bp = Blueprint::read(&blueprint)?;
            let run = RunDirectory::create(&p.root, "decode")?;
            run.snapshot(&blueprint, "source.spz2bp")?;
            let path = run.path.join("decoded.json");
            fs::write(&path, serde_json::to_vec_pretty(&bp.value)?)?;
            println!("{}", path.display());
        }
        Cmd::Prepare => {
            prepare_host(&p)?;
            println!("{}", p.host().display());
        }
        Cmd::Probe { operation, timeout } => {
            p.build("LabMod")?;
            let t = Duration::from_secs(timeout);
            let mut h = Host::start(&p, t)?;
            println!(
                "{}",
                serde_json::to_string(&h.request::<_, serde_json::Value>(
                    &operation,
                    json!({}),
                    t
                )?)?
            );
            println!("artifacts: {}", h.run.display());
            h.shutdown()?;
        }
        Cmd::Batch { suite, timeout } => run_suite(&p, &suite, Duration::from_secs(timeout))?,
    }
    Ok(())
}
