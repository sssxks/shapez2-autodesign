//! Physical routes inside a foundation, independent of blueprint encoding.
use std::collections::{HashMap, HashSet};
/// Building coordinates; the layer is a native building layer.
pub type TilePoint = (i32, i32, i16);
/// Clockwise movement vectors in native tile coordinates.
pub const TILE_DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
/// Arrival state. A catcher must leave straight on the same layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileRouteStep {
    pub p: TilePoint,
    pub d: usize,
    pub catch: bool,
}
/// An explicit equivalence class declared by the layout, preserving stage flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExchangeGroup(pub u8);
/// Directed connection; external endpoints require native receiver/sender buildings.
#[derive(Clone, Debug)]
pub struct TileNet {
    pub a: TilePoint,
    pub ad: usize,
    pub b: TilePoint,
    pub bd: usize,
    pub input: bool,
    pub output: bool,
    pub exchange: Option<ExchangeGroup>,
}
/// A solved connection includes the selected destination after any exchanges.
#[derive(Clone, Debug)]
pub struct TileRoute {
    pub net: TileNet,
    pub steps: Vec<TileRouteStep>,
}
/// Occupancy and launcher interference are separate physical resources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TileResource {
    Occupancy(TilePoint),
    LauncherRange(TilePoint, usize),
}
/// Foundation floor plus directed approach corridors for external ports.
pub struct FoundationGeometry {
    pub min: (i32, i32),
    pub max: (i32, i32),
    pub layers: i16,
    pub exits: HashMap<TilePoint, TilePoint>,
}
impl FoundationGeometry {
    /// Rectangular floor of the native 1xN foundation family.
    pub fn strip(chunks: i32, layers: i16) -> Self {
        Self {
            min: (3, 3),
            max: (chunks * 20 - 4, 16),
            layers,
            exits: HashMap::new(),
        }
    }
    /// Whether a tile belongs to the floor (external sockets are handled separately).
    pub fn contains(&self, p: TilePoint) -> bool {
        (self.min.0..=self.max.0).contains(&p.0)
            && (self.min.1..=self.max.1).contains(&p.1)
            && (0..self.layers).contains(&p.2)
    }
}
/// Obstacles, geometry and connections are owned by the design.
pub struct TileRoutingProblem {
    pub geometry: FoundationGeometry,
    pub fixed: HashSet<TilePoint>,
    pub nets: Vec<TileNet>,
}
/// Resources consumed by a path, including all intermediate lift layers.
pub fn tile_resources(path: &[TileRouteStep]) -> Vec<TileResource> {
    let mut resources = Vec::new();
    for (i, s) in path.iter().enumerate() {
        let next = path.get(i + 1);
        let z = next.map_or(s.p.2, |n| n.p.2);
        for layer in s.p.2.min(z)..=s.p.2.max(z) {
            resources.push(TileResource::Occupancy((s.p.0, s.p.1, layer)));
        }
        if next.is_some_and(|n| (n.p.0 - s.p.0).abs() + (n.p.1 - s.p.1).abs() > 1) {
            let d = TILE_DIRECTIONS[s.d];
            for k in 1..=5 {
                resources.push(TileResource::LauncherRange(
                    (s.p.0 + d.0 * k, s.p.1 + d.1 * k, s.p.2),
                    s.d,
                ));
            }
        }
    }
    resources
}
