//! Congestion negotiation for building-grid routes.
use anyhow::{Result, bail};
use shapez_model::{
    FoundationGeometry, TILE_DIRECTIONS as D, TileNet as Net, TilePoint as Point, TileResource,
    TileRoute, TileRouteStep as State, TileRoutingProblem, tile_resources as resources,
};

/// Destination indices reachable with production buildings and source tiles reserved.
pub fn reachable_targets(problem: &TileRoutingProblem) -> Vec<Vec<usize>> {
    let mut fixed = problem.fixed.clone();
    fixed.extend(problem.nets.iter().map(|n| n.a));
    problem
        .nets
        .iter()
        .enumerate()
        .map(|(i, net)| {
            let mut remaining: Vec<_> = problem
                .nets
                .iter()
                .enumerate()
                .filter(|(j, n)| *j == i || net.exchange.is_some() && n.exchange == net.exchange)
                .map(|(j, _)| j)
                .collect();
            let mut result = Vec::new();
            while !remaining.is_empty() {
                let targets: Vec<_> = remaining
                    .iter()
                    .map(|&j| (problem.nets[j].b, problem.nets[j].bd))
                    .collect();
                let Some(path) = search(
                    net,
                    &problem.geometry,
                    &fixed,
                    &HashMap::new(),
                    &HashMap::new(),
                    1,
                    &targets,
                ) else {
                    break;
                };
                let p = path.last().unwrap().p;
                let k = remaining
                    .iter()
                    .position(|&j| problem.nets[j].b == p)
                    .unwrap();
                result.push(remaining.remove(k));
            }
            result
        })
        .collect()
}
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
};

/// Match equivalent destinations before routing, preserving scarce accessible exits.
pub fn assign_reachable_targets(problem: &mut TileRoutingProblem) -> Result<()> {
    let mut fixed = problem.fixed.clone();
    fixed.extend(problem.nets.iter().map(|n| n.a));
    let mut groups = Vec::new();
    for net in &problem.nets {
        if let Some(group) = net.exchange
            && !groups.contains(&group)
        {
            groups.push(group);
        }
    }
    for group in groups {
        let ids: Vec<_> = problem
            .nets
            .iter()
            .enumerate()
            .filter(|(_, n)| n.exchange == Some(group))
            .map(|(i, _)| i)
            .collect();
        let targets: Vec<_> = ids
            .iter()
            .map(|&i| (problem.nets[i].b, problem.nets[i].bd))
            .collect();
        let mut edges = Vec::new();
        for &id in &ids {
            let mut remaining: Vec<_> = (0..targets.len()).collect();
            let mut reachable = Vec::new();
            while !remaining.is_empty() {
                let candidates: Vec<_> = remaining.iter().map(|&j| targets[j]).collect();
                let Some(path) = search(
                    &problem.nets[id],
                    &problem.geometry,
                    &fixed,
                    &HashMap::new(),
                    &HashMap::new(),
                    1,
                    &candidates,
                ) else {
                    break;
                };
                let p = path.last().unwrap().p;
                let j = remaining.iter().position(|&j| targets[j].0 == p).unwrap();
                reachable.push(remaining.remove(j));
            }
            anyhow::ensure!(
                !reachable.is_empty(),
                "Connection {id} at {:?} cannot reach any equivalent target",
                problem.nets[id].a
            );
            edges.push(reachable);
        }
        fn augment(
            i: usize,
            edges: &[Vec<usize>],
            seen: &mut [bool],
            matched: &mut [Option<usize>],
        ) -> bool {
            for &j in &edges[i] {
                if seen[j] {
                    continue;
                }
                seen[j] = true;
                if matched[j].is_none_or(|previous| augment(previous, edges, seen, matched)) {
                    matched[j] = Some(i);
                    return true;
                }
            }
            false
        }
        let mut matched = vec![None; ids.len()];
        let mut order: Vec<_> = (0..ids.len()).collect();
        order.sort_by_key(|&i| edges[i].len());
        for i in order {
            anyhow::ensure!(
                augment(i, &edges, &mut vec![false; ids.len()], &mut matched),
                "No reachable destination matching in group {:?}; source {}",
                group,
                ids[i]
            );
        }
        for (j, i) in matched.into_iter().enumerate() {
            let net = &mut problem.nets[ids[i.unwrap()]];
            (net.b, net.bd) = targets[j];
            net.exchange = None;
        }
        eprintln!("Matched {} connections in group {:?}", ids.len(), group);
    }
    Ok(())
}
/// Find a directional path, pricing conflicts for the next negotiated routing round.
fn search(
    n: &Net,
    geometry: &FoundationGeometry,
    fixed: &HashSet<Point>,
    usage: &HashMap<TileResource, i32>,
    history: &HashMap<TileResource, i32>,
    penalty: i32,
    targets: &[(Point, usize)],
) -> Option<Vec<State>> {
    let start = State {
        p: n.a,
        d: n.ad,
        catch: false,
    };
    let mut q = BinaryHeap::from([Reverse((0, start))]);
    let mut prev = HashMap::from([(start, None)]);
    let mut costs = HashMap::from([(start, 0)]);
    let legal = |p: Point| {
        (!fixed.contains(&p) || p == n.a)
            && (geometry.contains(p) || p == n.a || targets.iter().any(|v| v.0 == p))
    };
    let price = |p: TileResource| {
        10 + usage.get(&p).copied().unwrap_or(0) * penalty + history.get(&p).copied().unwrap_or(0)
    };
    while let Some(Reverse((cost, s))) = q.pop() {
        if costs[&s] != cost {
            continue;
        }
        if targets
            .iter()
            .any(|&(p, bd)| s.p == p && s.allows_destination(bd, n.output))
        {
            let mut path = vec![s];
            let mut cur = s;
            while let Some(p) = prev[&cur] {
                path.push(p);
                cur = p;
            }
            path.reverse();
            let r = resources(&path)
                .into_iter()
                .filter(|p| matches!(p, TileResource::Occupancy(_)))
                .collect::<Vec<_>>();
            if r.iter().copied().collect::<HashSet<_>>().len() == r.len() {
                return Some(path);
            }
            continue;
        }
        for z in 0..geometry.layers {
            let lift = z != s.p.2;
            if lift && (s.catch || s.p == n.a && n.input) {
                continue;
            }
            if lift && (s.p.2.min(z)..=s.p.2.max(z)).any(|k| !legal((s.p.0, s.p.1, k))) {
                continue;
            }
            for (d, v) in D.iter().enumerate() {
                for len in 1..=5 {
                    if !s.allows_move(d, z, len > 1, s.p == n.a && n.input) {
                        continue;
                    }
                    let p = (s.p.0 + v.0 * len, s.p.1 + v.1 * len, z);
                    if geometry.exits.get(&s.p).is_some_and(|&port| p != port) {
                        continue;
                    }
                    if !legal(p) || (targets.iter().any(|v| v.0 == p) && (lift || len > 1)) {
                        continue;
                    }
                    let mut ancestor = Some(s);
                    let mut repeat = false;
                    while let Some(a) = ancestor {
                        if a.p == p
                            || lift
                                && a != s
                                && a.p.0 == s.p.0
                                && a.p.1 == s.p.1
                                && (s.p.2.min(z)..=s.p.2.max(z)).contains(&a.p.2)
                        {
                            repeat = true;
                            break;
                        }
                        ancestor = prev[&a];
                    }
                    if repeat {
                        continue;
                    }
                    if len > 1 {
                        let mut child = s;
                        while let Some(parent) = prev[&child] {
                            if parent.p.2 == z
                                && parent.d == d
                                && (parent.p.0 - child.p.0).abs() + (parent.p.1 - child.p.1).abs()
                                    > 1
                            {
                                let aligned = if d % 2 == 0 {
                                    parent.p.1 == s.p.1
                                } else {
                                    parent.p.0 == s.p.0
                                };
                                if aligned
                                    && (parent.p.0 - s.p.0).abs() + (parent.p.1 - s.p.1).abs() < 5
                                {
                                    repeat = true;
                                    break;
                                }
                            }
                            child = parent;
                        }
                        if repeat {
                            continue;
                        }
                    }
                    let next = State {
                        p,
                        d,
                        catch: len > 1,
                    };
                    let extra = if lift {
                        (s.p.2.min(z)..=s.p.2.max(z))
                            .filter(|&k| k != s.p.2)
                            .map(|k| price(TileResource::Occupancy((s.p.0, s.p.1, k))))
                            .sum::<i32>()
                    } else if len > 1 {
                        (1..=5)
                            .map(|k| {
                                price(TileResource::LauncherRange(
                                    (s.p.0 + v.0 * k, s.p.1 + v.1 * k, z),
                                    d,
                                )) - 9
                            })
                            .sum()
                    } else {
                        0
                    };
                    let nc = cost + price(TileResource::Occupancy(p)) + extra;
                    if costs.get(&next).is_none_or(|&c| nc < c) {
                        costs.insert(next, nc);
                        prev.insert(next, Some(s));
                        q.push(Reverse((nc, next)));
                    }
                }
            }
        }
    }
    None
}
/// Resolve physical occupancy and same-direction launcher interference together.
pub fn solve_tiles_negotiated(
    problem: TileRoutingProblem,
    rounds: usize,
) -> Result<Vec<TileRoute>> {
    let TileRoutingProblem {
        geometry,
        fixed,
        mut nets,
    } = problem;
    let mut fixed = fixed;
    fixed.extend(nets.iter().map(|n| n.a));
    let mut paths = vec![Vec::new(); nets.len()];
    let mut usage = HashMap::<TileResource, i32>::new();
    let mut history = HashMap::<TileResource, i32>::new();
    let mut best = usize::MAX;
    for round in 0..rounds {
        for offset in 0..nets.len() {
            let id = (offset + round) % nets.len();
            for p in resources(&paths[id]) {
                *usage.entry(p).or_default() -= 1;
            }
            let candidates = (offset..nets.len())
                .map(|v| (v + round) % nets.len())
                .filter(|&j| {
                    j == id || nets[id].exchange.is_some() && nets[id].exchange == nets[j].exchange
                })
                .collect::<Vec<_>>();
            let targets = candidates
                .iter()
                .map(|&j| (nets[j].b, nets[j].bd))
                .collect::<Vec<_>>();
            let Some(path) = search(
                &nets[id],
                &geometry,
                &fixed,
                &usage,
                &history,
                20 + round as i32 * 3,
                &targets,
            )
            .or_else(|| {
                search(
                    &nets[id],
                    &geometry,
                    &fixed,
                    &HashMap::new(),
                    &HashMap::new(),
                    1,
                    &targets,
                )
            }) else {
                bail!(
                    "Unreachable connection {id} from {:?} toward {:?}",
                    nets[id].a,
                    targets
                );
            };
            let endpoint = path
                .last()
                .ok_or_else(|| anyhow::anyhow!("Empty route for connection {id}"))?
                .p;
            let other = candidates
                .into_iter()
                .find(|&j| nets[j].b == endpoint)
                .ok_or_else(|| anyhow::anyhow!("Route did not reach a candidate destination"))?;
            let old = (nets[id].b, nets[id].bd);
            (nets[id].b, nets[id].bd) = (nets[other].b, nets[other].bd);
            (nets[other].b, nets[other].bd) = old;
            for p in resources(&path) {
                *usage.entry(p).or_default() += 1;
            }
            paths[id] = path;
        }
        let conflicts = usage.values().filter(|&&v| v > 1).count();
        if conflicts < best || round % 10 == 0 {
            best = best.min(conflicts);
            eprintln!(
                "Round {round}: {conflicts} conflicts, {} physical",
                usage
                    .iter()
                    .filter(|(p, v)| matches!(p, TileResource::Occupancy(_)) && **v > 1)
                    .count()
            );
        }
        if conflicts == 0 {
            return Ok(nets
                .into_iter()
                .zip(paths)
                .map(|(net, steps)| TileRoute { net, steps })
                .collect());
        }
        for (&p, &v) in &usage {
            if v > 1 {
                *history.entry(p).or_default() += 10 * (v - 1);
            }
        }
    }
    bail!("Routing stopped at {best} conflicts")
}
