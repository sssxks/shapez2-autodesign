use crate::{compact_cmam, connect_cmam};
use anyhow::Result;
use shapez_runtime::{LabPaths, RunDirectory};
use std::path::Path;
pub fn run_compact(
    p: &LabPaths,
    blueprint: &Path,
    rgb: &Path,
    previous: Option<&Path>,
) -> Result<()> {
    let run = RunDirectory::create(&p.root, "cmam-compact")?;
    let blueprint = run.snapshot(blueprint, "source.spz2bp")?;
    let rgb = run.snapshot(rgb, "rgb.spz2bp")?;
    let previous = previous
        .map(|p| run.snapshot(p, "previous.spz2bp"))
        .transpose()?;
    let report = compact_cmam(&blueprint, &rgb, &run.path, previous.as_deref())?;
    run.publish(&p.root)?;
    println!("artifacts: {}", run.path.display());
    println!(
        "buildings={} islands={}",
        report["summary"]["buildings"], report["summary"]["islands"]
    );
    println!("space nodes: {}", report["space_nodes"]);
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
    println!("{}", report["blueprint"]);
    println!("platforms: {}", report["generated_platforms"]);
    Ok(())
}
