//! Convert validated transport junctions into space infrastructure.
use crate::{Island, Rotation};
use shapez_model::RouteNode;
/// Encode a legal junction at the supplied horizontal position. Its validated
/// base layer belongs to the node, so callers cannot substitute an invalid layer.
pub fn space_island(x: i32, y: i32, node: &RouteNode, fluid: bool) -> Island {
    Island::new(
        format!(
            "Space{}_{}",
            if fluid { "Pipe" } else { "Belt" },
            node.suffix()
        ),
        x,
        y,
        node.layer(),
        Rotation::new(node.incoming().index()),
    )
}
