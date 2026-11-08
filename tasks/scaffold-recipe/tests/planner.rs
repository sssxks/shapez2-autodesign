use scaffold_recipe::plan_recipe;

#[test]
fn task_builds_valid_dags_and_keeps_strategy_rejections_local() {
    for target in ["CuCuCuCu", "P-cwP-cw", "WwcbWwcb:P-cwP-cw:WgP---P-"] {
        let recipe = plan_recipe(target).unwrap();
        recipe.validate().unwrap();
        assert_eq!(recipe.target().as_str(), target);
    }
    assert!(plan_recipe("Cu------:--------:Cu------").is_err());
    assert!(plan_recipe("P-------:cr------").is_err());
}

#[test]
fn all_single_layer_symbol_combinations_and_sparse_columns() {
    let suite = scaffold_recipe::check_suite().unwrap();
    assert_eq!(suite.cases.len(), 624);
    for case in suite.cases {
        let recipe = case.recipe.unwrap();
        recipe.validate().unwrap();
        assert_eq!(
            recipe.nodes()[recipe.root().0 as usize].expected(),
            recipe.target()
        );
    }
    for target in [
        "------cr",
        "--P-----",
        "Cu------:cr------",
        "------Cu:------cr",
        "Ru--Wg--:P---cb--",
    ] {
        let recipe = plan_recipe(target).unwrap();
        assert_eq!(recipe.target().as_str(), target);
        recipe.validate().unwrap();
    }
}
