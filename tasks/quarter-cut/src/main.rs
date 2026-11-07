//! Resolve native cutter and rotator definitions in an isolated lab world.
use anyhow::Result;
use shapez_blueprint::{
    Blueprint, BlueprintContent, Island, IslandBlueprint, Rotation, encode_blueprint,
};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, time::Duration};

/// Run task-owned discovery without publishing an unfinished factory.
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    let bp = Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![Island::new(
            "Foundation_1x2",
            0,
            0,
            0,
            Rotation::new(0),
        )])),
        icon: None,
        source_format_version: None,
    };
    let blueprint = run.path.join("definition-probe.spz2bp");
    fs::write(&blueprint, encode_blueprint(&bp)?)?;
    let suite = serde_json::json!({
        "projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],
        "cases":[{"name":"definitions","blueprint":blueprint,"expected_buildings":0,
        "experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterCutDefinitions"}}]
    });
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    println!("Discovery artifacts: {}", run.path.display());
    run_suite(&paths, &suite_path, Duration::from_secs(180))
}
