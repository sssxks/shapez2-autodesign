//! Generate a twelve-lane overflow-controlled priority merger on one foundation.
mod design;
mod routing;
use anyhow::Result;
use design::{DESIGNS, Design};
use shapez_blueprint::{
    Blueprint, BlueprintContent, Building, BuildingBlueprint, Configuration, Island,
    IslandBlueprint, Rotation, encode_blueprint,
};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, time::Duration};

/// One independently controlled lane, with local coordinates matching the circuit diagram.
fn cell() -> shapez_blueprint::Cell {
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
    let buildings = specs
        .into_iter()
        .map(|(dx, dy, r, t)| {
            let mut b = Building::new(t, dx, dy, 0, Rotation::new(r));
            if t.starts_with("Constant") {
                b.configuration = Some(Configuration(vec![5]));
            }
            b
        })
        .collect();
    use shapez_blueprint::{Cell, CellPort};
    Cell::new(
        buildings,
        [],
        [
            (
                "primary",
                CellPort {
                    position: (-2, -2, 0),
                    direction: 1,
                },
            ),
            (
                "secondary",
                CellPort {
                    position: (1, 1, 0),
                    direction: 2,
                },
            ),
            (
                "output",
                CellPort {
                    position: (-1, 2, 0),
                    direction: 1,
                },
            ),
        ],
    )
}

/// Build one independently routed interface with three identical building layers.
fn build(design: &Design) -> Result<Blueprint> {
    let centers = if matches!(design.secondary, design::Side::South) {
        [(5, 6), (12, 6), (5, 12), (12, 12)]
    } else {
        [(6, 6), (13, 6), (6, 13), (13, 13)]
    };
    let cells: Vec<_> = centers
        .into_iter()
        .map(|(x, y)| {
            cell().placed(shapez_blueprint::Placement {
                origin: (x, y, 0),
                rotation: Rotation::new(0),
            })
        })
        .collect();
    let mut buildings = cells
        .iter()
        .flat_map(|c| c.buildings.iter().cloned())
        .collect();
    routing::connect(&mut buildings, &cells, design)?;
    let layer = buildings.clone();
    for z in 1..3 {
        buildings.extend(layer.iter().cloned().map(|mut b| {
            b.layer = z;
            b
        }));
    }
    let mut island = Island::new("Foundation_1x1", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    Ok(Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: Some(design.icon()),
        source_format_version: None,
    })
}

/// Build all variants, validate transport and library icons, then publish together.
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "priority-merge")?;
    let mut cases = Vec::new();
    for design in &DESIGNS {
        let bp = build(design)?;
        let count = bp.islands()[0].buildings().len();
        let output = run.path.join(format!("{}.spz2bp", design.name));
        fs::write(&output, encode_blueprint(&bp)?)?;
        fs::write(
            run.path.join(format!("{}-layout.json", design.id)),
            serde_json::to_vec_pretty(&bp)?,
        )?;
        let mut harness = bp.clone();
        if let BlueprintContent::Islands(islands) = &mut harness.content {
            for (x, y, r) in [
                design.primary.harness(),
                design.secondary.harness(),
                (1, 0, 0),
            ] {
                islands
                    .entries
                    .push(Island::new("SpaceBelt_Forward", x, y, 0, Rotation::new(r)));
            }
        }
        let test_blueprint = run.path.join(format!("{}-harness.spz2bp", design.id));
        fs::write(&test_blueprint, encode_blueprint(&harness)?)?;
        cases.push(serde_json::json!({
        "name":design.id,"blueprint":test_blueprint,"expected_buildings":count,"export_graph":true,
        "primary_side":design.primary.name(),"secondary_side":design.secondary.name(),
        "experiment":{"assembly":"${LAB}/bin/PriorityTask/Release/netstandard2.1/ShapezLab.Priority.dll","type":"PriorityExperiment"}
    }));
        cases.push(serde_json::json!({
        "name":format!("{}-icon",design.id),"operation":"inspect-blueprint","blueprint":output,
        "expected_format_version":5,"expected_library_warning":false,"expected_icon":design.icon().components
    }));
        println!("Blueprint: {} ({count} buildings)", output.display());
        if design.id == "inline" {
            // A minimal reproducer must fail in the shared loader, without a task plugin.
            let mut invalid = bp.clone();
            if let BlueprintContent::Islands(islands) = &mut invalid.content {
                islands.entries[0].buildings = Some(BuildingBlueprint::new(vec![Building::new(
                    "BeltDefaultForwardInternalVariant",
                    5,
                    17,
                    0,
                    Rotation::new(0),
                )]));
            }
            let invalid_floor = run.path.join("invalid-floor.spz2bp");
            fs::write(&invalid_floor, encode_blueprint(&invalid)?)?;
            cases.push(serde_json::json!({
        "name":"reject-off-floor-without-plugin","blueprint":invalid_floor,"expected_ok":false
    }));
        }
    }
    let suite = serde_json::json!({"projects":["${REPO}/tasks/priority-merge/interop/PriorityTask.csproj"],"cases":cases});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    run_suite(&paths, &suite_path, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}
