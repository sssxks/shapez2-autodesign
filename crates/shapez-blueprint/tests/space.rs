//! Exhaustive junction encoding and rejected-transition coverage.
use shapez_blueprint::space_island;
use shapez_model::{Direction, RouteNode};

#[test]
fn every_planar_subset_and_lift_direction_encodes() {
    for incoming in Direction::ALL {
        for mask in [1_u8, 2, 8, 3, 9, 10, 11] {
            let directions: Vec<_> = Direction::ALL
                .into_iter()
                .filter(|d| mask & (1 << ((d.index() + 4 - incoming.index()) % 4)) != 0)
                .collect();
            let mut node = RouteNode::new(0, incoming, 0, 0, directions[0]).unwrap();
            for &d in &directions {
                node.add_output(d).unwrap();
            }
            let expected = match mask {
                1 => "Forward",
                2 => "RightTurn",
                8 => "LeftTurn",
                3 => "RightFwdSplitter",
                9 => "LeftFwdSplitter",
                10 => "YSplitter",
                _ => "TripleSplitter",
            };
            for fluid in [false, true] {
                let island = space_island(7, -3, &node, fluid);
                assert_eq!(
                    island.definition,
                    format!("Space{}_{}", if fluid { "Pipe" } else { "Belt" }, expected)
                );
                assert_eq!(island.rotation.index(), incoming.index());
            }
            assert!(node.add_output(incoming.opposite()).is_err());
            assert_eq!(node.outgoing().count(), directions.len());
        }
        for delta in [-2, -1, 1, 2] {
            for outgoing in Direction::ALL {
                let mut node = RouteNode::new(0, incoming, 0, delta, outgoing).unwrap();
                node.add_output(outgoing).unwrap();
                assert!(node.add_output(outgoing.opposite()).is_err());
                let direction = ["Forward", "Right", "Backward", "Left"]
                    [(outgoing.index() + 4 - incoming.index()) % 4];
                assert_eq!(
                    node.suffix(),
                    format!(
                        "Lift{}{}{direction}",
                        delta.abs(),
                        if delta > 0 { "Up" } else { "Down" }
                    )
                );
            }
        }
    }
}

#[test]
fn rejects_unrepresentable_layers_and_unsupported_transitions() {
    for (base, output) in [(0, 3), (0, -3), (i32::MIN, 0), (0, i32::MAX)] {
        assert!(RouteNode::new(0, Direction::East, base, output, Direction::East).is_err());
    }
    assert!(RouteNode::new(0, Direction::East, 0, 0, Direction::West).is_err());
}
