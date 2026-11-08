use shapez_model::{Node, Operation, Recipe, RecipeNode, RecipeReport, RecipeTrace, Shape};

#[test]
fn resolves_second_native_output_into_downstream_connection() {
    let shape: Shape = "CuCuCuCu".parse().unwrap();
    let half: Shape = "CuCu----".parse().unwrap();
    let recipe = Recipe {
        target: half.clone(),
        root: Node(2),
        nodes: vec![
            RecipeNode {
                id: Node(0),
                op: Operation::Source,
                inputs: vec![],
                expected: shape.clone(),
                color: None,
            },
            RecipeNode {
                id: Node(1),
                op: Operation::Cut,
                inputs: vec![Node(0)],
                expected: half.clone(),
                color: None,
            },
            RecipeNode {
                id: Node(2),
                op: Operation::RotateCw,
                inputs: vec![Node(1)],
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
                id: Node(0),
                op: Operation::Source,
                expected: shape,
                outputs: vec![Some("CuCuCuCu".into())],
                selected_port: 0,
            },
            RecipeTrace {
                id: Node(1),
                op: Operation::Cut,
                expected: half.clone(),
                outputs: vec![None, Some("CuCu----".into())],
                selected_port: 1,
            },
            RecipeTrace {
                id: Node(2),
                op: Operation::RotateCw,
                expected: half,
                outputs: vec![Some("CuCu----".into())],
                selected_port: 0,
            },
        ],
    };
    let verified = recipe.verified(&report).unwrap();
    assert_eq!(verified.nodes[2].inputs[0].port, 1);
    assert_eq!(verified.nodes[2].inputs[0].node, Node(1));
    report.trace[1].selected_port = 0;
    assert!(recipe.verified(&report).is_err());
}

#[test]
fn shape_deserialization_validates_wire_input() {
    for text in ["", "CuCuCu", "cucucucu", "CuCuCu你好", "CzCuCuCu"] {
        assert!(serde_json::from_value::<Shape>(serde_json::json!(text)).is_err());
    }
    let shape: Shape = "P-cw--Ru:CuCuCuCu".parse().unwrap();
    assert_eq!(shape.layers().count(), 2);
    assert_eq!(serde_json::to_value(shape).unwrap(), "P-cw--Ru:CuCuCuCu");
}

#[test]
fn layers_preserve_wire_order_and_node_indices_are_bounded() {
    use shapez_model::{Color, Layer, Material, Part};
    let shape: Shape = "CuCuCuCu:P-------".parse().unwrap();
    let mut layers = shape.layers();
    assert_eq!(
        layers.next_back(),
        Some(Layer([Part::Pin, Part::Empty, Part::Empty, Part::Empty]))
    );
    assert_eq!(
        layers.next(),
        Some(Layer([Part::Solid(Material::Circle, Color::Uncolored); 4]))
    );
    assert_eq!(layers.next(), None);
    assert_eq!(std::mem::size_of::<Node>(), 4);
    assert!(serde_json::from_str::<Node>("4294967296").is_err());
    assert_eq!(
        serde_json::from_str::<Node>("4294967295").unwrap(),
        Node(u32::MAX)
    );
}
