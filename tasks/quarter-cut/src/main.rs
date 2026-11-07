//! Validate the delivered 1x3 cutter and experimentally solve its 1x2 replacement.
mod compact;
mod design;
mod distribution;
mod layout;
use anyhow::Result;
use shapez_blueprint::{
    Blueprint, BlueprintContent, BlueprintIcon, BuildingBlueprint, IconComponent, Island,
    IslandBlueprint, Rotation, encode_blueprint,
};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, time::Duration};
/// Package the native-validated template, or validate a supplied blueprint path.
fn main() -> Result<()> {
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    let argument = std::env::args().nth(1);
    if matches!(
        argument.as_deref(),
        Some(
            "--cell"
                | "--mirror-cell"
                | "--middle-cell"
                | "--middle-cell-2"
                | "--jump-cell"
                | "--definitions"
                | "--capacity"
        )
    ) {
        return probe(&paths, &run, argument.as_deref().unwrap());
    }
    let source = argument.map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("templates/quarter-cut-1x3.spz2bp")
    });
    let compact = source.to_string_lossy() == "--compact";
    let chunks = if compact { 2 } else { 3 };
    let sockets = if compact {
        compact::sockets()
    } else {
        design::sockets()
    };
    let bp = if source.to_string_lossy() == "--solve" || compact {
        let mut island = Island::new(
            if compact {
                "Foundation_1x2"
            } else {
                "Foundation_1x3"
            },
            if compact { 0 } else { 1 },
            0,
            0,
            Rotation::new(0),
        );
        island.buildings = Some(BuildingBlueprint::new(
            (if compact {
                compact::build()?
            } else {
                layout::build()?
            })
            .into_iter()
            .map(|mut b| {
                if !compact {
                    b.x -= 20;
                }
                b
            })
            .collect(),
        ));
        Blueprint {
            game_version: 1138,
            content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
            icon: None,
            source_format_version: None,
        }
    } else {
        Blueprint::read(source)?
    };
    let mut bp = bp;
    bp.icon = Some(BlueprintIcon {
        components: [
            Some(IconComponent::Shape("CuCuCuCu".into())),
            Some(IconComponent::Shape("Cu------".into())),
            Some(IconComponent::Shape("Cu------".into())),
            Some(IconComponent::Shape("Cu------".into())),
        ],
    });
    let output = run.path.join(if compact {
        "quarter-cut-1x2.spz2bp"
    } else {
        "quarter-cut-1x3.spz2bp"
    });
    fs::write(&output, encode_blueprint(&bp)?)?;
    fs::write(
        run.path.join("layout.json"),
        serde_json::to_vec_pretty(&bp)?,
    )?;
    println!("Factory: {}", output.display());
    let count = bp.islands()[0].buildings().len();
    let merger_count = bp.islands()[0]
        .buildings()
        .iter()
        .filter(|b| b.definition.starts_with("Merger"))
        .count();
    let mut harness = bp.clone();
    if let BlueprintContent::Islands(islands) = &mut harness.content {
        islands.entries.extend(shapez_blueprint::foundation_harness(
            &shapez_model::FoundationGeometry::strip(chunks, 3),
            &sockets,
        ));
    }
    let test = run.path.join("harness.spz2bp");
    fs::write(&test, encode_blueprint(&harness)?)?;
    let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":"factory","blueprint":test,"ports":shapez_model::FoundationGeometry::strip(chunks,3).ports(&sockets),"expected_buildings":count,"expected_mergers":merger_count,"export_graph":true,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterFactoryExperiment"}},{"name":"library-icon","operation":"inspect-blueprint","blueprint":output,"expected_format_version":5,"expected_library_warning":false,"expected_icon":bp.icon.as_ref().unwrap().components}]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    run_suite(&paths, &suite_path, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}

/// Run a bounded native discovery or unit-cell experiment without publishing a factory.
fn probe(paths: &LabPaths, run: &RunDirectory, mode: &str) -> Result<()> {
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
    let bp = Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: None,
        source_format_version: None,
    };
    let blueprint = run.path.join("probe.spz2bp");
    fs::write(&blueprint, encode_blueprint(&bp)?)?;
    let experiment = match mode {
        "--capacity" => "QuarterCapacityExperiment",
        "--cell" | "--mirror-cell" | "--middle-cell" | "--middle-cell-2" | "--jump-cell" => {
            "QuarterCellExperiment"
        }
        _ => "QuarterCutDefinitions",
    };
    let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":mode.trim_start_matches('-'),"mirrored":mode=="--mirror-cell","jump_input":mode=="--jump-cell","output_row":if mode=="--middle-cell-2" {2}else if mode=="--middle-cell" {1}else if mode=="--mirror-cell" {0}else{3},"blueprint":blueprint,"expected_buildings":count,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":experiment}}]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    run_suite(paths, &suite_path, Duration::from_secs(180))
}
