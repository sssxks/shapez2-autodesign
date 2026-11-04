use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use shapez_lab::{Blueprint, Host, LabPaths, compact_cmam, connect_cmam, prepare_host};
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
        Cmd::RecipeCheck => {
            let dir = p.root.join("recipe-check");
            fs::create_dir_all(&dir)?;
            let mut cases = Vec::new();
            let symbols = ["--", "P-", "Cu", "cw", "cr"];
            for n in 1..625 {
                let mut k = n;
                let mut target = String::new();
                for _ in 0..4 {
                    target.push_str(symbols[k % 5]);
                    k /= 5;
                }
                let path = dir.join(format!("{n}.json"));
                fs::write(
                    &path,
                    serde_json::to_vec_pretty(&shapez_lab::plan_recipe(&target)?)?,
                )?;
                cases.push(json!({"name":target,"operation":"recipe","recipe":path}));
            }
            let path = dir.join("suite.json");
            fs::write(&path, serde_json::to_vec_pretty(&json!({"cases":cases}))?)?;
            println!("{}", path.display());
        }
        Cmd::Recipe { target } => {
            let recipe = shapez_lab::plan_recipe(&target)?;
            let path = p.root.join("recipe.json");
            fs::write(&path, serde_json::to_vec_pretty(&recipe)?)?;
            p.build("LabMod")?;
            let t = Duration::from_secs(180);
            let mut h = Host::start(&p, t)?;
            let report = h.request("recipe", json!({"recipe":path}), t)?;
            let mut readable = format!(
                "Target: {target}\nVerified: {}\n",
                report["result"]["verified"]
            );
            if let Some(trace) = report["result"]["trace"].as_array() {
                for step in trace {
                    let id = step["id"].as_u64().unwrap() as usize;
                    readable.push_str(&format!(
                        "{} {} inputs={} output={} port={}\n",
                        id,
                        step["op"],
                        recipe["nodes"][id]["inputs"],
                        step["expected"],
                        step["selected_port"]
                    ));
                }
            }
            fs::write(h.run.join("recipe.txt"), readable)?;
            println!(
                "verified={} steps={}",
                report["result"]["verified"], report["result"]["steps"]
            );
            println!("artifacts: {}", h.run.display());
            h.shutdown()?;
            ensure!(
                report["ok"] == true && report["result"]["verified"] == true,
                "Candidate recipe did not pass native operations"
            );
        }
        Cmd::Compact { blueprint, rgb } => {
            let report = compact_cmam(&blueprint, &rgb, &p.root.join("compact"))?;
            println!("{}", report["summary"]);
            println!("space nodes: {}", report["space_nodes"]);
        }
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
                        && result["result"]["verified"].as_bool() != Some(false)
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
