use shapez_runtime::{LabPaths, RunDirectory, Suite, expand_paths};
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
    let mut suite = Suite {
        projects: vec![],
        cases: vec![
            shapez_runtime::case! { name: "example", blueprint: std::path::PathBuf::from("${TASK:example}/factory.spz2bp") },
        ],
    };
    expand_paths(&mut suite, &paths).unwrap();

    suite.validate().unwrap();
    assert_eq!(
        suite.cases[0].blueprint.as_ref().unwrap(),
        &second.path.join("factory.spz2bp")
    );
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
