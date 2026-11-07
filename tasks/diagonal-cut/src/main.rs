//! Generate and validate both diagonal factories.
mod design;
use anyhow::Result;
use shapez_blueprint::{
    Blueprint, BlueprintContent, BlueprintIcon, BuildingBlueprint, IconComponent, Island,
    IslandBlueprint, Rotation, encode_blueprint,
};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, time::Duration};
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "diagonal-cut")?;
    let mut cases = Vec::new();
    for (name, ne) in [("NE-SW", true), ("NW-SE", false)] {
        let (mut buildings, fixed, nets) =
            design::layout(ne, [[1, 0, 3, 2], [2, 3, 1, 0], [2, 1, 0, 3]])?;
        for route in shapez_routing::solve_tiles_sequential(
            shapez_model::TileRoutingProblem {
                geometry: shapez_model::FoundationGeometry::strip(2, 1),
                fixed: fixed.into_iter().collect(),
                nets,
            },
            shapez_model::SearchOptions {
                attempts: 30,
                seed: 1234567,
            },
        )? {
            shapez_blueprint::route_buildings(&route, &mut buildings);
        }
        let layer = buildings.clone();
        for z in 1..3 {
            buildings.extend(layer.iter().cloned().map(|mut b| {
                b.layer = z;
                b
            }));
        }
        let mut island = Island::new("Foundation_1x2", 0, 0, 0, Rotation::new(0));
        island.buildings = Some(BuildingBlueprint::new(buildings));
        let bp = Blueprint {
            game_version: 1138,
            content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
            icon: Some(BlueprintIcon {
                components: [
                    Some(IconComponent::Shape("CgCgCgCg".into())),
                    Some(IconComponent::Icon("ArrowDown".into())),
                    Some(IconComponent::Shape(
                        if ne { "Cg--Cg--" } else { "--Cg--Cg" }.into(),
                    )),
                    Some(IconComponent::Shape(
                        if ne { "Cg--Cg--" } else { "--Cg--Cg" }.into(),
                    )),
                ],
            }),
            source_format_version: None,
        };
        let file = run.path.join(format!("{name}.spz2bp"));
        fs::write(&file, encode_blueprint(&bp)?)?;
        cases.push(serde_json::json!({"name":format!("{name}-icon"),"operation":"inspect-blueprint","blueprint":file,"expected_format_version":5,"expected_library_warning":false,"expected_icon":bp.icon.as_ref().map(|i| &i.components)}));
        let mut harness = bp.clone();
        if let BlueprintContent::Islands(islands) = &mut harness.content {
            islands.entries.extend(shapez_blueprint::foundation_harness(
                &shapez_model::FoundationGeometry::strip(2, 3),
                &design::sockets(),
            ));
        }
        let fixture = run.path.join(format!("{name}-harness.spz2bp"));
        fs::write(&fixture, encode_blueprint(&harness)?)?;
        println!("{}", file.display());
        cases.push(serde_json::json!({"name":name,"blueprint":fixture,"ports":shapez_model::FoundationGeometry::strip(2,3).ports(&design::sockets()),"export_graph":true,"experiment":{"assembly":"${LAB}/bin/DiagonalTask/Release/netstandard2.1/ShapezLab.Diagonal.dll","type":"DiagonalExperiment"}}));
    }
    let suite = serde_json::json!({"projects":["${REPO}/tasks/diagonal-cut/interop/DiagonalTask.csproj"],"cases":cases});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    run_suite(&paths, &suite_path, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}
