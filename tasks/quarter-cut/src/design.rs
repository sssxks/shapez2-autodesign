//! Four-way cutting cell; four normalized quarters merge to one full belt.
use shapez_blueprint::{Building, Cell, CellPort, Rotation};
/// Local cell occupies five columns and four rows, facing east.
pub fn cell(factory: bool) -> Cell {
    let mut b = vec![];
    for (dx, dy, r, t) in [
        (0, 2, 0, "CutterDefaultInternalVariant"),
        (1, 1, 0, "RotatorOneQuadInternalVariant"),
        (1, 2, 0, "RotatorOneQuadInternalVariant"),
        (2, 1, 0, "CutterDefaultInternalVariant"),
        (2, 2, 0, "CutterDefaultInternalVariantMirrored"),
        (3, 0, 0, "RotatorOneQuadInternalVariant"),
        (3, 1, 0, "BeltDefaultForwardInternalVariant"),
        (3, 2, 0, "RotatorOneQuadCCWInternalVariant"),
        (3, 3, 0, "RotatorHalfInternalVariant"),
        (4, 0, 0, "BeltDefaultLeftInternalVariantMirrored"),
        (4, 1, 1, "Merger2To1LInternalVariantMirrored"),
        (4, 2, 1, "Merger2To1LInternalVariantMirrored"),
        (4, 3, 0, "Merger2To1LInternalVariant"),
    ] {
        b.push(Building::new(t, dx, dy, 0, Rotation::new(r)));
    }
    if factory {
        let last = &mut b[12];
        last.definition = "Merger2To1LInternalVariantMirrored".into();
        last.rotation = Rotation::new(1);
    }
    Cell::new(
        b,
        [(0, 1, 0), (2, 0, 0), (2, 3, 0)],
        [
            (
                "input",
                CellPort {
                    position: (-1, 2, 0),
                    direction: 0,
                },
            ),
            (
                "output",
                CellPort {
                    position: if factory { (4, 4, 0) } else { (5, 3, 0) },
                    direction: usize::from(factory),
                },
            ),
        ],
    )
}

/// Reflect physical connectors without changing the shape rotation operations.
pub fn reflected_cell(factory: bool) -> Cell {
    reflect(cell(factory))
}

/// Merge the two already aligned quarters before the final three-way junction.
/// All three final inputs are used; its full belt exits from an interior row.
pub fn middle_cell(row: i32) -> Cell {
    assert!(row == 1 || row == 2);
    let mut result = cell(false);
    result.buildings[2].definition = "RotatorOneQuadCCWInternalVariant".into();
    result.buildings[8].definition = "RotatorOneQuadInternalVariant".into();
    for b in &mut result.buildings {
        let (kind, rotation) = match (b.x, b.y) {
            (3, 1) if row == 1 => ("Merger2To1LInternalVariantMirrored", 0),
            (3, 1) => ("BeltDefaultLeftInternalVariantMirrored", 0),
            (3, 2) if row == 1 => ("BeltDefaultLeftInternalVariant", 0),
            (3, 2) => ("Merger2To1LInternalVariant", 0),
            (4, y) if y == row => ("Merger3To1InternalVariant", 0),
            (4, 1) => ("BeltDefaultForwardInternalVariant", 1),
            (4, 2) => ("BeltDefaultForwardInternalVariant", 3),
            (4, 3) => ("BeltDefaultLeftInternalVariant", 0),
            _ => continue,
        };
        b.definition = kind.into();
        b.rotation = Rotation::new(rotation);
    }
    result.ports.insert(
        "output",
        CellPort {
            position: (5, row, 0),
            direction: 0,
        },
    );
    result
}

pub fn variant(kind: u8, mirrored: bool) -> Cell {
    let result = match kind {
        0 => cell(false),
        1 => cell(true),
        2 => middle_cell(1),
        3 => middle_cell(2),
        _ => panic!("Unknown cell variant"),
    };
    if mirrored { reflect(result) } else { result }
}

fn reflect(mut cell: Cell) -> Cell {
    for b in &mut cell.buildings {
        b.y = 3 - b.y;
        b.rotation = Rotation::new((4 - b.rotation.index()) % 4);
        if ["CutterDefault", "BeltDefaultLeft", "Merger2To1L"]
            .iter()
            .any(|prefix| b.definition.starts_with(prefix))
        {
            b.definition = if let Some(normal) = b.definition.strip_suffix("Mirrored") {
                normal.into()
            } else {
                format!("{}Mirrored", b.definition)
            };
        }
    }
    cell.footprint = cell
        .footprint
        .into_iter()
        .map(|(x, y, z)| (x, 3 - y, z))
        .collect();
    for port in cell.ports.values_mut() {
        port.position.1 = 3 - port.position.1;
        port.direction = (4 - port.direction) % 4;
    }
    cell
}

/// Library icon shared by every four-quarter factory representation.
pub fn icon() -> shapez_blueprint::BlueprintIcon {
    shapez_blueprint::BlueprintIcon {
        components: ["CuCuCuCu", "Cu------", "Cu------", "Cu------"]
            .map(|shape| Some(shapez_blueprint::IconComponent::Shape(shape.into()))),
    }
}
