//! Task-owned wiring between priority cells and the external interface.
use crate::design::Design;
use anyhow::Result;
use shapez_blueprint::Building;
use shapez_model::{FoundationGeometry, SearchOptions, TileNet as Net, TileRoutingProblem};
use shapez_routing::solve_tiles_sequential;
use std::collections::HashSet;
pub fn connect(
    buildings: &mut Vec<Building>,
    cells: &[shapez_blueprint::Cell],
    design: &Design,
) -> Result<()> {
    let mut nets = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        let a = cell.port("primary")?;
        let b = cell.port("secondary")?;
        let out = cell.port("output")?;
        let k = i as i32;
        let (primary, primary_direction) = design.primary.port(k);
        let (secondary, secondary_direction) = design.secondary.port(k);
        nets.push(Net {
            a: (primary.0, primary.1, 0),
            ad: primary_direction,
            b: a.position,
            bd: a.direction,
            input: true,
            output: false,
            exchange: None,
        });
        nets.push(Net {
            a: (secondary.0, secondary.1, 0),
            ad: secondary_direction,
            b: b.position,
            bd: b.direction,
            input: true,
            output: false,
            exchange: None,
        });
        nets.push(Net {
            a: out.position,
            ad: out.direction,
            b: (17, 8 + k, 0),
            bd: 0,
            input: false,
            output: true,
            exchange: None,
        });
    }
    let fixed: HashSet<_> = cells
        .iter()
        .flat_map(|c| c.footprint.iter().copied())
        .collect();
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
