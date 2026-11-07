use shapez_model::{FoundationGeometry, TileNet, TileRoutingProblem};
use shapez_routing::{reachable_targets, source_blockers};
use std::collections::HashSet;

#[test]
fn identifies_terminal_reservations_separating_a_corridor() {
    let geometry = FoundationGeometry::strip(1, 1);
    let fixed: HashSet<_> = (3..=16)
        .flat_map(|x| (3..=16).filter(|&y| y != 8).map(move |y| (x, y, 0)))
        .collect();
    let mut nets = vec![TileNet {
        a: (3, 8, 0),
        ad: 0,
        b: (16, 8, 0),
        bd: 0,
        input: false,
        output: false,
        exchange: None,
    }];
    for x in 8..=12 {
        nets.push(TileNet {
            a: (x, 8, 0),
            ad: 0,
            b: (x, 8, 0),
            bd: 0,
            input: false,
            output: false,
            exchange: None,
        });
    }
    let problem = TileRoutingProblem {
        geometry,
        fixed,
        nets,
    };
    assert!(reachable_targets(&problem)[0].is_empty());
    let blockers = source_blockers(&problem, 0).expect("Production floor permits passage");
    assert!(!blockers.is_empty());
    assert!(blockers.iter().all(|&id| (1..=5).contains(&id)));
}
