use anyhow::Result;
/// Borrowed recipe payload sent to the native execution boundary.
#[derive(serde::Serialize)]
struct RecipeRequest<'a> {
    /// Validated execution graph to run in the isolated game host.
    recipe: &'a shapez_model::Recipe,
}
use shapez_model::RecipeReport;
use shapez_runtime::{Host, LabPaths, RunDirectory};
use std::{fs, time::Duration};
/// Execute the generated construction suite against the native game implementation.
pub fn run_check(p: &LabPaths) -> Result<()> {
    shapez_runtime::run_suite(p, &crate::check_suite()?, Duration::from_secs(300))
}
/// Plan one target, execute it natively and publish its report and resolved graph.
pub fn run_recipe(p: &LabPaths, target: &str) -> Result<()> {
    let recipe = crate::plan_recipe(target)?;
    let run = RunDirectory::create(&p.root, "scaffold-recipe")?;

    p.build("LabMod")?;
    let t = Duration::from_secs(180);
    let mut host = Host::start_in(p, &run.path, t)?;
    let report = host
        .request::<_, RecipeReport>("recipe", RecipeRequest { recipe: &recipe }, t)?
        .into_result()?;
    host.shutdown()?;
    let verified = recipe.verified(&report)?;
    fs::write(
        run.path.join("verified-recipe.json"),
        serde_json::to_vec_pretty(&verified)?,
    )?;
    let mut readable = format!("Target: {target}\nVerified: true\n");
    for (node, trace) in recipe.nodes().iter().zip(&report.trace) {
        readable.push_str(&format!(
            "{} {:?} inputs={:?} output={} port={}\n",
            node.id().0,
            node.op(),
            node.inputs(),
            node.expected(),
            trace.selected_port
        ));
    }
    fs::write(run.path.join("recipe.txt"), readable)?;
    run.publish(&p.root)?;
    println!(
        "verified=true steps={}\nartifacts: {}",
        verified.nodes().len(),
        run.path.display()
    );
    Ok(())
}
