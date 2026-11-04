use crate::{geometry::ChunkPos, routing::RouteNode};
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub fn space_island(p: ChunkPos, node: &RouteNode, fluid: bool) -> Value {
    let r = node.input.index();
    let out: Vec<usize> = node
        .outputs
        .iter()
        .map(|d| (d.index() + 4 - r) % 4)
        .collect();
    let suffix = if node.zout != p.2 {
        format!(
            "Lift{}{}{}",
            (node.zout - p.2).abs(),
            if node.zout > p.2 { "Up" } else { "Down" },
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
    json!({"T":format!("Space{}_{}",if fluid{"Pipe"}else{"Belt"},suffix),"X":p.0,"Y":p.1,"Z":p.2,"R":r})
}
