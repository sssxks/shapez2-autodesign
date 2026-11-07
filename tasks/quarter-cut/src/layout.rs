//! Outward-facing cutting rows keep distribution in the central aisle.
use crate::design;
use anyhow::Result;
use shapez_blueprint::{Building, Placement, Rotation};
use shapez_model::{
    ExchangeGroup, FoundationGeometry, TileNet as Net, TilePoint as Point, TileRoutingProblem,
};
use shapez_routing::solve_tiles_negotiated;
use std::collections::HashSet;
/// Assemble rigid cells, native port fanout and rate-preserving transport nets.
pub fn build() -> Result<Vec<Building>> {
    let mut b = Vec::new();
    let mut fixed = HashSet::<Point>::new();
    let mut nets = Vec::new();
    for z in 0..3 {
        let point = |i: usize, p: (i32, i32)| {
            let col = i % 8;
            let left = [4, 12, 18, 23, 32, 37, 42, 52][col]
                + if col == 5 || col == 6 {
                    i32::from(z)
                } else {
                    0
                };
            if i < 8 {
                (left + p.1, 7 - p.0, z)
            } else {
                (left + 3 - p.1, 12 + p.0, z)
            }
        };
        let cells: Vec<_> = (0..16)
            .map(|i| {
                design::cell(true).placed(Placement {
                    origin: point(i, (0, 0)),
                    rotation: Rotation::new(if i < 8 { 3 } else { 1 }),
                })
            })
            .collect();
        for (i, cell) in cells.iter().enumerate() {
            fixed.extend(cell.footprint.iter().copied());
            b.extend(cell.buildings.iter().cloned());
            if i % 4 != 3 {
                let left = i % 8 < 4;
                let p = cells[i].port("input")?.position;
                let mirrored = if i < 8 { left } else { !left };
                fixed.insert(p);
                b.push(Building::new(
                    if mirrored {
                        "Splitter1To2LInternalVariantMirrored"
                    } else {
                        "Splitter1To2LInternalVariant"
                    },
                    p.0,
                    p.1,
                    z,
                    Rotation::new(if left { 2 } else { 0 }),
                ));
            }
        }
        for k in 0..4 {
            let x = 28 + k;
            fixed.insert((x, 2, z));
            b.push(Building::new(
                "BeltPortReceiverInternalVariant",
                x,
                2,
                z,
                Rotation::new(1),
            ));
            let end = if k == 0 || k == 3 { 8 } else { 9 };
            for y in 3..=end {
                let t = if y < end {
                    "BeltDefaultForwardInternalVariant"
                } else if k < 2 {
                    "BeltDefaultLeftInternalVariantMirrored"
                } else {
                    "BeltDefaultLeftInternalVariant"
                };
                fixed.insert((x, y, z));
                b.push(Building::new(t, x, y, z, Rotation::new(1)));
            }
        }
        for (x, y, r) in [(28, 9, 2), (31, 9, 0)] {
            fixed.insert((x, y, z));
            b.push(Building::new(
                "BeltDefaultForwardInternalVariant",
                x,
                y,
                z,
                Rotation::new(r),
            ));
        }
        let inputs = cells
            .iter()
            .map(|c| c.port("input"))
            .collect::<Result<Vec<_>>>()?;
        let outputs = cells
            .iter()
            .map(|c| c.port("output"))
            .collect::<Result<Vec<_>>>()?;
        let mut used = [false; 16];
        for g in 0..4 {
            let (x, y, dir) = [(27, 8, 2), (27, 9, 2), (32, 9, 0), (32, 8, 0)][g];
            let mut a = (x, y, z);
            let mut ad = dir;
            for j in 0..4 {
                let id = (0..16)
                    .filter(|&i| !used[i] && (i % 4 != 3) == (j < 3))
                    .min_by_key(|&i| {
                        let p = inputs[i].position;
                        (a.0 - p.0).abs() + (a.1 - p.1).abs()
                    })
                    .unwrap();
                used[id] = true;
                let p = inputs[id].position;
                let left = id % 8 < 4;
                let (target, bd) = if j < 3 {
                    (
                        (p.0 + if left { 1 } else { -1 }, p.1, z),
                        if left { 2 } else { 0 },
                    )
                } else {
                    (p, inputs[id].direction)
                };
                nets.push(Net {
                    a,
                    ad,
                    b: target,
                    bd,
                    input: false,
                    output: false,
                    exchange: Some(ExchangeGroup(j as u8)),
                });
                a = (p.0 + if left { -1 } else { 1 }, p.1, z);
                ad = if left { 2 } else { 0 };
            }
        }
        let mut ports = Vec::new();
        for k in 0..4 {
            ports.extend([
                ((8 + k, 2, z), 3),
                ((48 + k, 2, z), 3),
                ((8 + k, 17, z), 1),
                ((48 + k, 17, z), 1),
            ]);
        }
        let dist = |i: usize, j: usize| {
            let p = outputs[i].position;
            let q = ports[j].0;
            (p.0 - q.0).abs() + (p.1 - q.1).abs()
        };
        let mut ids = (0..16).collect::<Vec<_>>();
        loop {
            let mut changed = false;
            for i in 0..16 {
                for j in i + 1..16 {
                    if dist(i, ids[j]) + dist(j, ids[i]) < dist(i, ids[i]) + dist(j, ids[j]) {
                        ids.swap(i, j);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for i in 0..16 {
            let (p, d) = ports[ids[i]];
            nets.push(Net {
                a: outputs[i].position,
                ad: outputs[i].direction,
                b: p,
                bd: d,
                input: false,
                output: true,
                exchange: Some(ExchangeGroup(4)),
            });
        }
    }
    for (i, n) in nets.iter().enumerate() {
        if fixed.contains(&n.a) || fixed.contains(&n.b) {
            anyhow::bail!(
                "Connection {i} overlaps a production building: {:?} {:?}",
                n.a,
                n.b
            );
        }
    }
    let mut geometry = FoundationGeometry::strip(3, 3);
    for n in nets.iter().filter(|n| n.output) {
        let d = shapez_model::TILE_DIRECTIONS[n.bd];
        geometry
            .exits
            .insert((n.b.0 - d.0, n.b.1 - d.1, n.b.2), n.b);
    }
    for route in solve_tiles_negotiated(
        TileRoutingProblem {
            geometry,
            fixed,
            nets,
        },
        240,
    )? {
        shapez_blueprint::route_buildings(&route, &mut b);
    }
    Ok(b)
}
