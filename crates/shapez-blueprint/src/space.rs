use crate::{Island, Rotation};
use shapez_model::{ChunkPos, RouteNode};
use std::collections::BTreeSet;
pub fn space_island(p: ChunkPos, node: &RouteNode, fluid: bool) -> Island {
    let r = node.incoming.index();
    let out: Vec<usize> = node
        .outgoing
        .iter()
        .map(|d| (d.index() + 4 - r) % 4)
        .collect();
    let suffix = if node.output_layer != p.2 {
        format!(
            "Lift{}{}{}",
            (node.output_layer - p.2).abs(),
            if node.output_layer > p.2 {
                "Up"
            } else {
                "Down"
            },
            match out[0] {
                0 => "Forward",
                1 => "Right",
                2 => "Backward",
                _ => "Left",
            }
        )
    } else {
        let set: BTreeSet<_> = out.into_iter().collect();
        match set.iter().copied().collect::<Vec<_>>().as_slice() {
            [0] => "Forward",
            [1] => "RightTurn",
            [3] => "LeftTurn",
            [0, 1] => "RightFwdSplitter",
            [0, 3] => "LeftFwdSplitter",
            [1, 3] => "YSplitter",
            [0, 1, 3] => "TripleSplitter",
            _ => panic!("invalid space junction"),
        }
        .to_owned()
    };
    Island::new(
        format!("Space{}_{}", if fluid { "Pipe" } else { "Belt" }, suffix),
        p.0,
        p.1,
        i16::try_from(p.2).expect("routing layer fits game coordinates"),
        Rotation::new(r),
    )
}
