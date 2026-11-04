use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use shapez_lab::{Blueprint, Host, LabPaths, connect_cmam, prepare_host};
use std::{fs, path::PathBuf, time::Duration};
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}
#[derive(Subcommand)]
enum Cmd {
    Inspect {
        blueprint: PathBuf,
    },
    Decode {
        blueprint: PathBuf,
    },
    Prepare,
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
        Cmd::Connect {
            blueprint,
            splitter,
            rgb,
        } => {
            let report = connect_cmam(&blueprint, &splitter, &rgb, &p.root.join("adapter"))?;
            println!("{}", report["blueprint"]);
            println!("platforms: {}", report["generated_platforms"]);
        }
        Cmd::Inspect { blueprint } => println!(
            "{}",
            serde_json::to_string_pretty(&Blueprint::read(blueprint)?.summary())?
        ),
        Cmd::Decode { blueprint } => {
            let bp = Blueprint::read(blueprint)?;
            let path = p.root.join("decoded.json");
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
            println!("{}", h.request(&operation, json!({}), t)?);
            println!("artifacts: {}", h.run.display());
            h.shutdown()?;
        }
        Cmd::Batch { suite, timeout } => {
            let mut suite: Value = serde_json::from_slice(&fs::read(suite)?)?;
            expand_paths(&mut suite, &p);
            let cases = suite["cases"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("suite.cases must be an array"))?;
            let t = Duration::from_secs(timeout);
            p.build("LabMod")?;
            let mut host = Host::start(&p, t)?;
            let mut results = vec![];
            for case in cases {
                let op = case["operation"].as_str().unwrap_or("blueprint");
                let mut result = match host.request(op, case.clone(), t) {
                    Ok(result) => result,
                    Err(error) => {
                        results.push(json!({"name":case["name"],"passed":false,"host_error":error.to_string()}));
                        break;
                    }
                };
                result["name"] = case["name"].clone();
                result["passed"] = json!(
                    result["ok"].as_bool() == Some(case["expected_ok"].as_bool().unwrap_or(true))
                );
                println!(
                    "{}: {}",
                    case["name"],
                    if result["passed"] == true {
                        "PASS"
                    } else {
                        "FAIL"
                    }
                );
                if result["passed"] != true {
                    eprintln!("{}", result["error"]);
                }
                results.push(result);
            }
            let path = host.run.join("report.json");
            fs::write(&path, serde_json::to_vec_pretty(&json!({"cases":results}))?)?;
            host.shutdown()?;
            println!("{}", path.display());
            ensure!(
                results.len() == cases.len() && results.iter().all(|r| r["passed"] == true),
                "One or more test cases failed"
            );
        }
    }
    Ok(())
}

fn expand_paths(value: &mut Value, p: &LabPaths) {
    match value {
        Value::String(s) => {
            let library = PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
                .join("AppData/LocalLow/tobspr Games/shapez 2/blueprints");
            *s = s
                .replace("${LAB}", &p.root.to_string_lossy())
                .replace("${LIBRARY}", &library.to_string_lossy());
        }
        Value::Array(values) => {
            for v in values {
                expand_paths(v, p)
            }
        }
        Value::Object(values) => {
            for v in values.values_mut() {
                expand_paths(v, p)
            }
        }
        _ => {}
    }
}
