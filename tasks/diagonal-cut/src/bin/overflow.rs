//! Correct the supplied overflow cutter's output quadrants.
#[path = "../overflow_design.rs"]
mod design;
use anyhow::Result;
use shapez_blueprint::{BlueprintContent, Island, Rotation, decode_blueprint, encode_blueprint};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, path::PathBuf, time::Duration};

fn main() -> Result<()> {
    let source = PathBuf::from(std::env::args_os().nth(1).expect("source blueprint"));
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "diagonal-overflow")?;
    // Keep the reference's layer-specific overflow lifts even if the library copy was replaced.
    let mut bp = decode_blueprint(include_str!("../../templates/overflow-reference.spz2bp"))?;
    let BlueprintContent::Islands(islands) = &mut bp.content else {
        anyhow::bail!("islands required")
    };
    let island = &mut islands.entries[0];
    island.x = 0;
    let buildings = design::layout(island.buildings.as_ref().expect("buildings"))?;
    island.buildings = Some(shapez_blueprint::BuildingBlueprint::new(buildings));
    let output = run.path.join("diagonal-overflow-corrected.spz2bp");
    fs::write(&output, encode_blueprint(&bp)?)?;
    let mut harness = bp.clone();
    if let BlueprintContent::Islands(islands) = &mut harness.content {
        for (x, y) in [(0, -1), (0, 1), (1, 1)] {
            islands
                .entries
                .push(Island::new("SpaceBelt_Forward", x, y, 0, Rotation::new(1)));
        }
    }
    let fixture = run.path.join("harness.spz2bp");
    fs::write(&fixture, encode_blueprint(&harness)?)?;
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/diagonal-cut/interop/DiagonalTask.csproj".into()],
        cases: vec![
            shapez_runtime::case! {name:"icon",operation:"inspect-blueprint",blueprint:output.clone(),expected_format_version:5,expected_library_warning:false},
            shapez_runtime::case! {name:"overflow",blueprint:fixture,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/DiagonalTask/Release/netstandard2.1/ShapezLab.Diagonal.dll".into(), type_name: ("OverflowExperiment").into() }},
        ],
    };
    run_suite(&paths, &suite, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    let destination = source.with_file_name(
        "通用对角切割(溢流·朝向修正) General Diagonal Cutter (Overflow Corrected).spz2bp",
    );
    fs::copy(&output, &destination)?;
    println!("{}", destination.display());
    Ok(())
}
