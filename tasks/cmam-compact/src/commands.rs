use crate::{compact_cmam, connect_cmam};
use anyhow::Result;
use shapez_runtime::{LabPaths, RunDirectory};
use std::path::Path;
pub fn run_compact(
    p: &LabPaths,
    blueprint: &Path,
    rgb: &Path,
    previous: Option<&Path>,
    native: bool,
    targets: &[String],
) -> Result<()> {
    let run = RunDirectory::create(&p.root, "cmam-compact")?;
    let blueprint = run.snapshot(blueprint, "source.spz2bp")?;
    let rgb = run.snapshot(rgb, "rgb.spz2bp")?;
    let previous = previous
        .map(|p| run.snapshot(p, "previous.spz2bp"))
        .transpose()?;
    let report = compact_cmam(&blueprint, &rgb, &run.path, previous.as_deref())?;
    if native {
        let blueprint = run.path.join("CMAM compact full-rate.spz2bp");
        let mut cases = vec![
            shapez_runtime::case! { name: "compact-placement", operation: "place-blueprint", blueprint: blueprint.clone(), expected_buildings: report.summary.buildings, seconds: 1 },
            shapez_runtime::case! { name: "compact-library-format", operation: "inspect-blueprint", blueprint: blueprint.clone(), expected_format_version: 5, expected_library_warning: false },
            shapez_runtime::case! { name: "input-capacity-diagnostic", operation: "blueprint-diagnostic", blueprint: blueprint.clone(), rate_manifest: report.boundary.clone(), warmup_seconds: 300, seconds: 120 },
        ];
        for target in targets {
            cases.push(shapez_runtime::case! { name: format!("production-diagnostic-{}", cases.len()), operation: "blueprint-diagnostic", blueprint: blueprint.clone(),
                production_manifest: shapez_runtime::ProductionManifest { interface: report.boundary.interface.clone(), production: report.production.clone() },
                target: target.clone(), seconds: 600, min_produced: 1,
                experiment: shapez_runtime::Experiment { assembly: p.root.join("bin/CmamTask/Release/netstandard2.1/ShapezLab.Cmam.dll"), type_name: "CmamExperiment".into() } });
        }
        let projects = if targets.is_empty() {
            vec![]
        } else {
            vec![p.repo.join("tasks/cmam-compact/interop/CmamTask.csproj")]
        };
        shapez_runtime::run_suite(
            p,
            &shapez_runtime::Suite { projects, cases },
            std::time::Duration::from_secs(900),
        )?;
    }
    run.publish(&p.root)?;
    println!("artifacts: {}", run.path.display());
    println!(
        "buildings={} islands={}",
        report.summary.buildings, report.summary.islands
    );
    println!("space nodes: {}", report.space_nodes);
    Ok(())
}
pub fn run_legacy(p: &LabPaths, blueprint: &Path, splitter: &Path, rgb: &Path) -> Result<()> {
    let run = RunDirectory::create(&p.root, "cmam-legacy")?;
    let blueprint = run.snapshot(blueprint, "source.spz2bp")?;
    let splitter = run.snapshot(splitter, "splitter.spz2bp")?;
    let rgb = run.snapshot(rgb, "rgb.spz2bp")?;
    let report = connect_cmam(&blueprint, &splitter, &rgb, &run.path)?;
    run.publish(&p.root)?;
    println!("artifacts: {}", run.path.display());
    println!("{}", report.blueprint.display());
    println!("platforms: {}", report.generated_platforms);
    Ok(())
}
