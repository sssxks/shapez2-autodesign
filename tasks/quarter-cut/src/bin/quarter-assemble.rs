//! Assemble bank solutions and run native end-to-end acceptance.
use anyhow::Result;
use clap::Parser;
use shapez_blueprint::{BlueprintContent, Building, encode_blueprint};
use shapez_model::FoundationGeometry;
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, path::PathBuf, time::Duration};
#[derive(Parser)]
struct Args {
    left: PathBuf,
    right: PathBuf,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let left: Vec<Building> = serde_json::from_slice(&fs::read(args.left)?)?;
    let right: Vec<Building> = serde_json::from_slice(&fs::read(args.right)?)?;
    let bp = quarter_cut::assemble_banks(&left, &right)?;
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    let sockets = quarter_cut::factory_sockets();
    let geometry = FoundationGeometry::strip(2, 3);
    let buildings = bp.islands()[0].buildings();
    fs::write(
        run.path.join("buildings.json"),
        serde_json::to_vec_pretty(buildings)?,
    )?;
    let count = buildings.len();
    let mergers = buildings
        .iter()
        .filter(|b| b.definition.starts_with("Merger"))
        .count();
    let output = run.path.join("quarter-cut-1x2.spz2bp");
    fs::write(&output, encode_blueprint(&bp)?)?;
    let mut harness = bp.clone();
    if let BlueprintContent::Islands(islands) = &mut harness.content {
        islands
            .entries
            .extend(shapez_blueprint::foundation_harness(&geometry, &sockets));
    }
    let test = run.path.join("harness.spz2bp");
    fs::write(&test, encode_blueprint(&harness)?)?;
    let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[
        {"name":"factory","blueprint":test,"ports":geometry.ports(&sockets),"expected_buildings":count,
        "expected_mergers":mergers,"export_graph":true,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterFactoryExperiment"}},
        {"name":"library-icon","operation":"inspect-blueprint","blueprint":output,"expected_format_version":5,"expected_library_warning":false,"expected_icon":bp.icon.as_ref().unwrap().components}]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    println!("Factory: {}", output.display());
    run_suite(&paths, &suite_path, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}
