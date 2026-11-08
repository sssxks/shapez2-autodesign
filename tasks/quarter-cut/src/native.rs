//! Native acceptance of an in-memory factory and standalone cell probes.
use anyhow::Result;
use quarter_cut::design;
use shapez_blueprint::{Blueprint, BuildingBlueprint, Island, Rotation};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::time::Duration;
pub fn validate(bp: Blueprint) -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    anyhow::ensure!(bp.islands().len() == 1, "Expected one factory platform");
    let original = bp.islands()[0]
        .buildings()
        .iter()
        .any(|b| b.definition == "BeltPortReceiverInternalVariant" && b.x == 8 && b.y == 2);
    anyhow::ensure!(
        bp.islands()[0].definition == "Foundation_1x2",
        "Expected a 1x2 factory platform"
    );
    let sockets = if original {
        quarter_cut::original_sockets()
    } else {
        quarter_cut::factory_sockets()
    };
    let mut bp = bp;
    bp.icon = Some(design::icon());
    let output = run.path.join(if original {
        "quarter-cut-1x2-original.spz2bp"
    } else {
        "quarter-cut-1x2.spz2bp"
    });
    bp.write(&output)?;

    println!("Factory: {}", output.display());
    let count = bp.islands()[0].buildings().len();
    let merger_count = bp.islands()[0]
        .buildings()
        .iter()
        .filter(|b| b.definition.starts_with("Merger"))
        .count();
    let harness = bp.with_harness(&shapez_model::FoundationGeometry::strip(2, 3), &sockets)?;
    let test = run.path.join("harness.spz2bp");
    harness.write(&test)?;
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj".into()],
        cases: vec![
            shapez_runtime::case! {name:"factory",blueprint:test,ports:shapez_model::FoundationGeometry::strip(2,3).ports(&sockets),expected_buildings:count,expected_mergers:merger_count,export_graph:true,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll".into(), type_name: ("QuarterFactoryExperiment").into() }},
            shapez_runtime::case! {name:"library-icon",operation:"inspect-blueprint",blueprint:output.clone(),expected_format_version:5,expected_library_warning:false,expected_icon:bp.icon.as_ref().unwrap().components.clone()},
        ],
    };
    run_suite(&paths, &suite, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}

/// Run a bounded native discovery or unit-cell experiment without publishing a factory.
pub fn probe(paths: &LabPaths, run: &RunDirectory, mode: &str) -> Result<()> {
    let cell = mode != "--definitions";
    let mut buildings = if cell {
        (if mode == "--middle-cell-2" {
            design::middle_cell(2)
        } else if mode == "--middle-cell" {
            design::middle_cell(1)
        } else if mode == "--mirror-cell" {
            design::reflected_cell(false)
        } else {
            design::cell(false)
        })
        .placed(shapez_blueprint::Placement {
            origin: (8, 6, 0),
            rotation: Rotation::new(0),
        })
        .buildings
    } else {
        Vec::new()
    };
    if mode == "--jump-cell" {
        buildings.extend(
            [
                (3, "BeltDefaultForwardInternalVariant"),
                (4, "BeltPortSenderInternalVariant"),
                (7, "BeltPortReceiverInternalVariant"),
            ]
            .map(|(x, t)| shapez_blueprint::Building::new(t, x, 8, 0, Rotation::new(0))),
        );
    }
    let count = buildings.len();
    let mut island = Island::new("Foundation_2x2", 0, 0, 0, Rotation::new(0));
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let bp = Blueprint::platform(island, None);
    let blueprint = run.path.join("probe.spz2bp");
    bp.write(&blueprint)?;
    let experiment = match mode {
        "--capacity" => "QuarterCapacityExperiment",
        "--cell" | "--mirror-cell" | "--middle-cell" | "--middle-cell-2" | "--jump-cell" => {
            "QuarterCellExperiment"
        }
        _ => "QuarterCutDefinitions",
    };
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj".into()],
        cases: vec![
            shapez_runtime::case! {operation:if matches!(mode,"--definitions" | "--capacity") {"blueprint-diagnostic"} else {"blueprint"},name:mode.trim_start_matches('-'),mirrored:mode=="--mirror-cell",jump_input:mode=="--jump-cell",output_row:if mode=="--middle-cell-2" {2}else if mode=="--middle-cell" {1}else if mode=="--mirror-cell" {0}else{3},blueprint:blueprint.clone(),expected_buildings:count,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll".into(), type_name: (experiment).into() }},
        ],
    };
    run_suite(paths, &suite, Duration::from_secs(180))
}
