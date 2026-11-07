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
                    position: (4, 4, 0),
                    direction: 1,
                },
            ),
        ],
    )
}

/// Central input and four full-rate output banks.
pub fn sockets() -> [shapez_model::FoundationSocket; 5] {
    use shapez_model::{Direction, FoundationSocket};
    [
        (Direction::North, 1, false, "input"),
        (Direction::North, 0, true, "nw"),
        (Direction::North, 2, true, "ne"),
        (Direction::South, 0, true, "sw"),
        (Direction::South, 2, true, "se"),
    ]
    .map(|(side, segment, output, role)| FoundationSocket {
        side,
        segment,
        output,
        role,
    })
}
