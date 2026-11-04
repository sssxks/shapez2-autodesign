use crate::{
    Host, LabPaths,
    artifacts::RunDirectory,
    protocol::{Response, Suite},
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::Path, time::Duration};

#[derive(Serialize)]
pub struct CaseResult {
    pub name: String,
    pub passed: bool,
    #[serde(flatten)]
    pub response: Option<Response>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_error: Option<String>,
}

/// Variables affect path/configuration strings; task schemas remain task-owned.
pub fn expand_paths(value: &mut Value, p: &LabPaths) -> Result<()> {
    match value {
        Value::String(s) => {
            let library =
                std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
                    .join("AppData/LocalLow/tobspr Games/shapez 2/blueprints");
            *s = s
                .replace("${REPO}", &p.repo.to_string_lossy())
                .replace("${LAB}", &p.root.to_string_lossy())
                .replace("${LIBRARY}", &library.to_string_lossy());
            while let Some(start) = s.find("${TASK:") {
                let end = s[start..]
                    .find('}')
                    .context("Unclosed task artifact variable")?
                    + start;
                let task = &s[start + 7..end];
                let run = RunDirectory::latest(&p.root, task)
                    .with_context(|| format!("No published artifacts for task {task}"))?;
                s.replace_range(start..=end, &run.path.to_string_lossy());
            }
        }
        Value::Array(a) => {
            for v in a {
                expand_paths(v, p)?;
            }
        }
        Value::Object(m) => {
            for v in m.values_mut() {
                expand_paths(v, p)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn run_suite(p: &LabPaths, path: &Path, timeout: Duration) -> Result<()> {
    let run = RunDirectory::create(&p.root, "suite")?;
    let snapshot = run.snapshot(path, "suite.json")?;
    let mut raw: Value = serde_json::from_slice(&fs::read(snapshot)?)?;
    expand_paths(&mut raw, p)?;
    let suite: Suite = serde_json::from_value(raw)?;
    suite.validate()?;
    fs::write(
        run.path.join("resolved-suite.json"),
        serde_json::to_vec_pretty(&suite)?,
    )?;
    p.build("LabMod")?;
    for project in &suite.projects {
        p.build_project(project)?;
    }
    let mut host = Host::start_in(p, &run.path, timeout)?;
    let mut results = Vec::new();
    for case in &suite.cases {
        let response: Response = match host.request(&case.operation, case, timeout) {
            Ok(response) => response,
            Err(e) => {
                results.push(CaseResult {
                    name: case.name.clone(),
                    passed: false,
                    response: None,
                    host_error: Some(e.to_string()),
                });
                break;
            }
        };
        let passed = response.ok == case.expected_ok
            && response
                .result
                .as_ref()
                .and_then(|r| r.get("verified"))
                .and_then(Value::as_bool)
                != Some(false);
        println!("{}: {}", case.name, if passed { "PASS" } else { "FAIL" });
        if !passed {
            eprintln!(
                "{}",
                response
                    .error
                    .as_deref()
                    .unwrap_or("Native acceptance failed")
            );
        }
        results.push(CaseResult {
            name: case.name.clone(),
            passed,
            response: Some(response),
            host_error: None,
        });
    }
    let report = run.path.join("report.json");
    fs::write(
        &report,
        serde_json::to_vec_pretty(&serde_json::json!({"cases":results}))?,
    )?;
    host.shutdown()?;
    println!("{}", report.display());
    ensure!(
        results.len() == suite.cases.len() && results.iter().all(|r| r.passed),
        "One or more test cases failed"
    );
    run.publish(&p.root)?;
    Ok(())
}
