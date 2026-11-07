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

#[test]
fn catcher_and_external_input_cannot_turn_lift_or_relaunch() {
    use shapez_model::TileRouteStep;
    for (catch, external) in [(true, false), (false, true)] {
        let s = TileRouteStep {
            p: (5, 5, 0),
            d: 0,
            catch,
        };
        assert!(s.allows_move(0, 0, false, external));
        assert!(!s.allows_move(1, 0, false, external));
        assert!(!s.allows_move(0, 1, false, external));
        assert!(!s.allows_move(0, 0, true, external));
    }
}
#[test]
fn lift_occupancy_and_launcher_interference_remain_distinct() {
    use shapez_model::TileRouteStep;
    let path = [
        TileRouteStep {
            p: (5, 5, 0),
            d: 0,
            catch: false,
        },
        TileRouteStep {
            p: (6, 5, 2),
            d: 0,
            catch: false,
        },
        TileRouteStep {
            p: (10, 5, 2),
            d: 0,
            catch: true,
        },
    ];
    let resources = tile_resources(&path);
    assert!(resources.contains(&TileResource::Occupancy((5, 5, 1))));
    assert!(resources.contains(&TileResource::LauncherRange((11, 5, 2), 0)));
    assert!(!resources.contains(&TileResource::Occupancy((8, 5, 2))));
}

#[test]
fn jump_can_land_directly_before_a_machine_input() {
    let mut p = problem();
    p.nets[0] = TileNet {
        a: (4, 8, 0),
        ad: 0,
        b: (7, 8, 0),
        bd: 0,
        input: false,
        output: false,
        exchange: None,
    };
    p.fixed = (3..=16)
        .flat_map(|x| (3..=16).map(move |y| (x, y, 0)))
        .filter(|p| *p != (4, 8, 0) && *p != (7, 8, 0))
        .collect();
    let routes = solve_tiles_negotiated(p, 1).unwrap();
    assert_eq!(routes[0].steps.len(), 2);
    assert!(routes[0].steps[1].catch);
}
