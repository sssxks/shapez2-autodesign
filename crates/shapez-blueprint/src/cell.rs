//! Composable rigid production cells with transformed occupancy and named ports.
use crate::{Building, Rotation};
use anyhow::{Result, anyhow};
use shapez_model::TilePoint;
use std::collections::{BTreeMap, HashSet};
/// A connection tile adjacent to a cell and its direction of travel.
#[derive(Clone, Copy, Debug)]
pub struct CellPort {
    /// Port position in local tile coordinates.
    pub position: TilePoint,
    /// Port direction in clockwise quarter turns.
    pub direction: shapez_model::Direction,
}
/// Rigid clockwise rotation followed by translation, including a layer offset.
#[derive(Clone, Copy)]
pub struct Placement {
    /// Translation applied after rotation.
    pub origin: TilePoint,
    /// Clockwise quarter-turn orientation.
    pub rotation: Rotation,
}
impl Placement {
    /// Transform a local tile into the placed cell's coordinate system.
    pub fn point(self, p: TilePoint) -> TilePoint {
        let (x, y) = match self.rotation.index() {
            0 => (p.0, p.1),
            1 => (-p.1, p.0),
            2 => (-p.0, -p.1),
            _ => (p.1, -p.0),
        };
        (x + self.origin.0, y + self.origin.1, p.2 + self.origin.2)
    }
}
/// A reusable layout owns all of its occupied tiles, including non-anchor tiles.
#[derive(Clone)]
pub struct Cell {
    /// Buildings composing this layout.
    pub buildings: Vec<Building>,
    /// Occupied local tiles used for placement conflict checks.
    pub footprint: HashSet<TilePoint>,
    /// Named connection points in local coordinates.
    pub ports: BTreeMap<&'static str, CellPort>,
}
impl Cell {
    /// Construct a cell, adding occupied non-anchor tiles from device footprints.
    pub fn new(
        buildings: Vec<Building>,
        extra: impl IntoIterator<Item = TilePoint>,
        ports: impl IntoIterator<Item = (&'static str, CellPort)>,
    ) -> Self {
        let footprint = buildings
            .iter()
            .map(|b| (b.x, b.y, b.layer))
            .chain(extra)
            .collect();
        Self {
            buildings,
            footprint,
            ports: ports.into_iter().collect(),
        }
    }
    /// Place buildings, occupied tiles and ports with the same rigid transform.
    pub fn placed(&self, placement: Placement) -> Self {
        Self {
            buildings: self
                .buildings
                .iter()
                .cloned()
                .map(|mut b| {
                    (b.x, b.y, b.layer) = placement.point((b.x, b.y, b.layer));
                    b.rotation = Rotation::new(b.rotation.index() + placement.rotation.index());
                    b
                })
                .collect(),
            footprint: self.footprint.iter().map(|&p| placement.point(p)).collect(),
            ports: self
                .ports
                .iter()
                .map(|(&name, p)| {
                    (
                        name,
                        CellPort {
                            position: placement.point(p.position),
                            direction: p.direction.rotated(placement.rotation.index()),
                        },
                    )
                })
                .collect(),
        }
    }
    /// Resolve a named connection declared by the production cell.
    pub fn port(&self, name: &str) -> Result<CellPort> {
        self.ports
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("Unknown cell port {name}"))
    }
}
