//! Native acceptance regressions: exact placement, throughput and recovery failures.
use anyhow::Result;
use shapez_blueprint::{
    Blueprint, BlueprintContent, Building, BuildingBlueprint, Island, IslandBlueprint, Rotation,
};
use shapez_runtime::{Experiment, LabPaths, RunDirectory, Suite, run_suite};
use std::time::Duration;
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "native-acceptance")?;
    let mut cases = Vec::new();
    for (name, error) in [
        ("healthy", None),
        ("upper-trash", Some("Incomplete player blueprint placement")),
        (
            "outside-floor",
            Some("Incomplete player blueprint placement"),
        ),
        ("half-speed", Some("Nominal throughput failed")),
        ("wrong-shape", Some("Wrong output shape")),
        ("latched-output", Some("Recovery deadline exceeded")),
        (
            "no-experiment",
            Some("Blueprint acceptance requires an experiment"),
        ),
        (
            "unverified",
            Some("Experiment did not explicitly verify acceptance"),
        ),
    ] {
        let mut buildings = Vec::new();
        for z in 0..3 {
            for x in 8..12 {
                for y in 2..18 {
                    let kind = match y {
                        2 => "BeltPortReceiverInternalVariant",
                        17 => "BeltPortSenderInternalVariant",
                        16 if name == "half-speed" => "RotatorOneQuadInternalVariant",
                        _ => "BeltDefaultForwardInternalVariant",
                    };
                    buildings.push(Building::new(kind, x, y, z, Rotation::new(1)));
                }
            }
        }
        if name == "upper-trash" {
            buildings.push(Building::new(
                "TrashDefaultInternalVariant",
                4,
                4,
                1,
                Rotation::new(0),
            ));
        }
        if name == "outside-floor" {
            buildings.push(Building::new(
                "BeltDefaultForwardInternalVariant",
                2,
                5,
                0,
                Rotation::new(1),
            ));
        }
        let mut island = Island::new("Foundation_1x1", 0, 0, 0, Rotation::new(0));
        island.buildings = Some(BuildingBlueprint::new(buildings));
        let bp = Blueprint {
            game_version: 1138,
            source_format_version: None,
            icon: None,
            content: BlueprintContent::Islands(IslandBlueprint::new(vec![
                island,
                Island::new("SpaceBelt_Forward", 0, -1, 0, Rotation::new(1)),
                Island::new("SpaceBelt_Forward", 0, 1, 0, Rotation::new(1)),
            ])),
        };
        let file = run.path.join(format!("{name}.spz2bp"));
        bp.write(&file)?;
        let mut case = shapez_runtime::case! {name:name,blueprint:file,export_graph:false,experiment:Experiment{assembly:"${LAB}/bin/AcceptanceTask/Release/netstandard2.1/ShapezLab.Acceptance.dll".into(),type_name:"AcceptanceExperiment".into()}};
        case.expected_ok = error.is_none();
        if name == "no-experiment" {
            case.experiment = None;
        }
        case.expected_error = error.map(Box::<str>::from);
        cases.push(case);
    }
    run_suite(
        &paths,
        &Suite {
            projects: vec!["${REPO}/tasks/native-acceptance/interop/AcceptanceTask.csproj".into()],
            cases,
        },
        Duration::from_secs(180),
    )?;
    run.publish(&paths.root)
}
