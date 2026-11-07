//! Correct the supplied overflow cutter's output quadrants.
use anyhow::{Result, ensure};
use shapez_blueprint::{BlueprintContent, Island, Rotation, decode_blueprint, encode_blueprint};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, path::PathBuf, time::Duration};

fn main() -> Result<()> {
    let source = PathBuf::from(std::env::args_os().nth(1).expect("source blueprint"));
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "diagonal-overflow")?;
    let mut bp = decode_blueprint(&fs::read_to_string(&source)?)?;
    let BlueprintContent::Islands(islands) = &mut bp.content else {
        anyhow::bail!("islands required")
    };
    let island = &mut islands.entries[0];
    island.x = 0;
    let mut changed = 0;
    for b in &mut island.buildings.as_mut().expect("buildings").entries {
        if b.y == 16 && [9, 10, 11, 28, 29, 30].contains(&b.x) {
            ensure!(
                b.definition == "BeltDefaultForwardInternalVariant",
                "expected straight belt"
            );
            b.definition = "RotatorOneQuadInternalVariant".into();
            changed += 1;
        }
    }
    ensure!(changed == 18, "expected 18 output rotators, got {changed}");
    for b in &mut island.buildings.as_mut().expect("buildings").entries {
        if [7, 32].contains(&b.x) && [7, 9, 11, 13].contains(&b.y) {
            ensure!(
                b.definition == "BeltDefaultForwardInternalVariant",
                "parallel output belt"
            );
            b.definition = "RotatorOneQuadInternalVariant".into();
        }
    }
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
    let suite = serde_json::json!({"projects":["${REPO}/tasks/diagonal-cut/interop/DiagonalTask.csproj"],"cases":[
        {"name":"icon","operation":"inspect-blueprint","blueprint":output,"expected_format_version":5,"expected_library_warning":false},
        {"name":"overflow","blueprint":fixture,"experiment":{"assembly":"${LAB}/bin/DiagonalTask/Release/netstandard2.1/ShapezLab.Diagonal.dll","type":"OverflowExperiment"}}
    ]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    run_suite(&paths, &suite_path, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    let destination = source.with_file_name(
        "通用对角切割(溢流·朝向修正) General Diagonal Cutter (Overflow Corrected).spz2bp",
    );
    fs::copy(&output, &destination)?;
    println!("{}", destination.display());
    Ok(())
}
