use shapez_blueprint::{Building, Cell, CellPort, Placement, Rotation};
#[test]
fn placement_transforms_non_anchor_occupancy_and_ports_together() {
    let cell = Cell::new(
        vec![Building::new(
            "CutterDefaultInternalVariant",
            0,
            1,
            0,
            Rotation::new(0),
        )],
        [(0, 0, 0)],
        [(
            "input",
            CellPort {
                position: (-1, 1, 0),
                direction: 0,
            },
        )],
    );
    let placed = cell.placed(Placement {
        origin: (10, 20, 2),
        rotation: Rotation::new(1),
    });
    assert_eq!(
        (
            placed.buildings[0].x,
            placed.buildings[0].y,
            placed.buildings[0].layer
        ),
        (9, 20, 2)
    );
    assert!(placed.footprint.contains(&(10, 20, 2)));
    assert_eq!(placed.port("input").unwrap().position, (9, 19, 2));
    assert_eq!(placed.port("input").unwrap().direction, 1);
    assert_eq!(placed.buildings[0].rotation.index(), 1);
}
