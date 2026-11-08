use anyhow::Result;
#[derive(serde::Serialize)]
struct RecipeRequest<'a> {
    recipe: &'a shapez_model::Recipe,
}
use shapez_model::RecipeReport;
use shapez_runtime::{Host, LabPaths, RunDirectory};
use std::{fs, time::Duration};
pub fn run_check(p: &LabPaths) -> Result<()> {
    shapez_runtime::run_suite(p, &crate::check_suite()?, Duration::from_secs(300))
}
pub fn run_recipe(p: &LabPaths, target: &str) -> Result<()> {
    let recipe = crate::plan_recipe(target)?;
    let run = RunDirectory::create(&p.root, "scaffold-recipe")?;
    let path = run.path.join("recipe.json");
    fs::write(&path, serde_json::to_vec_pretty(&recipe)?)?;
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
    for (node, trace) in recipe.nodes.iter().zip(&report.trace) {
        readable.push_str(&format!(
            "{} {:?} inputs={:?} output={} port={}\n",
            node.id.0, node.op, node.inputs, node.expected, trace.selected_port
        ));
    }
    fs::write(run.path.join("recipe.txt"), readable)?;
    run.publish(&p.root)?;
    println!(
        "verified=true steps={}\nartifacts: {}",
        verified.nodes.len(),
        run.path.display()
    );
    Ok(())
}
