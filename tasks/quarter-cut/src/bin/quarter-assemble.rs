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
    #[arg(long)]
    original: bool,
}
fn main() -> Result<()> {
    let args = Args::parse();
    let left: Vec<Building> = serde_json::from_slice(&fs::read(args.left)?)?;
    let right: Vec<Building> = serde_json::from_slice(&fs::read(args.right)?)?;
    let bp = if args.original {
        quarter_cut::assemble_original_halves(&left, &right)?
    } else {
        quarter_cut::assemble_banks(&left, &right)?
    };
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    let sockets = if args.original {
        quarter_cut::original_sockets()
    } else {
        quarter_cut::factory_sockets()
    };
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
    let output = run.path.join(if args.original {
        "quarter-cut-1x2-original.spz2bp"
    } else {
        "quarter-cut-1x2.spz2bp"
    });
    fs::write(&output, encode_blueprint(&bp)?)?;
    let mut harness = bp.clone();
    if let BlueprintContent::Islands(islands) = &mut harness.content {
        islands
            .entries
            .extend(shapez_blueprint::foundation_harness(&geometry, &sockets));
    }
    let test = run.path.join("harness.spz2bp");
    fs::write(&test, encode_blueprint(&harness)?)?;
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj".into()],
        cases: vec![
            shapez_runtime::case! {name:"factory",blueprint:test,ports:geometry.ports(&sockets),expected_buildings:count,
            expected_mergers:mergers,export_graph:true,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll".into(), type_name: ("QuarterFactoryExperiment").into() }},
            shapez_runtime::case! {name:"library-icon",operation:"inspect-blueprint",blueprint:output.clone(),expected_format_version:5,expected_library_warning:false,expected_icon:bp.icon.as_ref().unwrap().components.clone()},
        ],
    };
    println!("Factory: {}", output.display());
    run_suite(&paths, &suite, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}
