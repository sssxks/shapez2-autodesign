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

#[test]
fn native_cases_keep_payloads_typed_and_toml_rejects_misspelled_fields() {
    let suite: Suite = toml::from_str("[[cases]]\nname = 'inspection'\noperation = 'inspect-blueprint'\nblueprint = 'factory.spz2bp'\nexpected_format_version = 5\n").unwrap();
    assert_eq!(suite.cases[0].expected_format_version, Some(5));
    assert!(toml::from_str::<Suite>("[[cases]]\nname = 'bad'\nexpected_buidlings = 12\n").is_err());
    let legacy: Suite = toml::from_str(include_str!(
        "../../../tasks/cmam-adapter/tests/legacy.toml"
    ))
    .unwrap();
    legacy.validate().unwrap();
    let recipe: shapez_model::Recipe = shapez_model::RecipeDto {
        target: "CuCuCuCu".parse().unwrap(),
        root: shapez_model::Node(0),
        nodes: vec![shapez_model::RecipeNodeDto {
            id: shapez_model::Node(0),
            op: shapez_model::Operation::Source,
            inputs: vec![],
            expected: "CuCuCuCu".parse().unwrap(),
            color: None,
        }],
    }
    .try_into()
    .unwrap();
    recipe.validate().unwrap();
    let case = shapez_runtime::case! { name: "recipe", operation: "recipe", recipe: recipe };
    let wire = serde_json::to_string(&case).unwrap();
    let decoded: shapez_runtime::TestCase = serde_json::from_str(&wire).unwrap();
    assert_eq!(decoded.recipe.unwrap().target().to_string(), "CuCuCuCu");
}
