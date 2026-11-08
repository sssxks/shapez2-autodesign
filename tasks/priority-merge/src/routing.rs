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
    let geometry = FoundationGeometry::strip(1, 1);
    let sockets = design.sockets();
    for (i, cell) in cells.iter().enumerate() {
        let a = cell.port("primary")?;
        let b = cell.port("secondary")?;
        let out = cell.port("output")?;
        let k = i as i32;
        let primary = geometry.socket_tile(sockets[0], k, 0);
        let secondary = geometry.socket_tile(sockets[1], k, 0);
        nets.push(Net {
            a: primary,
            ad: sockets[0].direction(),
            b: a.position,
            bd: a.direction,
            input: true,
            output: false,
            exchange: None,
        });
        nets.push(Net {
            a: secondary,
            ad: sockets[1].direction(),
            b: b.position,
            bd: b.direction,
            input: true,
            output: false,
            exchange: None,
        });
        nets.push(Net {
            a: out.position,
            ad: out.direction,
            b: geometry.socket_tile(sockets[2], k, 0),
            bd: shapez_model::Direction::East,
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
            geometry,
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
