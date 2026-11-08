//! Legacy adapter routing and physical transport generation.
use crate::reports::{AdapterReport, Connection, Summary};
use anyhow::{Result, bail, ensure};
use shapez_blueprint::{Blueprint, Building, BuildingBlueprint, Island, Rotation};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

/// Horizontal tile coordinates in a generated platform.
type Point = (i32, i32);
/// Unit offsets indexed east, south, west and north.
const D: [Point; 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
#[derive(Clone, Default)]
/// One net crossing a tile, with one incoming side and possible branching outputs.
struct Flow {
    /// Side from which the net enters the tile.
    input: Option<usize>,
    /// Sides to which this net leaves the tile.
    outputs: BTreeSet<usize>,
}
/// Per-tile routes grouped by logical net.
type Grid = BTreeMap<Point, BTreeMap<usize, Flow>>;
/// Return the cardinal direction between adjacent points; panics for nonadjacent points.
fn dir(a: Point, b: Point) -> usize {
    D.iter()
        .position(|&(x, y)| (b.0 - a.0, b.1 - a.1) == (x, y))
        .unwrap()
}
/// Expand orthogonal waypoints into interior tile flows, rejecting conflicting arrivals.
fn route(grid: &mut Grid, net: usize, waypoints: &[Point]) -> Result<()> {
    let mut points = vec![waypoints[0]];
    for pair in waypoints.windows(2) {
        let (mut a, b) = (pair[0], pair[1]);
        ensure!(a.0 == b.0 || a.1 == b.1, "Non-orthogonal route");
        while a != b {
            a.0 += (b.0 - a.0).signum();
            a.1 += (b.1 - a.1).signum();
            points.push(a);
        }
    }
    // Endpoints belong to the source / destination, not the generated grid.
    for p in points.windows(3) {
        let f = grid.entry(p[1]).or_default().entry(net).or_default();
        let input = dir(p[1], p[0]);
        ensure!(
            f.input.is_none_or(|i| i == input),
            "Converging route at {:?}",
            p[1]
        );
        f.input = Some(input);
        f.outputs.insert(dir(p[1], p[2]));
    }
    Ok(())
}
/// Append a building at a tile with its rotation normalized by the blueprint model.
fn put(b: &mut Vec<Building>, p: Point, z: i16, r: usize, t: &str) {
    b.push(Building::new(t, p.0, p.1, z, Rotation::new(r)));
}
/// Append a belt or fluid sender or receiver at the requested tile.
fn port(b: &mut Vec<Building>, p: Point, z: i16, r: usize, fluid: bool, send: bool) {
    put(
        b,
        p,
        z,
        r,
        &format!(
            "{}Port{}InternalVariant",
            if fluid { "Fluid" } else { "Belt" },
            if send { "Sender" } else { "Receiver" }
        ),
    );
}
/// Append straight or turning transport for an incoming travel direction and outgoing direction.
fn turn(b: &mut Vec<Building>, p: Point, z: i16, in_dir: usize, out: usize, fluid: bool) {
    let delta = (out + 4 - in_dir) % 4;
    let (t, r) = if fluid {
        if delta == 0 {
            ("PipeForwardInternalVariant", in_dir)
        } else if delta == 3 {
            ("PipeLeftInternalVariant", in_dir)
        } else {
            ("PipeLeftInternalVariant", (in_dir + 3) % 4)
        }
    } else {
        (
            match delta {
                0 => "BeltDefaultForwardInternalVariant",
                3 => "BeltDefaultLeftInternalVariant",
                1 => "BeltDefaultLeftInternalVariantMirrored",
                _ => panic!("U-turn"),
            },
            in_dir,
        )
    };
    put(b, p, z, r, t);
}
/// Locate one of four lanes on a platform side.
fn edge(side: usize, k: i32) -> Point {
    match side {
        0 => (17, 8 + k),
        1 => (8 + k, 17),
        2 => (2, 8 + k),
        _ => (8 + k, 2),
    }
}
/// Connect opposite platform sides across three layers, optionally jumping over the center.
fn straight(b: &mut Vec<Building>, input: usize, output: usize, fluid: bool, jump: bool) {
    for z in 0..3 {
        for k in 0..4 {
            let a = edge(input, k);
            let end = edge(output, k);
            let d = (input + 2) % 4;
            let mut p = a;
            while p != end {
                if p == a {
                    port(b, p, z, d, fluid, false)
                } else if jump && ((d == 0 && p.0 == 7) || (d == 2 && p.0 == 12)) {
                    port(b, p, z, d, fluid, true);
                    p = (p.0 + D[d].0 * 5, p.1);
                    port(b, p, z, d, fluid, false);
                    p = (p.0 + D[d].0, p.1);
                    continue;
                } else {
                    turn(b, p, z, d, d, fluid)
                }
                p = (p.0 + D[d].0, p.1 + D[d].1);
            }
            port(b, end, z, d, fluid, true);
        }
    }
}
/// Connect adjacent platform sides with nested four-lane corners on three layers.
fn corner(b: &mut Vec<Building>, input: usize, output: usize, fluid: bool) {
    // Rotate a north -> west nested corner; mirror for north -> east.
    let rot = (input + 1) % 4;
    let mirror = output == rot;
    for z in 0..3 {
        for k in 0..4 {
            let mut path = vec![];
            for y in 2..=8 + k {
                path.push((8 + k, y));
            }
            for x in (2..8 + k).rev() {
                path.push((x, 8 + k));
            }
            for p in &mut path {
                if mirror {
                    p.0 = 19 - p.0;
                }
                for _ in 0..rot {
                    *p = (19 - p.1, p.0);
                }
            }
            for i in 0..path.len() {
                if i == 0 {
                    port(b, path[i], z, (input + 2) % 4, fluid, false)
                } else if i + 1 == path.len() {
                    port(b, path[i], z, output, fluid, true)
                } else {
                    turn(
                        b,
                        path[i],
                        z,
                        dir(path[i - 1], path[i]),
                        dir(path[i], path[i + 1]),
                        fluid,
                    )
                }
            }
        }
    }
}
/// Transform a splitter template to match the requested input and output sides.
fn split(
    b: &mut Vec<Building>,
    template: &[Building],
    input: usize,
    outputs: &BTreeSet<usize>,
    fluid: bool,
) -> Result<()> {
    let rot = (input + 1) % 4;
    let canonical_side = rot;
    let mirror = !outputs.contains(&canonical_side);
    for e in template {
        let (mut x, mut y) = (e.x, e.y);
        let mut r = e.rotation.index();
        let mut t = e.definition.clone();
        if mirror {
            x = 19 - x;
            r = (2 + 4 - r) % 4;
            if t.contains("Splitter") {
                if t.ends_with("Mirrored") {
                    t.truncate(t.len() - 8)
                } else {
                    t.push_str("Mirrored")
                }
            }
        }
        for _ in 0..rot {
            (x, y) = (19 - y, x);
        }
        r = (r + rot) % 4;
        if fluid {
            if t.contains("Splitter") {
                // Splitter inputs W, outputs E,N (mirrored: E,S). Pipe junction has E,S,N.
                let missing = if t.ends_with("Mirrored") {
                    (r + 3) % 4
                } else {
                    (r + 1) % 4
                };
                r = (missing + 2) % 4;
                t = "PipeJunctionInternalVariant".to_owned();
            } else {
                t = t.replace("BeltPort", "FluidPort");
            }
        }
        let mut building = e.clone();
        building.x = x;
        building.y = y;
        building.rotation = Rotation::new(r);
        building.definition = t;
        b.push(building);
    }
    Ok(())
}

/// Generate a full, reproducible adapter using the user's own splitter and RGB templates.
pub fn connect_cmam(
    input: &Path,
    splitter: &Path,
    rgb: &Path,
    out: &Path,
) -> Result<AdapterReport> {
    let mut original = Blueprint::read(input)?;
    let split_bp = Blueprint::read(splitter)?;
    let rgb_bp = Blueprint::read(rgb)?;
    ensure!(
        original.islands().len() == 108,
        "Expected the supplied 108-island CMAM layout"
    );
    let template = split_bp
        .islands()
        .first()
        .and_then(|i| i.buildings.as_ref())
        .ok_or_else(|| anyhow::anyhow!("Splitter template missing buildings"))?;
    let mut islands = vec![];
    for i in original.islands() {
        let mut i = i.clone();
        let y = i.y;
        if y >= 1 {
            let marker = i.buildings.as_ref().is_some_and(|b| {
                b.entries
                    .iter()
                    .all(|v| v.definition.starts_with("Trash") || v.definition.starts_with("Pipe"))
            });
            if marker {
                continue;
            }
            i.y = y + 4;
        }
        islands.push(i);
    }
    let mut trans = rgb_bp
        .islands()
        .first()
        .ok_or_else(|| anyhow::anyhow!("RGB template missing island"))?
        .clone();
    trans.x = 20;
    trans.y = -3;
    islands.push(trans);
    let mut grid = Grid::new();
    // North bus: pins, stars, windmills, RGB painter A, RGB painter B.
    for (net, source, row, col) in [
        (0, 14, -2, -18),
        (1, 17, -1, -19),
        (2, 18, 0, -17),
        (3, 19, 1, -16),
        (4, 20, 2, -15),
    ] {
        let source_y = if net >= 3 { -3 } else { -4 };
        for j in 0..5 {
            let x = col + 5 * j;
            route(
                &mut grid,
                net,
                &[(source, source_y), (source, row), (x, row), (x, 6)],
            )?;
        }
    }
    // South bus: circle, rectangle, RGB for the south painting inlet.
    for (net, source, trunk, feed_row, bus_row, col) in [
        (5, 15, 10, 3, 12, -19),
        (6, 16, 11, 4, 11, -17),
        (7, 21, 12, 5, 10, -18),
    ] {
        let source_y = if net == 7 { -3 } else { -4 };
        for j in 0..5 {
            let x = col + 5 * j;
            route(
                &mut grid,
                net,
                &[
                    (source, source_y),
                    (source, feed_row),
                    (trunk, feed_row),
                    (trunk, bus_row),
                    (x, bus_row),
                    (x, 8),
                ],
            )?;
        }
    }
    let mut connections = vec![];
    for (&pos, nets) in &grid {
        ensure!(nets.len() <= 2, "More than two streams at {pos:?}");
        let mut b = vec![];
        for (&net, f) in nets {
            let input = f.input.unwrap();
            let fluid = [3, 4, 7].contains(&net);
            if f.outputs.len() == 2 {
                ensure!(nets.len() == 1, "Branch and crossover overlap at {pos:?}");
                split(&mut b, &template.entries, input, &f.outputs, fluid)?;
            } else {
                let output = *f.outputs.first().unwrap();
                if (input + 2) % 4 == output {
                    straight(
                        &mut b,
                        input,
                        output,
                        fluid,
                        nets.len() == 2 && input % 2 == 0,
                    )
                } else {
                    ensure!(nets.len() == 1, "Corner overlaps crossing at {pos:?}");
                    corner(&mut b, input, output, fluid)
                }
            }
            connections.push(Connection {
                chunk: [pos.0, pos.1],
                net,
                input,
                outputs: f.outputs.clone(),
            });
        }
        let mut occupied = BTreeSet::new();
        for e in &b {
            ensure!(
                occupied.insert((e.x, e.y, e.layer)),
                "Building collision in {pos:?}: {e:?}"
            );
        }
        if islands.iter().any(|i| i.x == pos.0 && i.y == pos.1) {
            bail!("Generated platform overlaps existing island at {pos:?}")
        }
        let mut island = Island::new("Foundation_1x1", pos.0, pos.1, 0, Rotation::default());
        island.buildings = Some(BuildingBlueprint::new(b));
        islands.push(island);
    }
    original.replace_islands(islands)?;
    fs::create_dir_all(out)?;
    let path = out.join("CMAM connected.spz2bp");
    original.write(&path)?;
    let report = AdapterReport {
        blueprint: path,
        generated_platforms: grid.len(),
        summary: Summary::from(&original),
        connections,
    };
    fs::write(
        out.join("adapter.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
