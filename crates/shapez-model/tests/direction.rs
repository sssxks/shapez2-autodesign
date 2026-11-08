//! Checked direction parsing and rotation contracts.
use serde::{Deserialize, Serialize};
use shapez_model::Direction;

/// Legacy numeric direction field, independent of the named domain encoding.
#[derive(Debug, Serialize, Deserialize)]
struct Numeric(#[serde(with = "shapez_model::numeric_direction")] Direction);

/// Numeric fields preserve all valid values and reject malformed headings.
#[test]
fn numeric_wire_is_checked_and_compatible() {
    for (index, direction) in Direction::ALL.into_iter().enumerate() {
        let wire = index.to_string();
        assert_eq!(serde_json::to_string(&Numeric(direction)).unwrap(), wire);
        assert_eq!(serde_json::from_str::<Numeric>(&wire).unwrap().0, direction);
        assert_eq!(Direction::from_index(index), Some(direction));
    }
    for invalid in [
        "4",
        "255",
        "18446744073709551615",
        "-1",
        "1.5",
        "null",
        "\"east\"",
    ] {
        assert!(
            serde_json::from_str::<Numeric>(invalid).is_err(),
            "{invalid}"
        );
    }
    assert_eq!(Direction::from_index(usize::MAX), None);
    assert_eq!(serde_json::to_string(&Direction::East).unwrap(), "\"east\"");
}

/// Composition and relative turns agree for every pair of cardinal headings.
#[test]
fn rotation_composes_without_overflow() {
    for incoming in Direction::ALL {
        for outgoing in Direction::ALL {
            assert_eq!(incoming.rotated(outgoing.turns_from(incoming)), outgoing);
        }
        assert_eq!(incoming.rotated(2), incoming.opposite());
        assert_eq!(
            incoming.rotated(usize::MAX),
            incoming.rotated(usize::MAX % 4)
        );
    }
}
