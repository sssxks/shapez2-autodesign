use crate::{
    Host, LabPaths,
    artifacts::RunDirectory,
    protocol::{Response, Suite},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
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

/// Resolve only path fields; descriptions and arbitrary payload strings are never rewritten.
pub fn expand_paths(suite: &mut Suite, p: &LabPaths) -> Result<()> {
    fn resolve(path: &mut std::path::PathBuf, p: &LabPaths) -> Result<()> {
        let library = std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default())
            .join("AppData/LocalLow/tobspr Games/shapez 2/blueprints");
        let mut text = path
            .to_string_lossy()
            .replace("${REPO}", &p.repo.to_string_lossy())
            .replace("${LAB}", &p.root.to_string_lossy())
            .replace("${LIBRARY}", &library.to_string_lossy());
        while let Some(start) = text.find("${TASK:") {
            let end = text[start..]
                .find('}')
                .context("Unclosed task artifact variable")?
                + start;
            let task = &text[start + 7..end];
            let run = RunDirectory::latest(&p.root, task)?;
            text.replace_range(start..=end, &run.path.to_string_lossy());
        }
        *path = text.into();
        Ok(())
    }
    for path in &mut suite.projects {
        resolve(path, p)?;
    }
    for case in &mut suite.cases {
        if let Some(path) = &mut case.blueprint {
            resolve(path, p)?;
        }
        if let Some(experiment) = &mut case.experiment {
            resolve(&mut experiment.assembly, p)?;
        }
    }
    Ok(())
}
pub fn run_suite_file(p: &LabPaths, path: &Path, timeout: Duration) -> Result<()> {
    let suite: Suite = toml::from_str(&fs::read_to_string(path)?)?;
    run_suite(p, &suite, timeout)
}

/// Execute a suite directly; saved reports are outputs, never execution inputs.
pub fn run_suite(p: &LabPaths, suite: &Suite, timeout: Duration) -> Result<()> {
    let mut suite = suite.clone();
    expand_paths(&mut suite, p)?;
    suite.validate()?;
    let run = RunDirectory::create(&p.root, "suite")?;
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
        let outcome: Option<CaseOutcome> = response.result.as_ref().map(|raw| serde_json::from_str(raw.get())).transpose()?;
        let passed = response.ok == case.expected_ok
            && case.expected_error.as_ref().is_none_or(|expected| {
                response
                    .error
                    .as_ref()
                    .is_some_and(|actual| actual.contains(expected))
            })
            && outcome.as_ref().and_then(|r| r.verified) != Some(false)
            && (!case.expected_ok
                || !matches!(case.operation.as_str(), "blueprint" | "place-blueprint")
                || outcome.as_ref().and_then(|r| r.verified) == Some(true));
        println!(
            "{}: {}",
            case.name,
            if !passed {
                "FAIL"
            } else if case.operation == "blueprint-diagnostic" {
                "DIAGNOSTIC"
            } else {
                "PASS"
            }
        );
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
        serde_json::to_vec_pretty(&SuiteReport { cases: &results })?,
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

/// Only acceptance status participates in runner logic; plugin diagnostics are archived verbatim.
#[derive(Serialize, Deserialize)]
pub struct CaseOutcome {
    pub verified: Option<bool>,
}
#[derive(Serialize)]
struct SuiteReport<'a> {
    cases: &'a [CaseResult],
}
