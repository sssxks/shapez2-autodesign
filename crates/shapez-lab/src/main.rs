use anyhow::{Result,ensure};
use clap::{Parser,Subcommand};
use serde_json::{Value,json};
use shapez_lab::{Blueprint,Host,LabPaths,prepare_host};
use std::{path::PathBuf,time::Duration,fs};
#[derive(Parser)]struct Cli{#[command(subcommand)]command:Cmd}
#[derive(Subcommand)]enum Cmd{
 Inspect{blueprint:PathBuf},Decode{blueprint:PathBuf},Prepare,
 Batch{suite:PathBuf,#[arg(long,default_value_t=180)]timeout:u64},
 Probe{#[arg(long,default_value="capabilities")]operation:String,#[arg(long,default_value_t=180)]timeout:u64},
}
fn main()->Result<()>{
 let p=LabPaths::discover();fs::create_dir_all(&p.root)?;
 match Cli::parse().command{
  Cmd::Inspect{blueprint}=>println!("{}",serde_json::to_string_pretty(&Blueprint::read(blueprint)?.summary())?),
  Cmd::Decode{blueprint}=>{let bp=Blueprint::read(blueprint)?;let path=p.root.join("decoded.json");fs::write(&path,serde_json::to_vec_pretty(&bp.value)?)?;println!("{}",path.display());},
  Cmd::Prepare=>{prepare_host(&p)?;println!("{}",p.host().display());},
  Cmd::Probe{operation,timeout}=>{let t=Duration::from_secs(timeout);let mut h=Host::start(&p,t)?;println!("{}",h.request(&operation,json!({}),t)?);println!("artifacts: {}",h.run.display());h.shutdown()?;},
  Cmd::Batch{suite,timeout}=>{
   let suite:Value=serde_json::from_slice(&fs::read(suite)?)?;let cases=suite["cases"].as_array().ok_or_else(||anyhow::anyhow!("suite.cases must be an array"))?;
   let t=Duration::from_secs(timeout);let mut host=Host::start(&p,t)?;let mut results=vec![];
   for case in cases{let op=case["operation"].as_str().unwrap_or("blueprint");let result=host.request(op,case.clone(),t)?;println!("{}: {}",case["name"],result["ok"]);results.push(result);}
   let path=host.run.join("report.json");fs::write(&path,serde_json::to_vec_pretty(&json!({"cases":results}))?)?;host.shutdown()?;
   println!("{}",path.display());ensure!(results.iter().all(|r|r["ok"]==true),"One or more test cases failed");
  }
 }Ok(())
}
