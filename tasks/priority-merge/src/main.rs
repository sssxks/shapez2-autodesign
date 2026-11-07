//! Generate a twelve-lane overflow-controlled priority merger on one foundation.
mod routing;
use anyhow::Result;
use shapez_blueprint::{
    Blueprint, BlueprintContent, Building, BuildingBlueprint, Configuration, Island,
    IslandBlueprint, Rotation, encode_blueprint,
};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, time::Duration};

/// One independently controlled lane, with local coordinates matching the circuit diagram.
fn cell(x: i32, y: i32) -> Vec<Building> {
    // Overflow feedback circuit adapted from Rhaversen's published single-lane design:
    // https://www.reddit.com/r/shapezio/comments/1l53i2c/input_priority_to_vortex/
    let specs = [
        (0, -3, 1, "ConstantSignalDefaultInternalVariant"),
        (0, -2, 1, "LogicGateIfInternalVariant"),
        (0, -1, 1, "LogicGateNotInternalVariant"),
        (0, 0, 0, "WireDefaultBridgeInternalVariant"),
        (1, -2, 3, "WireDefaultLeftInternalVariant"),
        (1, -1, 1, "WireDefaultForwardInternalVariant"),
        (1, 0, 0, "WireDefaultLeftInternalVariant"),
        (-2, -1, 1, "SplitterOverflowLInternalVariant"),
        (-2, 0, 1, "BeltDefaultForwardInternalVariant"),
        (-2, 1, 1, "BeltDefaultLeftInternalVariant"),
        (-1, -1, 0, "BeltDefaultLeftInternalVariantMirrored"),
        (-1, 0, 1, "BeltReaderDefaultInternalVariantMirrored"),
        (-1, 1, 1, "Merger3To1InternalVariant"),
        (0, 1, 2, "BeltFilterDefaultInternalVariant"),
    ];
    specs
        .into_iter()
        .map(|(dx, dy, r, t)| {
            let mut b = Building::new(t, x + dx, y + dy, 0, Rotation::new(r));
            if t.starts_with("Constant") {
                b.configuration = Some(Configuration(vec![5]));
            }
            b
        })
        .collect()
}

/// Build, validate in the native host, and publish the successful run.
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "priority-merge")?;
    let centers = [(6, 6), (13, 6), (6, 13), (13, 13)];
    let mut buildings = centers
        .into_iter()
        .flat_map(|(x, y)| cell(x, y))
        .collect::<Vec<_>>();
    routing::connect(&mut buildings, &centers)?;
    let layer = buildings.clone();
    for z in 1..3 {
        buildings.extend(layer.iter().cloned().map(|mut b| {
            b.layer = z;
            b
        }));
    }
    let count = buildings.len();
    let mut island = Island::new("Foundation_1x1", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let bp = Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: None,
        source_format_version: None,
    };
    let output = run.path.join("priority-12.spz2bp");
    fs::write(&output, encode_blueprint(&bp)?)?;
    fs::write(
        run.path.join("layout.json"),
        serde_json::to_vec_pretty(&bp)?,
    )?;
    let mut harness = bp.clone();
    if let BlueprintContent::Islands(islands) = &mut harness.content {
        for (x, y, r) in [(-1, 0, 0), (0, -1, 1), (1, 0, 0)] {
            islands
                .entries
                .push(Island::new("SpaceBelt_Forward", x, y, 0, Rotation::new(r)));
        }
    }
    let test_blueprint = run.path.join("harness.spz2bp");
    fs::write(&test_blueprint, encode_blueprint(&harness)?)?;
    let suite = serde_json::json!({"projects":["${REPO}/tasks/priority-merge/interop/PriorityTask.csproj"],"cases":[{
        "name":"priority-12","blueprint":test_blueprint,"expected_buildings":count,"export_graph":true,
        "experiment":{"assembly":"${LAB}/bin/PriorityTask/Release/netstandard2.1/ShapezLab.Priority.dll","type":"PriorityExperiment"}
    }]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    println!("Blueprint: {} ({count} buildings)", output.display());
    run_suite(&paths, &suite_path, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}
