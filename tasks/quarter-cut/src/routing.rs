//! Three-floor negotiated routing for the compact factory.
use anyhow::{Result, bail};
use shapez_blueprint::{Building, Rotation};
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
};
/// Building tile; synthetic layers 3..14 reserve directional jump ranges.
pub type Point = (i32, i32, i16);
/// Clockwise travel directions in native blueprint coordinates.
const D: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
/// Arrival direction and straight-only receiver state.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct State {
    p: Point,
    d: usize,
    catch: bool,
}
/// Directed endpoints, optionally attached to an external port.
pub struct Net {
    pub a: Point,
    pub ad: usize,
    pub b: Point,
    pub bd: usize,
    pub input: bool,
    pub output: bool,
    pub flex: bool,
    /// Only equal-rate stages can exchange destinations: 0..3 inputs, 4 outputs.
    pub class: u8,
}
/// All occupied lift tiles plus launcher ranges that must not interfere.
fn resources(path: &[State]) -> Vec<Point> {
    let mut r = Vec::new();
    for (i, s) in path.iter().enumerate() {
        let next = path.get(i + 1);
        let z = next.map_or(s.p.2, |n| n.p.2);
        for k in s.p.2.min(z)..=s.p.2.max(z) {
            r.push((s.p.0, s.p.1, k));
        }
        if next.is_some_and(|n| (n.p.0 - s.p.0).abs() + (n.p.1 - s.p.1).abs() > 1) {
            let d = D[s.d];
            for k in 1..=5 {
                r.push((
                    s.p.0 + d.0 * k,
                    s.p.1 + d.1 * k,
                    s.p.2 + 3 * (s.d as i16 + 1),
                ));
            }
        }
    }
    r
}
/// Find a directional path, pricing conflicts for the next negotiated routing round.
fn search(
    n: &Net,
    fixed: &HashSet<Point>,
    usage: &HashMap<Point, i32>,
    history: &HashMap<Point, i32>,
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
            && ((3..=56).contains(&p.0) && (3..=16).contains(&p.1)
                || p == n.a
                || targets.iter().any(|v| v.0 == p))
    };
    let price = |p: Point| {
        10 + usage.get(&p).copied().unwrap_or(0) * penalty + history.get(&p).copied().unwrap_or(0)
    };
    while let Some(Reverse((cost, s))) = q.pop() {
        if costs[&s] != cost {
            continue;
        }
        if targets.iter().any(|&(p, bd)| {
            s.p == p
                && (!s.catch || s.d == bd)
                && (!n.output || s.d == bd)
                && (bd + 4 - s.d) % 4 != 2
        }) {
            let mut path = vec![s];
            let mut cur = s;
            while let Some(p) = prev[&cur] {
                path.push(p);
                cur = p;
            }
            path.reverse();
            let r = resources(&path)
                .into_iter()
                .filter(|p| p.2 < 3)
                .collect::<Vec<_>>();
            if r.iter().copied().collect::<HashSet<_>>().len() == r.len() {
                return Some(path);
            }
            continue;
        }
        for z in 0..3 {
            let lift = z != s.p.2;
            if lift && (s.catch || s.p == n.a && n.input) {
                continue;
            }
            if lift && (s.p.2.min(z)..=s.p.2.max(z)).any(|k| !legal((s.p.0, s.p.1, k))) {
                continue;
            }
            for (d, v) in D.iter().enumerate() {
                if !lift && (d + 4 - s.d) % 4 == 2 {
                    continue;
                }
                if (s.catch || s.p == n.a && n.input) && d != s.d {
                    continue;
                }
                for len in 1..=5 {
                    if len > 1 && (lift || d != s.d || s.catch || s.p == n.a && n.input) {
                        continue;
                    }
                    let p = (s.p.0 + v.0 * len, s.p.1 + v.1 * len, z);
                    if ((8..=11).contains(&s.p.0) || (48..=51).contains(&s.p.0))
                        && (s.p.1 == 3 || s.p.1 == 16)
                    {
                        let port = (s.p.0, if s.p.1 == 3 { 2 } else { 17 }, s.p.2);
                        if p != port {
                            continue;
                        }
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
                            .map(|k| price((s.p.0, s.p.1, k)))
                            .sum::<i32>()
                    } else if len > 1 {
                        (1..=5)
                            .map(|k| {
                                price((s.p.0 + v.0 * k, s.p.1 + v.1 * k, z + 3 * (d as i16 + 1)))
                                    - 9
                            })
                            .sum()
                    } else {
                        0
                    };
                    let nc = cost + price(p) + extra;
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
pub fn connect(
    mut b: Vec<Building>,
    fixed: HashSet<Point>,
    mut nets: Vec<Net>,
) -> Result<Vec<Building>> {
    let mut fixed = fixed;
    fixed.extend(nets.iter().map(|n| n.a));
    let mut paths = vec![Vec::new(); nets.len()];
    let mut usage = HashMap::<Point, i32>::new();
    let mut history = HashMap::<Point, i32>::new();
    let mut best = usize::MAX;
    for round in 0..240 {
        for offset in 0..nets.len() {
            let id = (offset + round) % nets.len();
            for p in resources(&paths[id]) {
                *usage.entry(p).or_default() -= 1;
            }
            let candidates = (offset..nets.len())
                .map(|v| (v + round) % nets.len())
                .filter(|&j| {
                    j == id || nets[id].flex && nets[j].flex && nets[id].class == nets[j].class
                })
                .collect::<Vec<_>>();
            let targets = candidates
                .iter()
                .map(|&j| (nets[j].b, nets[j].bd))
                .collect::<Vec<_>>();
            let Some(path) = search(
                &nets[id],
                &fixed,
                &usage,
                &history,
                20 + round as i32 * 3,
                &targets,
            )
            .or_else(|| {
                search(
                    &nets[id],
                    &fixed,
                    &HashMap::new(),
                    &HashMap::new(),
                    1,
                    &targets,
                )
            }) else {
                bail!("Unreachable connection {id}");
            };
            let other = candidates
                .into_iter()
                .find(|&j| nets[j].b == path.last().unwrap().p)
                .unwrap();
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
                usage.iter().filter(|(p, v)| p.2 < 3 && **v > 1).count()
            );
        }
        if conflicts == 0 {
            for (id, path) in paths.iter().enumerate() {
                let n = &nets[id];
                for (i, s) in path.iter().enumerate() {
                    let next = path.get(i + 1);
                    let d = next.map_or(n.bd, |v| v.d);
                    let z = next.map_or(s.p.2, |v| v.p.2);
                    let delta = (d + 4 - s.d) % 4;
                    let jump =
                        next.is_some_and(|v| (v.p.0 - s.p.0).abs() + (v.p.1 - s.p.1).abs() > 1);
                    let t = if z != s.p.2 {
                        format!(
                            "Lift{}{}{}InternalVariant{}",
                            (z - s.p.2).abs(),
                            if z > s.p.2 { "Up" } else { "Down" },
                            match delta {
                                0 => "Forward",
                                2 => "Backward",
                                _ => "Left",
                            },
                            if delta == 1 { "Mirrored" } else { "" }
                        )
                    } else if s.catch || i == 0 && n.input {
                        "BeltPortReceiverInternalVariant".into()
                    } else if jump || i + 1 == path.len() && n.output {
                        "BeltPortSenderInternalVariant".into()
                    } else {
                        match delta {
                            0 => "BeltDefaultForwardInternalVariant",
                            1 => "BeltDefaultLeftInternalVariantMirrored",
                            3 => "BeltDefaultLeftInternalVariant",
                            _ => unreachable!(),
                        }
                        .into()
                    };
                    b.push(Building::new(t, s.p.0, s.p.1, s.p.2, Rotation::new(s.d)));
                }
            }
            return Ok(b);
        }
        for (&p, &v) in &usage {
            if v > 1 {
                *history.entry(p).or_default() += 10 * (v - 1);
            }
        }
    }
    bail!("Routing stopped at {best} conflicts")
}
