use crate::{ChunkPos, Direction, GridBounds};
use std::collections::{HashMap, HashSet};

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
/// A nonempty transport junction. Construction checks coordinate and lift bounds;
/// subsequent outputs must preserve a planar junction or the lift's single exit.
#[derive(Clone)]
pub struct RouteNode {
    net: usize,
    incoming: Direction,
    layer: i16,
    output_layer: i16,
    /// Relative direction bits. Equivalent to a nonempty subset of forward,
    /// right and left for planar nodes, or one of four directions for lifts.
    outputs: u8,
}
impl RouteNode {
    /// Construct one legal exit. Space lifts span one or two island layers.
    pub fn new(
        net: usize,
        incoming: Direction,
        layer: i32,
        output_layer: i32,
        outgoing: Direction,
    ) -> anyhow::Result<Self> {
        let layer = i16::try_from(layer)?;
        let output_layer = i16::try_from(output_layer)?;
        anyhow::ensure!(
            layer.abs_diff(output_layer) <= 2,
            "Space lift exceeds two layers"
        );
        let mut node = Self {
            net,
            incoming,
            layer,
            output_layer,
            outputs: 0,
        };
        node.add_output(outgoing)?;
        Ok(node)
    }
    /// Add a planar branch or repeat the existing lift exit. Failure leaves
    /// the node unchanged; planar U-turns and branching lifts are rejected.
    pub fn add_output(&mut self, direction: Direction) -> anyhow::Result<()> {
        let bit = 1 << ((direction.index() + 4 - self.incoming.index()) % 4);
        anyhow::ensure!(self.layer != self.output_layer || bit != 4, "Planar U-turn");
        anyhow::ensure!(
            self.layer == self.output_layer || self.outputs == 0 || self.outputs == bit,
            "Lift cannot branch"
        );
        self.outputs |= bit;
        Ok(())
    }
    /// Owning transport net index.
    pub fn net(&self) -> usize {
        self.net
    }
    /// Direction of travel on entry.
    pub fn incoming(&self) -> Direction {
        self.incoming
    }
    /// Base layer, representable by the blueprint format.
    pub fn layer(&self) -> i16 {
        self.layer
    }
    /// Absolute layer at which all exits leave.
    pub fn output_layer(&self) -> i32 {
        i32::from(self.output_layer)
    }
    /// Exit directions in clockwise world order, without allocation.
    pub fn outgoing(&self) -> impl Iterator<Item = Direction> + '_ {
        Direction::ALL
            .into_iter()
            .filter(|d| self.outputs & (1 << ((d.index() + 4 - self.incoming.index()) % 4)) != 0)
    }
    /// Game definition suffix for this legal junction. The planar bit pattern
    /// directly selects the junction; lifts always contain exactly one exit.
    pub fn suffix(&self) -> std::borrow::Cow<'static, str> {
        if self.layer != self.output_layer {
            let direction = match self.outputs {
                1 => "Forward",
                2 => "Right",
                4 => "Backward",
                _ => "Left",
            };
            return format!(
                "Lift{}{}{direction}",
                self.layer.abs_diff(self.output_layer),
                if self.output_layer > self.layer {
                    "Up"
                } else {
                    "Down"
                }
            )
            .into();
        }
        match self.outputs {
            1 => "Forward",
            2 => "RightTurn",
            8 => "LeftTurn",
            3 => "RightFwdSplitter",
            9 => "LeftFwdSplitter",
            10 => "YSplitter",
            _ => "TripleSplitter",
        }
        .into()
    }
}
/// Transport junctions keyed by island-grid position.
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
