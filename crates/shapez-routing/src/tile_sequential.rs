//! Ordered disjoint routing with deterministic permutation retries.
use anyhow::{Result, bail};
use shapez_model::{
    FoundationGeometry, SearchOptions, TILE_DIRECTIONS as D, TileNet as Net, TilePoint as Point,
    TileRoute, TileRouteStep as State, TileRoutingProblem,
};
use std::collections::{HashMap, HashSet, VecDeque};
/// Route all twelve connections, restarting deterministic permutations on congestion.
pub fn solve_tiles_sequential(
    problem: TileRoutingProblem,
    options: SearchOptions,
) -> Result<Vec<TileRoute>> {
    let TileRoutingProblem {
        geometry,
        fixed,
        nets,
    } = problem;
    let reservations: Vec<_> = nets
        .iter()
        .map(|n| {
            let approach = if n.input {
                (n.a.0 + D[n.ad].0, n.a.1 + D[n.ad].1, n.a.2)
            } else {
                (n.b.0 - D[n.bd].0, n.b.1 - D[n.bd].1, n.b.2)
            };
            [n.a, n.b, approach]
        })
        .collect();
    let mut seed = options.seed;
    for _ in 0..options.attempts {
        let mut order: Vec<_> = (0..nets.len()).collect();
        for i in (1..order.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            order.swap(i, seed as usize % (i + 1));
        }
        let mut occupied = fixed.clone();
        let mut result = Vec::new();
        for id in order {
            let net = &nets[id];
            let forbidden: HashSet<_> = reservations
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != id)
                .flat_map(|(_, points)| points.iter().copied())
                .collect();
            let Some(path) = search(
                net,
                &geometry,
                &occupied,
                &forbidden,
                (seed as usize + id) % 4,
            ) else {
                break;
            };

            occupied.extend(path.iter().map(|s| s.p));
            result.push(TileRoute {
                net: net.clone(),
                steps: path,
            });
        }
        if result.len() == nets.len() {
            return Ok(result);
        }
    }
    bail!("No disjoint twelve-connection route found")
}
/// Breadth-first search with straight-only launchers and catchers.
fn search(
    n: &Net,
    geometry: &FoundationGeometry,
    occupied: &HashSet<Point>,
    ends: &HashSet<Point>,
    preference: usize,
) -> Option<Vec<State>> {
    let start = State {
        p: n.a,
        d: n.ad,
        catch: false,
    };
    let mut queue = VecDeque::from([start]);
    let mut previous = HashMap::from([(start, None)]);
    while let Some(s) = queue.pop_front() {
        if s.p == n.b
            && (!s.catch || s.d == n.bd)
            && (!n.output || s.d == n.bd)
            && (n.bd + 4 - s.d) % 4 != 2
        {
            let mut path = vec![s];
            let mut current = s;
            while let Some(p) = previous[&current] {
                path.push(p);
                current = p;
            }
            // Directional BFS can revisit a tile from another direction; a real
            // belt route cannot occupy that tile twice to perform a U-turn.
            if path.iter().map(|node| node.p).collect::<HashSet<_>>().len() != path.len() {
                continue;
            }
            path.reverse();
            return Some(path);
        }
        for offset in 0..4 {
            let d = (offset + preference) % 4;
            let direction = D[d];
            if (d + 4 - s.d) % 4 == 2 || ((s.catch || s == start && n.input) && d != s.d) {
                continue;
            }
            for length in 1..=5 {
                if length > 1 && (d != s.d || s.catch || s == start && n.input) {
                    continue;
                }
                let p = (
                    s.p.0 + direction.0 * length,
                    s.p.1 + direction.1 * length,
                    s.p.2,
                );
                // Foundation_1x2 has a 34x14 floor, not a filled 16x16 square.
                // Boundary cells are only legal at the dedicated external ports.
                if (!geometry.contains(p) && p != n.b)
                    || occupied.contains(&p)
                    || (ends.contains(&p) && p != n.b)
                    || (p == n.b && length > 1)
                {
                    continue;
                }
                let next = State {
                    p,
                    d,
                    catch: length > 1,
                };
                let mut ancestor = Some(s);
                let mut repeated = false;
                while let Some(node) = ancestor {
                    if node.p == p {
                        repeated = true;
                        break;
                    }
                    ancestor = previous[&node];
                }
                if repeated {
                    continue;
                }
                if let std::collections::hash_map::Entry::Vacant(e) = previous.entry(next) {
                    e.insert(Some(s));
                    queue.push_back(next);
                }
            }
        }
    }
    None
}
