use crate::{Blueprint, encode_blueprint};
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

type Point = (i32, i32);
const D: [Point; 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
#[derive(Clone, Default)]
struct Flow {
    input: Option<usize>,
    outputs: BTreeSet<usize>,
}
type Grid = BTreeMap<Point, BTreeMap<usize, Flow>>;
fn n(v: &Value, k: &str) -> i32 {
    v[k].as_i64().unwrap_or(0) as i32
}
fn dir(a: Point, b: Point) -> usize {
    D.iter()
        .position(|&(x, y)| (b.0 - a.0, b.1 - a.1) == (x, y))
        .unwrap()
}
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
fn put(b: &mut Vec<Value>, p: Point, z: i32, r: usize, t: &str) {
    b.push(json!({"X":p.0,"Y":p.1,"L":z,"R":r,"T":t}));
}
fn port(b: &mut Vec<Value>, p: Point, z: i32, r: usize, fluid: bool, send: bool) {
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
fn turn(b: &mut Vec<Value>, p: Point, z: i32, in_dir: usize, out: usize, fluid: bool) {
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
fn edge(side: usize, k: i32) -> Point {
    match side {
        0 => (17, 8 + k),
        1 => (8 + k, 17),
        2 => (2, 8 + k),
        _ => (8 + k, 2),
    }
}
fn straight(b: &mut Vec<Value>, input: usize, output: usize, fluid: bool, jump: bool) {
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
fn corner(b: &mut Vec<Value>, input: usize, output: usize, fluid: bool) {
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
fn split(
    b: &mut Vec<Value>,
    template: &[Value],
    input: usize,
    outputs: &BTreeSet<usize>,
    fluid: bool,
) -> Result<()> {
    let rot = (input + 1) % 4;
    let canonical_side = rot;
    let mirror = !outputs.contains(&canonical_side);
    for e in template {
        let (mut x, mut y) = (n(e, "X"), n(e, "Y"));
        let mut r = n(e, "R") as usize;
        let mut t = e["T"].as_str().unwrap().to_owned();
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
        put(b, (x, y), n(e, "L"), r, &t);
    }
    Ok(())
}

/// Generate a full, reproducible adapter using the user's own splitter and RGB templates.
pub fn connect_cmam(input: &Path, splitter: &Path, rgb: &Path, out: &Path) -> Result<Value> {
    let mut original = Blueprint::read(input)?;
    let split_bp = Blueprint::read(splitter)?;
    let rgb_bp = Blueprint::read(rgb)?;
    ensure!(
        original.islands().len() == 108,
        "Expected the supplied 108-island CMAM layout"
    );
    let template = split_bp.islands()[0]["B"]["Entries"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Splitter template missing buildings"))?;
    let mut islands = vec![];
    for i in original.islands() {
        let mut i = i.clone();
        let y = n(&i, "Y");
        if y >= 1 {
            let marker = i["B"]["Entries"].as_array().is_some_and(|b| {
                b.iter().all(|v| {
                    v["T"]
                        .as_str()
                        .is_some_and(|t| t.starts_with("Trash") || t.starts_with("Pipe"))
                })
            });
            if marker {
                continue;
            }
            i["Y"] = json!(y + 4);
        }
        islands.push(i);
    }
    let mut trans = rgb_bp.islands()[0].clone();
    trans["X"] = json!(20);
    trans["Y"] = json!(-3);
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
                split(&mut b, template, input, &f.outputs, fluid)?;
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
            connections
                .push(json!({"chunk":[pos.0,pos.1],"net":net,"input":input,"outputs":f.outputs}));
        }
        let mut occupied = BTreeSet::new();
        for e in &b {
            ensure!(
                occupied.insert((n(e, "X"), n(e, "Y"), n(e, "L"))),
                "Building collision in {pos:?}: {e}"
            );
        }
        if islands
            .iter()
            .any(|i| n(i, "X") == pos.0 && n(i, "Y") == pos.1)
        {
            bail!("Generated platform overlaps existing island at {pos:?}")
        }
        islands.push(
            json!({"T":"Foundation_1x1","X":pos.0,"Y":pos.1,"B":{"$type":"Building","Entries":b}}),
        );
    }
    original.value["BP"]["Entries"] = json!(islands);
    fs::create_dir_all(out)?;
    let path = out.join("CMAM connected.spz2bp");
    fs::write(&path, encode_blueprint(&original.value)?)?;
    let report = json!({"blueprint":path,"generated_platforms":grid.len(),"summary":original.summary(),"connections":connections});
    fs::write(
        out.join("adapter.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
