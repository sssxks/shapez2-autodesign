//! Small-grid directed belt routing, including native two-to-five-tile jumps.
use anyhow::{Result, bail};
use shapez_blueprint::{Building, Rotation};
use std::collections::{HashMap, HashSet, VecDeque};
type Point = (i32, i32);
const D: [Point; 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
/// Arrival direction and whether this tile is a jump catcher.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct State {
    p: Point,
    d: usize,
    catch: bool,
}
/// A route includes its external port and its final tile adjacent to the cell.
#[derive(Clone, Copy)]
struct Net {
    a: Point,
    ad: usize,
    b: Point,
    bd: usize,
    input: bool,
}
/// Route all twelve connections, restarting deterministic permutations on congestion.
pub fn connect(buildings: &mut Vec<Building>, centers: &[Point; 4]) -> Result<()> {
    let mut nets = Vec::new();
    for (i, &(x, y)) in centers.iter().enumerate() {
        let k = i as i32;
        nets.push(Net {
            a: (2, 8 + k),
            ad: 0,
            b: (x - 2, y - 2),
            bd: 1,
            input: true,
        });
        nets.push(Net {
            a: (8 + k, 2),
            ad: 1,
            b: (x + 1, y + 1),
            bd: 2,
            input: true,
        });
        nets.push(Net {
            a: (x - 1, y + 2),
            ad: 1,
            b: (17, 8 + k),
            bd: 0,
            input: false,
        });
    }
    let fixed: HashSet<_> = buildings.iter().map(|b| (b.x, b.y)).collect();
    let ends: HashSet<_> = nets.iter().flat_map(|n| [n.a, n.b]).collect();
    let mut seed = 1234567_u64;
    for _ in 0..2000 {
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
            let net = nets[id];
            let Some(path) = search(net, &occupied, &ends) else {
                break;
            };
            for (i, s) in path.iter().enumerate() {
                let next = path.get(i + 1);
                let outgoing = next.map_or(net.bd, |n| n.d);
                let jump = next.is_some_and(|n| (n.p.0 - s.p.0).abs() + (n.p.1 - s.p.1).abs() > 1);
                let t = if s.catch || (i == 0 && net.input) {
                    "BeltPortReceiverInternalVariant"
                } else if jump || (i + 1 == path.len() && !net.input) {
                    "BeltPortSenderInternalVariant"
                } else {
                    match (outgoing + 4 - s.d) % 4 {
                        0 => "BeltDefaultForwardInternalVariant",
                        1 => "BeltDefaultLeftInternalVariantMirrored",
                        3 => "BeltDefaultLeftInternalVariant",
                        _ => unreachable!(),
                    }
                };
                result.push(Building::new(t, s.p.0, s.p.1, 0, Rotation::new(s.d)));
                occupied.insert(s.p);
            }
        }
        if nets
            .iter()
            .all(|n| occupied.contains(&n.a) && occupied.contains(&n.b))
        {
            buildings.extend(result);
            return Ok(());
        }
    }
    bail!("No disjoint twelve-connection route found")
}
/// Breadth-first search with straight-only launchers and catchers.
fn search(n: Net, occupied: &HashSet<Point>, ends: &HashSet<Point>) -> Option<Vec<State>> {
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
            && (n.input || s.d == n.bd)
            && (n.bd + 4 - s.d) % 4 != 2
        {
            let mut path = vec![s];
            let mut current = s;
            while let Some(p) = previous[&current] {
                path.push(p);
                current = p;
            }
            path.reverse();
            return Some(path);
        }
        for (d, direction) in D.iter().enumerate() {
            if (d + 4 - s.d) % 4 == 2 || ((s.catch || s == start && n.input) && d != s.d) {
                continue;
            }
            for length in 1..=5 {
                if length > 1 && (d != s.d || s.catch || s == start && n.input) {
                    continue;
                }
                let p = (s.p.0 + direction.0 * length, s.p.1 + direction.1 * length);
                if !(2..=17).contains(&p.0)
                    || !(2..=17).contains(&p.1)
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
                if let std::collections::hash_map::Entry::Vacant(e) = previous.entry(next) {
                    e.insert(Some(s));
                    queue.push_back(next);
                }
            }
        }
    }
    None
}
