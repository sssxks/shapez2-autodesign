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

/// Four-lane socket on an edge of a 1xN foundation.
#[derive(Clone, Copy)]
pub struct FoundationSocket {
    pub side: crate::Direction,
    pub segment: i32,
    pub output: bool,
    pub role: &'static str,
}
/// Serialized physical lane used by native experiment bindings.
#[derive(Clone, serde::Serialize)]
pub struct FactoryBeltPort {
    pub role: &'static str,
    pub lane: usize,
    pub position: TilePoint,
    pub output: bool,
}
impl FoundationSocket {
    /// Direction of travel at this socket.
    pub fn direction(self) -> usize {
        if self.output {
            self.side.index()
        } else {
            self.side.opposite().index()
        }
    }
}
impl FoundationGeometry {
    /// Native tile of one socket lane.
    pub fn socket_tile(&self, socket: FoundationSocket, lane: i32, layer: i16) -> TilePoint {
        use crate::Direction;
        let along = 8 + 20 * socket.segment + lane;
        let (x, y) = match socket.side {
            Direction::North => (along, self.min.1 - 1),
            Direction::South => (along, self.max.1 + 1),
            Direction::West => (self.min.0 - 1, along),
            Direction::East => (self.max.0 + 1, along),
        };
        (x, y, layer)
    }
    /// Adjacent space-belt chunk and its direction for a test fixture.
    pub fn socket_harness(&self, socket: FoundationSocket) -> (i32, i32, usize) {
        use crate::Direction;
        let (x, y) = match socket.side {
            Direction::North => (socket.segment, -1),
            Direction::South => (socket.segment, 1),
            Direction::West => (-1, socket.segment),
            Direction::East => ((self.max.0 + 4) / 20, socket.segment),
        };
        (x, y, socket.direction())
    }
    /// Expand socket declarations into stable layer-major lane identities.
    pub fn ports(&self, sockets: &[FoundationSocket]) -> Vec<FactoryBeltPort> {
        sockets
            .iter()
            .flat_map(|&socket| {
                (0..self.layers).flat_map(move |z| {
                    (0..4).map(move |lane| FactoryBeltPort {
                        role: socket.role,
                        lane: z as usize * 4 + lane as usize,
                        position: self.socket_tile(socket, lane, z),
                        output: socket.output,
                    })
                })
            })
            .collect()
    }
}

impl TileRouteStep {
    /// Native movement rule shared by sequential and negotiated search.
    pub fn allows_move(
        self,
        outgoing: usize,
        layer: i16,
        jump: bool,
        external_input: bool,
    ) -> bool {
        let lift = layer != self.p.2;
        let locked = self.catch || external_input;
        if locked && (lift || outgoing != self.d) {
            return false;
        }
        if !lift && (outgoing + 4 - self.d) % 4 == 2 {
            return false;
        }
        !jump || (!lift && outgoing == self.d && !locked)
    }
    /// Native final-tile constraints at a device or external sender.
    pub fn allows_destination(self, direction: usize, external_output: bool) -> bool {
        (!(self.catch || external_output) || self.d == direction)
            && (direction + 4 - self.d) % 4 != 2
    }
}
