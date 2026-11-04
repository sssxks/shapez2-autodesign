//! Deterministic routing of shared transport trees in a bounded island grid.
use anyhow::{Result, bail, ensure};
use shapez_model::{
    ChunkPos, Direction, GridBounds, RouteNode, RoutePlan, SearchOptions, TransportEndpoint,
    TransportNet, TransportProblem,
};
use std::{
    cmp::Reverse,
    collections::{BTreeSet, BinaryHeap, HashMap, HashSet},
};
type State = (ChunkPos, Direction);
fn route(
    netid: usize,
    net: &TransportNet,
    end: TransportEndpoint,
    blocked: &HashSet<ChunkPos>,
    reserved: &HashMap<ChunkPos, usize>,
    nodes: &mut RoutePlan,
    bounds: GridBounds,
) -> Result<()> {
    let mut occupied = HashMap::new();
    for (&p, node) in nodes.iter() {
        for z in p.2.min(node.output_layer)..=p.2.max(node.output_layer) {
            occupied.insert(ChunkPos(p.0, p.1, z), p);
        }
    }
    let allowed = |p: ChunkPos| {
        bounds.contains(p)
            && !blocked.contains(&p)
            && reserved.get(&p).is_none_or(|&id| id == netid)
    };
    let mut heap = BinaryHeap::new();
    let mut cost = HashMap::<State, i32>::new();
    let mut prev = HashMap::<State, (State, Direction, i32)>::new();
    let start = (net.source.position, net.source.direction);
    cost.insert(start, 0);
    heap.push(Reverse((0, start)));
    let mut finish = None;
    while let Some(Reverse((c, s))) = heap.pop() {
        if cost.get(&s) != Some(&c) {
            continue;
        }
        let (p, h) = s;
        if (p.0, p.1) == (end.position.0, end.position.1)
            && (p.2 != end.position.2 || end.direction != h.opposite())
            && (p.2.min(end.position.2)..=p.2.max(end.position.2)).all(|z| {
                allowed(ChunkPos(p.0, p.1, z))
                    && occupied.get(&ChunkPos(p.0, p.1, z)).is_none_or(|base| {
                        *base == p
                            && nodes[base].net == netid
                            && nodes[base].incoming == h
                            && nodes[base].output_layer == end.position.2
                    })
            })
        {
            finish = Some(s);
            break;
        }
        if !allowed(p) {
            continue;
        }
        if let Some(base) = occupied.get(&p)
            && (*base != p || nodes[base].net != netid || nodes[base].incoming != h)
        {
            continue;
        }
        for z in bounds.min.2..=bounds.max.2 {
            for d in Direction::ALL {
                let (dx, dy) = d.delta();
                if z == p.2 && d == h.opposite() {
                    continue;
                }
                if let Some(node) = nodes.get(&p)
                    && (node.output_layer != z || z != p.2 && !node.outgoing.contains(&d))
                {
                    continue;
                }
                if (p.2.min(z)..=p.2.max(z)).any(|zz| {
                    !allowed(ChunkPos(p.0, p.1, zz))
                        || occupied
                            .get(&ChunkPos(p.0, p.1, zz))
                            .is_some_and(|base| *base != p)
                }) {
                    continue;
                }
                let next = (ChunkPos(p.0 + dx, p.1 + dy, z), d);
                if !allowed(next.0) {
                    continue;
                }
                let step = if nodes.get(&p).is_some_and(|v| v.outgoing.contains(&d)) {
                    1
                } else {
                    10 + if z != p.2 { 8 + 4 * (z - p.2).abs() } else { 0 }
                        + if d != h { 1 } else { 0 }
                };
                let nc = c + step;
                if cost.get(&next).is_none_or(|v| nc < *v) {
                    cost.insert(next, nc);
                    prev.insert(next, (s, d, z));
                    heap.push(Reverse((nc, next)));
                }
            }
        }
    }
    let Some(mut s) = finish else {
        bail!("Cannot route net {netid} to {end:?}")
    };
    let mut steps = vec![(s, end.direction, end.position.2)];
    while s != start {
        let &(parent, d, z) = prev.get(&s).unwrap();
        steps.push((parent, d, z));
        s = parent;
    }
    let mut seen = HashSet::new();
    for &(s, _, z) in &steps {
        for zz in s.0.2.min(z)..=s.0.2.max(z) {
            ensure!(
                seen.insert(ChunkPos(s.0.0, s.0.1, zz)),
                "Self-crossing route"
            );
        }
    }
    for ((p, h), d, z) in steps {
        let node = nodes.entry(p).or_insert_with(|| RouteNode {
            net: netid,
            incoming: h,
            output_layer: z,
            outgoing: BTreeSet::new(),
        });
        node.outgoing.insert(d);
    }
    Ok(())
}

pub fn solve_transport(problem: &TransportProblem, options: SearchOptions) -> Result<RoutePlan> {
    ensure!(
        options.attempts > 0,
        "Routing requires at least one attempt"
    );
    ensure!(
        problem.bounds.min.0 <= problem.bounds.max.0
            && problem.bounds.min.1 <= problem.bounds.max.1
            && problem.bounds.min.2 <= problem.bounds.max.2,
        "Invalid routing bounds"
    );
    let nets = &problem.nets;
    for net in nets {
        ensure!(
            problem.bounds.contains(net.source.position)
                && net
                    .sinks
                    .iter()
                    .all(|e| problem.bounds.contains(e.position)),
            "Endpoint outside routing bounds"
        );
    }
    let mut best: Option<RoutePlan> = None;
    let mut last_error = String::new();
    for attempt in 0..options.attempts {
        let mut order: Vec<usize> = (0..nets.len()).collect();
        let mut seed = options.seed.wrapping_add(attempt as u64);
        if attempt > 0 {
            for i in (1..order.len()).rev() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                order.swap(i, (seed >> 32) as usize % (i + 1));
            }
        }
        let mut nodes = RoutePlan::new();
        let mut good = true;
        for id in order {
            for &end in &nets[id].sinks {
                if let Err(e) = route(
                    id,
                    &nets[id],
                    end,
                    &problem.blocked,
                    &problem.reserved,
                    &mut nodes,
                    problem.bounds,
                ) {
                    last_error = e.to_string();
                    good = false;
                    break;
                }
            }
            if !good {
                break;
            }
        }
        if good && best.as_ref().is_none_or(|b| nodes.len() < b.len()) {
            best = Some(nodes);
        }
    }
    best.ok_or_else(|| anyhow::anyhow!("Routing failed: {last_error}"))
}
