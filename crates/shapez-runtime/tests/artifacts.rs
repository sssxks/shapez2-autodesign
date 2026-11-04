use shapez_runtime::{LabPaths, artifacts::RunDirectory, protocol::Suite, suite::expand_paths};
use std::fs;

#[test]
fn task_artifacts_publish_atomically_and_resolve_in_suites() {
    let mut paths = LabPaths::discover();
    paths.root = std::env::temp_dir().join(format!("shapez-artifact-test-{}", std::process::id()));
    let first = RunDirectory::create(&paths.root, "example").unwrap();
    assert!(RunDirectory::latest(&paths.root, "example").is_err());
    first.publish(&paths.root).unwrap();
    let second = RunDirectory::create(&paths.root, "example").unwrap();
    second.publish(&paths.root).unwrap();
    let mut suite = serde_json::json!({"cases":[{"name":"example","blueprint":"${TASK:example}/factory.spz2bp"}]});
    expand_paths(&mut suite, &paths).unwrap();
    let suite: Suite = serde_json::from_value(suite).unwrap();
    suite.validate().unwrap();
    assert_eq!(std::path::Path::new(suite.cases[0].args["blueprint"].as_str().unwrap()),second.path.join("factory.spz2bp"));
    for run in [first, second] {
        fs::remove_file(run.path.join("run.json")).unwrap();
        fs::remove_dir(run.path).unwrap();
    }
    fs::remove_file(paths.root.join("tasks/example/latest.json")).unwrap();
    fs::remove_dir(paths.root.join("tasks/example")).unwrap();
    fs::remove_dir(paths.root.join("tasks")).unwrap();
    fs::remove_dir(paths.root.join("runs")).unwrap();
    fs::remove_dir(paths.root).unwrap();
}
