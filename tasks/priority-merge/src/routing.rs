//! Task-owned wiring between priority cells and the external interface.
use crate::design::Design;
use anyhow::Result;
use shapez_blueprint::Building;
use shapez_model::{FoundationGeometry, SearchOptions, TileNet as Net, TileRoutingProblem};
use shapez_routing::solve_tiles_sequential;
use std::collections::HashSet;
pub fn connect(
    buildings: &mut Vec<Building>,
    centers: &[(i32, i32); 4],
    design: &Design,
) -> Result<()> {
    let mut nets = Vec::new();
    for (i, &(x, y)) in centers.iter().enumerate() {
        let k = i as i32;
        let (primary, primary_direction) = design.primary.port(k);
        let (secondary, secondary_direction) = design.secondary.port(k);
        nets.push(Net {
            a: (primary.0, primary.1, 0),
            ad: primary_direction,
            b: (x - 2, y - 2, 0),
            bd: 1,
            input: true,
            output: false,
            exchange: None,
        });
        nets.push(Net {
            a: (secondary.0, secondary.1, 0),
            ad: secondary_direction,
            b: (x + 1, y + 1, 0),
            bd: 2,
            input: true,
            output: false,
            exchange: None,
        });
        nets.push(Net {
            a: (x - 1, y + 2, 0),
            ad: 1,
            b: (17, 8 + k, 0),
            bd: 0,
            input: false,
            output: true,
            exchange: None,
        });
    }
    let fixed: HashSet<_> = buildings.iter().map(|b| (b.x, b.y, b.layer)).collect();
    for route in solve_tiles_sequential(
        TileRoutingProblem {
            geometry: FoundationGeometry::strip(1, 1),
            fixed,
            nets,
        },
        SearchOptions {
            attempts: 2000,
            seed: 1234567,
        },
    )? {
        shapez_blueprint::route_buildings(&route, buildings);
    }
    Ok(())
}
