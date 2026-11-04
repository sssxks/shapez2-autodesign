use shapez_model::{NodeId, Operation, Recipe, RecipeNode, RecipeReport, RecipeTrace, Shape};

#[test]
fn resolves_second_native_output_into_downstream_connection() {
    let shape: Shape = "CuCuCuCu".parse().unwrap();
    let half: Shape = "CuCu----".parse().unwrap();
    let recipe = Recipe {
        target: half.clone(),
        root: NodeId(2),
        nodes: vec![
            RecipeNode {
                id: NodeId(0),
                op: Operation::Source,
                inputs: vec![],
                expected: shape.clone(),
                color: None,
            },
            RecipeNode {
                id: NodeId(1),
                op: Operation::Cut,
                inputs: vec![NodeId(0)],
                expected: half.clone(),
                color: None,
            },
            RecipeNode {
                id: NodeId(2),
                op: Operation::RotateCw,
                inputs: vec![NodeId(1)],
                expected: half.clone(),
                color: None,
            },
        ],
    };
    let mut report = RecipeReport {
        verified: true,
        target: half.clone(),
        steps: Some(3),
        failed_node: None,
        trace: vec![
            RecipeTrace {
                id: NodeId(0),
                op: Operation::Source,
                expected: shape,
                outputs: vec![Some("CuCuCuCu".into())],
                selected_port: 0,
            },
            RecipeTrace {
                id: NodeId(1),
                op: Operation::Cut,
                expected: half.clone(),
                outputs: vec![None, Some("CuCu----".into())],
                selected_port: 1,
            },
            RecipeTrace {
                id: NodeId(2),
                op: Operation::RotateCw,
                expected: half,
                outputs: vec![Some("CuCu----".into())],
                selected_port: 0,
            },
        ],
    };
    let verified = recipe.verified(&report).unwrap();
    assert_eq!(verified.nodes[2].inputs[0].port, 1);
    assert_eq!(verified.nodes[2].inputs[0].node, NodeId(1));
    report.trace[1].selected_port = 0;
    assert!(recipe.verified(&report).is_err());
}

#[test]
fn shape_deserialization_validates_wire_input() {
    for text in ["", "CuCuCu", "cucucucu", "CuCuCu你好", "CzCuCuCu"] {
        assert!(serde_json::from_value::<Shape>(serde_json::json!(text)).is_err());
    }
    let shape: Shape = "P-cw--Ru:CuCuCuCu".parse().unwrap();
    assert_eq!(shape.layers().len(), 2);
    assert_eq!(serde_json::to_value(shape).unwrap(), "P-cw--Ru:CuCuCuCu");
}
