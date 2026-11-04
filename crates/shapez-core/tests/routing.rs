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
            source: TransportEndpoint {
                position: root,
                direction: Direction::East,
            },
            sinks: vec![
                TransportEndpoint {
                    position: ChunkPos(3, 0, 0),
                    direction: Direction::East,
                },
                TransportEndpoint {
                    position: ChunkPos(3, 2, 0),
                    direction: Direction::South,
                },
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
    assert!(plan.values().all(|n| n.net == 0));
    assert!(plan.contains_key(&root));
    assert!(plan.values().any(|n| n.outgoing.len() > 1));
    assert!(plan.iter().any(|(p, n)| p.2 != n.output_layer));
    for (p, n) in &plan {
        for z in p.2.min(n.output_layer)..=p.2.max(n.output_layer) {
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
            source: TransportEndpoint {
                position: ChunkPos(0, 0, 0),
                direction: Direction::East,
            },
            sinks: vec![TransportEndpoint {
                position: ChunkPos(2, 0, 0),
                direction: Direction::East,
            }],
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
