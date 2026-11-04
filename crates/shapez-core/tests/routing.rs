use shapez_core::{
    geometry::{ChunkPos, Direction, GridBounds},
    routing::*,
};
use std::collections::{HashMap, HashSet};

#[test]
fn routes_shared_tree_around_obstacles_and_across_layers() {
    let root = ChunkPos(0, 0, 0);
    let problem = TransportProblem {
        nets: vec![TransportNet {
            root,
            heading: Direction::East,
            ends: vec![
                (ChunkPos(3, 0, 0), Direction::East),
                (ChunkPos(3, 2, 0), Direction::South),
            ],
            fluid: false,
        }],
        blocked: HashSet::from([ChunkPos(1, 0, 0), ChunkPos(1, 1, 0), ChunkPos(1, 2, 0)]),
        reserved: HashMap::new(),
        bounds: GridBounds {
            min: root,
            max: ChunkPos(3, 2, 1),
        },
    };
    let plan = solve_transport(
        &problem,
        SearchOptions {
            attempts: 4,
            seed: 1,
        },
    )
    .unwrap();
    assert!(plan.values().all(|n| n.owner == 0));
    assert!(plan.contains_key(&root));
    assert!(plan.values().any(|n| n.outputs.len() > 1));
    assert!(plan.iter().any(|(p, n)| p.2 != n.zout));
    for (p, n) in &plan {
        for z in p.2.min(n.zout)..=p.2.max(n.zout) {
            assert!(!problem.blocked.contains(&ChunkPos(p.0, p.1, z)));
        }
    }
    assert!(plan.contains_key(&ChunkPos(3, 0, 0)) || plan.keys().any(|p| p.0 == 3 && p.1 == 0));
    assert!(plan.keys().any(|p| p.0 == 3 && p.1 == 2));
}

#[test]
fn rejects_unreachable_endpoint() {
    let problem = TransportProblem {
        nets: vec![TransportNet {
            root: ChunkPos(0, 0, 0),
            heading: Direction::East,
            ends: vec![(ChunkPos(2, 0, 0), Direction::East)],
            fluid: false,
        }],
        blocked: HashSet::from([ChunkPos(1, 0, 0)]),
        reserved: HashMap::new(),
        bounds: GridBounds {
            min: ChunkPos(0, 0, 0),
            max: ChunkPos(2, 0, 0),
        },
    };
    assert!(
        solve_transport(
            &problem,
            SearchOptions {
                attempts: 1,
                seed: 1
            }
        )
        .is_err()
    );
}
