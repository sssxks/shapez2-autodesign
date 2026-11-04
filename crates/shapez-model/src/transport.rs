use crate::{ChunkPos, Direction, GridBounds};
use std::collections::{BTreeSet, HashMap, HashSet};

/// Direction of travel entering a source tile or leaving a sink tile.
#[derive(Clone, Copy, Debug)]
pub struct TransportEndpoint {
    pub position: ChunkPos,
    pub direction: Direction,
}
#[derive(Clone)]
pub struct TransportNet {
    pub source: TransportEndpoint,
    pub sinks: Vec<TransportEndpoint>,
    pub fluid: bool,
}
#[derive(Clone)]
pub struct RouteNode {
    pub net: usize,
    pub incoming: Direction,
    pub output_layer: i32,
    pub outgoing: BTreeSet<Direction>,
}
pub type RoutePlan = HashMap<ChunkPos, RouteNode>;

/// The caller owns endpoint reservations, obstacles and the search envelope.
pub struct TransportProblem {
    pub nets: Vec<TransportNet>,
    pub blocked: HashSet<ChunkPos>,
    pub reserved: HashMap<ChunkPos, usize>,
    pub bounds: GridBounds,
}
#[derive(Clone, Copy)]
pub struct SearchOptions {
    pub attempts: usize,
    pub seed: u64,
}
