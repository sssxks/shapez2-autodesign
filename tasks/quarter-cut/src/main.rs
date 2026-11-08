//! Rebuild the delivered design, or explicitly run native game acceptance.
mod compact;
use quarter_cut::design;
mod distribution;
mod layout;
use anyhow::Result;
use shapez_blueprint::{
    Blueprint, BlueprintContent, BlueprintIcon, BuildingBlueprint, IconComponent, Island,
    IslandBlueprint, Rotation, encode_blueprint,
};
use shapez_runtime::{LabPaths, RunDirectory, run_suite};
use std::{fs, time::Duration};
/// Default to portable reconstruction; native experiments remain explicit operations.
fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let argument = arguments.next();
    if matches!(
        argument.as_deref(),
        None | Some("--rebuild" | "--input" | "--output")
    ) {
        let mut input = std::path::PathBuf::from("artifacts/quarter-cut/solutions");
        let mut output = std::path::PathBuf::from("artifacts/quarter-cut");
        let mut options = argument.into_iter().chain(arguments);
        while let Some(option) = options.next() {
            match option.as_str() {
                "--rebuild" => {}
                "--input" | "--output" => {
                    let value = options
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("{option} requires a directory"))?;
                    if option == "--input" {
                        input = value.into();
                    } else {
                        output = value.into();
                    }
                }
                _ => anyhow::bail!("Unknown reconstruction option {option}"),
            }
        }
        return quarter_cut::rebuild(&input, &output);
    }
    let paths = LabPaths::discover();
    let run = RunDirectory::create(&paths.root, "quarter-cut")?;
    let argument = argument.filter(|s| s != "--native");
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
            .join("templates/quarter-cut-1x2-original.spz2bp")
    });
    let compact = source.to_string_lossy() == "--compact";
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
    anyhow::ensure!(bp.islands().len() == 1, "Expected one factory platform");
    let original = bp.islands()[0]
        .buildings()
        .iter()
        .any(|b| b.definition == "BeltPortReceiverInternalVariant" && b.x == 8 && b.y == 2);
    let (chunks, sockets) = match bp.islands()[0].definition.as_str() {
        "Foundation_1x2" => (
            2,
            if compact {
                compact::sockets()
            } else {
                compact::sockets_for(!original)
            },
        ),
        "Foundation_1x3" => (3, design::sockets()),
        other => anyhow::bail!("Unsupported quarter-cut foundation {other}"),
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
    let output = run.path.join(if chunks == 2 && original {
        "quarter-cut-1x2-original.spz2bp"
    } else if chunks == 2 {
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
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj".into()],
        cases: vec![
            shapez_runtime::case! {name:"factory",blueprint:test,ports:shapez_model::FoundationGeometry::strip(chunks,3).ports(&sockets),expected_buildings:count,expected_mergers:merger_count,export_graph:true,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll".into(), type_name: ("QuarterFactoryExperiment").into() }},
            shapez_runtime::case! {name:"library-icon",operation:"inspect-blueprint",blueprint:output.clone(),expected_format_version:5,expected_library_warning:false,expected_icon:bp.icon.as_ref().unwrap().components.clone()},
        ],
    };
    run_suite(&paths, &suite, Duration::from_secs(180))?;
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
    let suite = shapez_runtime::Suite {
        projects: vec!["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj".into()],
        cases: vec![
            shapez_runtime::case! {name:mode.trim_start_matches('-'),mirrored:mode=="--mirror-cell",jump_input:mode=="--jump-cell",output_row:if mode=="--middle-cell-2" {2}else if mode=="--middle-cell" {1}else if mode=="--mirror-cell" {0}else{3},blueprint:blueprint.clone(),expected_buildings:count,experiment: shapez_runtime::Experiment { assembly: "${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll".into(), type_name: (experiment).into() }},
        ],
    };
    run_suite(paths, &suite, Duration::from_secs(180))
}
