use scaffold_recipe::plan_recipe;

#[test]
fn task_builds_valid_dags_and_keeps_strategy_rejections_local() {
    for target in ["CuCuCuCu", "P-cwP-cw", "WwcbWwcb:P-cwP-cw:WgP---P-"] {
        let recipe = plan_recipe(target).unwrap();
        recipe.validate().unwrap();
        assert_eq!(recipe.target.as_str(), target);
    }
    assert!(plan_recipe("Cu------:--------:Cu------").is_err());
    assert!(plan_recipe("P-------:cr------").is_err());
}
