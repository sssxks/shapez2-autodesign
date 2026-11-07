//! Build, package and validate the 1x3 twelve-to-forty-eight belt quarter cutter.
mod design;
mod layout;
mod routing;
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
    if matches!(argument.as_deref(), Some("--cell" | "--definitions")) {
        return probe(&paths, &run, argument.as_deref() == Some("--cell"));
    }
    let source = argument.map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("templates/quarter-cut-1x3.spz2bp")
    });
    let bp = if source.to_string_lossy() == "--solve" {
        let mut island = Island::new("Foundation_1x3", 1, 0, 0, Rotation::new(0));
        island.buildings = Some(BuildingBlueprint::new(
            layout::build()?
                .into_iter()
                .map(|mut b| {
                    b.x -= 20;
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
    let output = run.path.join("quarter-cut-1x3.spz2bp");
    fs::write(&output, encode_blueprint(&bp)?)?;
    fs::write(
        run.path.join("layout.json"),
        serde_json::to_vec_pretty(&bp)?,
    )?;
    println!("Factory: {}", output.display());
    let count = bp.islands()[0].buildings().len();
    let mut harness = bp.clone();
    if let BlueprintContent::Islands(islands) = &mut harness.content {
        for (x, y, r) in [(1, -1, 1), (0, -1, 3), (2, -1, 3), (0, 1, 1), (2, 1, 1)] {
            islands
                .entries
                .push(Island::new("SpaceBelt_Forward", x, y, 0, Rotation::new(r)));
        }
    }
    let test = run.path.join("harness.spz2bp");
    fs::write(&test, encode_blueprint(&harness)?)?;
    let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":"factory","blueprint":test,"expected_buildings":count,"export_graph":true,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterFactoryExperiment"}},{"name":"library-icon","operation":"inspect-blueprint","blueprint":output,"expected_format_version":5,"expected_library_warning":false,"expected_icon":bp.icon.as_ref().unwrap().components}]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    run_suite(&paths, &suite_path, Duration::from_secs(180))?;
    run.publish(&paths.root)?;
    Ok(())
}

/// Run a bounded native discovery or unit-cell experiment without publishing a factory.
fn probe(paths: &LabPaths, run: &RunDirectory, cell: bool) -> Result<()> {
    let buildings = if cell { design::cell(8, 6) } else { Vec::new() };
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
    let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":if cell {"cell"}else{"definitions"},"blueprint":blueprint,"expected_buildings":count,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":if cell {"QuarterCellExperiment"}else{"QuarterCutDefinitions"}}}]});
    let suite_path = run.path.join("suite.json");
    fs::write(&suite_path, serde_json::to_vec_pretty(&suite)?)?;
    run_suite(paths, &suite_path, Duration::from_secs(180))
}
