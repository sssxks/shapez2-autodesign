//! Validated island-grid transport junctions and routing inputs.
use crate::{ChunkPos, Direction, GridBounds};
use anyhow::{Result, bail, ensure};
use std::collections::{HashMap, HashSet};

/// Direction of travel entering a source tile or leaving a sink tile.
#[derive(Clone, Copy, Debug)]
pub struct TransportEndpoint {
    /// Island-grid position of the port.
    pub position: ChunkPos,
    /// World heading of transported material.
    pub direction: Direction,
}
/// One source and its destination ports.
#[derive(Clone)]
pub struct TransportNet {
    /// Supply port.
    pub source: TransportEndpoint,
    /// Destination ports in routing order.
    pub sinks: Vec<TransportEndpoint>,
    /// Whether the network transports fluid rather than shapes.
    pub fluid: bool,
}
/// Nonempty subsets of the three legal planar exits.
#[derive(Clone, Copy)]
enum Planar {
    /// Straight exit.
    Forward,
    /// Clockwise turn.
    Right,
    /// Counterclockwise turn.
    Left,
    /// Straight and clockwise branches.
    ForwardRight,
    /// Straight and counterclockwise branches.
    ForwardLeft,
    /// Both side branches.
    Sides,
    /// All three branches.
    Triple,
}
impl Planar {
    /// Add a relative exit, rejecting a planar reversal.
    fn add(self, direction: Direction) -> Result<Self> {
        use Planar::*;
        Ok(match direction {
            Direction::West => bail!("Planar U-turn"),
            Direction::East => match self {
                Forward => Forward,
                Right | ForwardRight => ForwardRight,
                Left | ForwardLeft => ForwardLeft,
                Sides | Triple => Triple,
            },
            Direction::South => match self {
                Right => Right,
                Forward | ForwardRight => ForwardRight,
                Left | Sides => Sides,
                ForwardLeft | Triple => Triple,
            },
            Direction::North => match self {
                Left => Left,
                Forward | ForwardLeft => ForwardLeft,
                Right | Sides => Sides,
                ForwardRight | Triple => Triple,
            },
        })
    }
    /// Relative exit bitset used only for allocation-free iteration.
    fn mask(self) -> u8 {
        match self {
            Self::Forward => 1,
            Self::Right => 2,
            Self::Left => 8,
            Self::ForwardRight => 3,
            Self::ForwardLeft => 9,
            Self::Sides => 10,
            Self::Triple => 11,
        }
    }
    /// Game definition suffix for the connection.
    fn suffix(self) -> &'static str {
        match self {
            Self::Forward => "Forward",
            Self::Right => "RightTurn",
            Self::Left => "LeftTurn",
            Self::ForwardRight => "RightFwdSplitter",
            Self::ForwardLeft => "LeftFwdSplitter",
            Self::Sides => "YSplitter",
            Self::Triple => "TripleSplitter",
        }
    }
}
/// Signed spans registered by the game's SpacePathDefinitionData.LiftChunksByLayer.
#[derive(Clone, Copy)]
enum Lift {
    /// Descend two layers.
    DownTwo,
    /// Descend one layer.
    DownOne,
    /// Ascend one layer.
    UpOne,
    /// Ascend two layers.
    UpTwo,
}
impl Lift {
    /// Signed layer offset.
    fn delta(self) -> i32 {
        match self {
            Self::DownTwo => -2,
            Self::DownOne => -1,
            Self::UpOne => 1,
            Self::UpTwo => 2,
        }
    }
    /// Native suffix indexed by the relative exit heading.
    fn suffix(self, direction: Direction) -> &'static str {
        let names = match self {
            Self::DownTwo => [
                "Lift2DownForward",
                "Lift2DownRight",
                "Lift2DownBackward",
                "Lift2DownLeft",
            ],
            Self::DownOne => [
                "Lift1DownForward",
                "Lift1DownRight",
                "Lift1DownBackward",
                "Lift1DownLeft",
            ],
            Self::UpOne => [
                "Lift1UpForward",
                "Lift1UpRight",
                "Lift1UpBackward",
                "Lift1UpLeft",
            ],
            Self::UpTwo => [
                "Lift2UpForward",
                "Lift2UpRight",
                "Lift2UpBackward",
                "Lift2UpLeft",
            ],
        };
        names[direction.index()]
    }
}
/// A planar connection or a single-exit lift; neither admits empty outputs.
#[derive(Clone, Copy)]
enum Junction {
    /// One of the seven nonempty planar exit combinations.
    Planar(Planar),
    /// Supported vertical span and relative exit direction.
    Lift(Lift, Direction),
}
/// A legal transport junction with a validated base and destination layer.
#[derive(Clone)]
pub struct RouteNode {
    /// Transport net index.
    net: usize,
    /// World-space arrival heading.
    incoming: Direction,
    /// Base layer in blueprint coordinates.
    layer: i16,
    /// Nonempty connection topology.
    junction: Junction,
}
impl RouteNode {
    /// Construct one legal exit, checking coordinates and native lift spans.
    pub fn new(
        net: usize,
        incoming: Direction,
        layer: i32,
        output_layer: i32,
        outgoing: Direction,
    ) -> Result<Self> {
        let layer = i16::try_from(layer)?;
        let output_layer = i16::try_from(output_layer)?;
        let relative = Direction::ALL[(outgoing.index() + 4 - incoming.index()) % 4];
        let junction = match i32::from(output_layer) - i32::from(layer) {
            0 => Junction::Planar(match relative {
                Direction::East => Planar::Forward,
                Direction::South => Planar::Right,
                Direction::North => Planar::Left,
                Direction::West => bail!("Planar U-turn"),
            }),
            -2 => Junction::Lift(Lift::DownTwo, relative),
            -1 => Junction::Lift(Lift::DownOne, relative),
            1 => Junction::Lift(Lift::UpOne, relative),
            2 => Junction::Lift(Lift::UpTwo, relative),
            _ => bail!("Space lift exceeds two layers"),
        };
        Ok(Self {
            net,
            incoming,
            layer,
            junction,
        })
    }
    /// Add a planar branch or repeat a lift's exit. Failure preserves the node.
    pub fn add_output(&mut self, direction: Direction) -> Result<()> {
        let relative = Direction::ALL[(direction.index() + 4 - self.incoming.index()) % 4];
        self.junction = match self.junction {
            Junction::Planar(planar) => Junction::Planar(planar.add(relative)?),
            Junction::Lift(lift, exit) => {
                ensure!(relative == exit, "Lift cannot branch");
                Junction::Lift(lift, exit)
            }
        };
        Ok(())
    }
    /// Transport net index.
    pub fn net(&self) -> usize {
        self.net
    }
    /// Direction of travel on entry.
    pub fn incoming(&self) -> Direction {
        self.incoming
    }
    /// Validated base layer.
    pub fn layer(&self) -> i16 {
        self.layer
    }
    /// Absolute destination layer, derived from the connection.
    pub fn output_layer(&self) -> i32 {
        i32::from(self.layer)
            + match self.junction {
                Junction::Planar(_) => 0,
                Junction::Lift(lift, _) => lift.delta(),
            }
    }
    /// Exit directions in clockwise world order, without allocation.
    pub fn outgoing(&self) -> impl Iterator<Item = Direction> + '_ {
        let mask = match self.junction {
            Junction::Planar(p) => p.mask(),
            Junction::Lift(_, d) => 1 << d.index(),
        };
        Direction::ALL
            .into_iter()
            .filter(move |d| mask & (1 << ((d.index() + 4 - self.incoming.index()) % 4)) != 0)
    }
    /// Native game definition suffix, borrowed without allocation.
    pub fn suffix(&self) -> &'static str {
        match self.junction {
            Junction::Planar(p) => p.suffix(),
            Junction::Lift(lift, direction) => lift.suffix(direction),
        }
    }
}
/// Transport junctions keyed by island-grid position.
pub type RoutePlan = HashMap<ChunkPos, RouteNode>;
/// Caller-owned endpoint reservations, obstacles and search envelope.
pub struct TransportProblem {
    /// Networks to connect.
    pub nets: Vec<TransportNet>,
    /// Unavailable island positions.
    pub blocked: HashSet<ChunkPos>,
    /// Positions reserved for a particular net index.
    pub reserved: HashMap<ChunkPos, usize>,
    /// Inclusive search bounds.
    pub bounds: GridBounds,
}
/// Seeded search budget.
#[derive(Clone, Copy)]
pub struct SearchOptions {
    /// Number of alternative net orderings to try.
    pub attempts: usize,
    /// Deterministic shuffle seed.
    pub seed: u64,
}
