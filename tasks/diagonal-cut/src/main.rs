//! Generate and validate both diagonal factories.
mod design;
mod overflow;
mod overflow_design;
use anyhow::Result;
use shapez_blueprint::{
    Blueprint, BlueprintIcon, BuildingBlueprint, IconComponent, Island, Rotation,
};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::time::Duration;
fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    if let Some(command) = args.next() {
        anyhow::ensure!(
            command == "overflow",
            "Usage: diagonal-cut [overflow <blueprint>]"
        );
        let source = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("overflow requires a blueprint"))?;
        anyhow::ensure!(args.next().is_none(), "Unexpected argument");
        return overflow::run(source.into());
    }
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
        let bp = Blueprint::platform(
            island,
            Some(BlueprintIcon {
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
        );
        let file = run.path.join(format!("{name}.spz2bp"));
        bp.write(&file)?;
        cases.push(shapez_runtime::case!{name:format!("{name}-icon"),operation:"inspect-blueprint",blueprint:file.clone(),expected_format_version:5,expected_library_warning:false,expected_icon:bp.icon.as_ref().map(|i| i.components.clone())});
        let harness = bp.with_harness(
            &shapez_model::FoundationGeometry::strip(2, 3),
            &design::sockets(),
        )?;
        let fixture = run.path.join(format!("{name}-harness.spz2bp"));
        harness.write(&fixture)?;
        println!("{}", file.display());
        cases.push(shapez_runtime::case!{name:name,blueprint:fixture,ports:shapez_model::FoundationGeometry::strip(2,3).ports(&design::sockets()),export_graph:true,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/DiagonalTask/Release/netstandard2.1/ShapezLab.Diagonal.dll".into(), type_name: ("DiagonalExperiment").into() }});
    }
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/diagonal-cut/interop/DiagonalTask.csproj".into()],
        cases,
    };
    run_suite(&paths, &suite, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}
