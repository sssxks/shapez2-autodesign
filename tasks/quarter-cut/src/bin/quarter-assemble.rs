//! Assemble solved northern banks and their reflected southern counterparts.
use anyhow::{Context, Result, ensure};
use clap::Parser;
use shapez_blueprint::{
    Blueprint, BlueprintContent, BlueprintIcon, Building, BuildingBlueprint, IconComponent, Island,
    IslandBlueprint, Rotation, encode_blueprint,
};
use shapez_model::{Direction, FoundationGeometry, FoundationSocket};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, path::PathBuf, time::Duration};

#[derive(Parser)]
struct Args {
    /// Local 17x7 bank with two inputs and an eastward through lane.
    left: PathBuf,
    /// Local 17x7 bank with an input on row five and outputs on columns 8..11.
    right: PathBuf,
}

/// Reflect connectors while preserving the actual shape rotation operation.
fn reflect(mut b: Building) -> Building {
    b.y = 13 - b.y;
    b.rotation = Rotation::new((4 - b.rotation.index()) % 4);
    if [
        "CutterDefault",
        "BeltDefaultLeft",
        "Merger2To1L",
        "Splitter1To2L",
    ]
    .iter()
    .any(|prefix| b.definition.starts_with(prefix))
        || b.definition.starts_with("Lift") && b.definition.contains("Left")
    {
        b.definition = if let Some(base) = b.definition.strip_suffix("Mirrored") {
            base.to_owned()
        } else {
            format!("{}Mirrored", b.definition)
        };
    }
    b
}

fn main() -> Result<()> {
    let args = Args::parse();
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    let mut buildings = Vec::new();
    for (file, x_offset) in [(&args.left, 0), (&args.right, 17)] {
        let bank: Vec<Building> = serde_json::from_slice(&fs::read(file)?)?;
        let layers = bank.iter().map(|b| b.layer).max().context("Empty bank")? + 1;
        ensure!(
            layers == 1 || layers == 3,
            "Banks must use one or three layers"
        );
        for layer in 0..if layers == 1 { 3 } else { 1 } {
            for b in &bank {
                ensure!(
                    (0..17).contains(&b.x) && (0..7).contains(&b.y),
                    "Bank outside 17x7"
                );
                for mut b in [b.clone(), reflect(b.clone())] {
                    b.x += 3 + x_offset;
                    b.y += 3;
                    b.layer += layer;
                    buildings.push(b);
                }
            }
        }
    }
    let sockets = [
        (Direction::West, 0, false, "input"),
        (Direction::North, 0, true, "nw"),
        (Direction::North, 1, true, "ne"),
        (Direction::South, 0, true, "sw"),
        (Direction::South, 1, true, "se"),
    ]
    .map(|(side, segment, output, role)| FoundationSocket {
        side,
        segment,
        output,
        role,
    });
    let geometry = FoundationGeometry::strip(2, 3);
    for socket in sockets {
        for z in 0..3 {
            for lane in 0..4 {
                let p = geometry.socket_tile(socket, lane, z);
                buildings.push(Building::new(
                    if socket.output {
                        "BeltPortSenderInternalVariant"
                    } else {
                        "BeltPortReceiverInternalVariant"
                    },
                    p.0,
                    p.1,
                    p.2,
                    Rotation::new(socket.direction()),
                ));
            }
        }
    }
    fs::write(
        run.path.join("buildings.json"),
        serde_json::to_vec_pretty(&buildings)?,
    )?;
    let count = buildings.len();
    let mergers = buildings
        .iter()
        .filter(|b| b.definition.starts_with("Merger"))
        .count();
    let mut island = Island::new("Foundation_1x2", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let bp = Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: Some(BlueprintIcon {
            components: [
                Some(IconComponent::Shape("CuCuCuCu".into())),
                Some(IconComponent::Shape("Cu------".into())),
                Some(IconComponent::Shape("Cu------".into())),
                Some(IconComponent::Shape("Cu------".into())),
            ],
        }),
        source_format_version: None,
    };
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
