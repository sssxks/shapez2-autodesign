//! Shared native acceptance boundary for solved grid and weave regions.
use anyhow::Result;
use shapez_blueprint::{Blueprint, BlueprintIcon, Building, BuildingBlueprint, Island, Rotation};
use shapez_model::TilePoint;
use shapez_runtime::{LabPaths, RunDirectory, run_suite};

/// One physical supply lane with its solved rate in items per second.
pub(crate) struct Input {
    /// Native tile coordinate and building layer of the lane endpoint.
    pub position: TilePoint,
    /// Required source throughput in items per second.
    pub rate: f64,
}
/// One physical measured lane and the shape class expected by the region experiment.
pub(crate) struct Output {
    /// Native tile coordinate and building layer of the lane endpoint.
    pub position: TilePoint,
    /// Whether the output carries full shapes rather than cut quarters.
    pub full: bool,
}
/// Package the region on a test platform and validate its explicit lane manifest.
pub(crate) fn validate(
    name: &str,
    buildings: Vec<Building>,
    icon: Option<BlueprintIcon>,
    inputs: &[Input],
    outputs: &[Output],
) -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, &format!("quarter-{name}"))?;
    let count = buildings.len();
    let mut island = Island::new("Foundation_2x2", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let blueprint = run.path.join(format!("{name}.spz2bp"));
    Blueprint::platform(island, icon).write(&blueprint)?;
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj".into()],
        cases: vec![shapez_runtime::case! {
            name:name, blueprint:blueprint, expected_buildings:count,
            inputs:inputs.iter().map(|p| p.position).collect::<Vec<_>>(),
            input_rates:inputs.iter().map(|p| p.rate).collect::<Vec<_>>(),
            outputs:outputs.iter().map(|p| p.position).collect::<Vec<_>>(),
            output_full:outputs.iter().map(|p| p.full).collect::<Vec<_>>(),
            experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll".into(), type_name: "QuarterWeaveExperiment".into() }
        }],
    };
    eprintln!("Native {name}: {}", run.path.display());
    run_suite(&paths, &suite, std::time::Duration::from_secs(180))
}
