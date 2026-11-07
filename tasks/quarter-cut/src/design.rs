//! Four-way cutting cell; four normalized quarters merge to one full belt.
use shapez_blueprint::{Building, Rotation};
/// Local cell occupies five columns and four rows, facing east.
pub fn cell(x: i32, y: i32) -> Vec<Building> {
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
        (4, 1, 1, "Merger3To1InternalVariant"),
        (4, 2, 1, "Merger3To1InternalVariant"),
        (4, 3, 0, "Merger3To1InternalVariant"),
    ] {
        b.push(Building::new(t, x + dx, y + dy, 0, Rotation::new(r)));
    }
    b
}
