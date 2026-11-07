use shapez_model::{
    FoundationGeometry, SearchOptions, TileNet, TileResource, TileRoutingProblem, tile_resources,
};
use shapez_routing::{solve_tiles_negotiated, solve_tiles_sequential};
use std::collections::HashSet;
fn problem() -> TileRoutingProblem {
    TileRoutingProblem {
        geometry: FoundationGeometry::strip(1, 1),
        fixed: HashSet::new(),
        nets: vec![TileNet {
            a: (2, 8, 0),
            ad: 0,
            b: (17, 8, 0),
            bd: 0,
            input: true,
            output: true,
            exchange: None,
        }],
    }
}
#[test]
fn both_strategies_connect_external_ports_without_reusing_tiles() {
    let plans = [
        solve_tiles_sequential(
            problem(),
            SearchOptions {
                attempts: 30,
                seed: 1234567,
            },
        )
        .unwrap(),
        solve_tiles_negotiated(problem(), 30).unwrap(),
    ];
    for plan in plans {
        assert_eq!(plan.len(), 1);
        let route = &plan[0];
        assert_eq!(route.steps.first().unwrap().p, route.net.a);
        assert_eq!(route.steps.last().unwrap().p, route.net.b);
        let resources = tile_resources(&route.steps);
        let physical: Vec<_> = resources
            .iter()
            .filter(|r| matches!(r, TileResource::Occupancy(_)))
            .collect();
        assert_eq!(
            physical.len(),
            physical.iter().collect::<HashSet<_>>().len()
        );
    }
}
