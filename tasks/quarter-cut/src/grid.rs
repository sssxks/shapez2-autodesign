//! Finite-domain production and routing on one complete platform layer.
use anyhow::{Result, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use shapez_blueprint::{Building, Rotation};
use shapez_solver::{Expr, Limits, Model, Solution, Var};
use std::{collections::BTreeMap, fs, path::PathBuf};
type Point = (i32, i32, i16);
const D: [Point; 6] = [
    (1, 0, 0),
    (0, 1, 0),
    (-1, 0, 0),
    (0, -1, 0),
    (0, 0, 1),
    (0, 0, -1),
];
fn opposite(d: usize) -> usize {
    if d < 4 { (d + 2) % 4 } else { 9 - d }
}
const SHAPES: [i64; 9] = [1, 2, 4, 8, 3, 6, 9, 12, 15];
#[derive(Parser)]
struct Args {
    output: PathBuf,
    #[arg(long, default_value_t = 120.)]
    seconds: f64,
    #[arg(long, default_value_t = 8)]
    workers: i32,
    #[arg(long)]
    cell: bool,
    #[arg(long)]
    quiet: bool,
    #[arg(long)]
    native: bool,
    #[arg(long)]
    hint: Option<PathBuf>,
    /// Embed a verified 5x4 unit at the west output bank as a feasible seed.
    #[arg(long)]
    unit_hint: Option<PathBuf>,
    /// Fix only cutting/half-rotation cores; synthesize the entire shared quarter network.
    #[arg(long)]
    cores: bool,
    #[arg(long, default_value_t = 1)]
    layers: i16,
    #[arg(long)]
    infer_hint: bool,
    #[arg(long, num_args = 4, requires = "hint")]
    window: Vec<i32>,
    #[arg(long, default_value_t = 0)]
    minimum: i64,
    #[arg(long)]
    buildings_hint: Option<PathBuf>,
    #[arg(long, requires = "buildings_hint")]
    lock_layout: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct Kind {
    definition: Option<String>,
    rotation: usize,
    auxiliary: bool,
    jump: Option<usize>,
    #[serde(default)]
    lift: bool,
}
struct Grammar {
    kinds: Vec<Kind>,
    rows: Vec<Vec<i64>>,
}
fn step(p: Point, d: usize, n: i32) -> Point {
    (p.0 + D[d].0 * n, p.1 + D[d].1 * n, p.2 + D[d].2 * n as i16)
}
fn token(shape: i64, q: i64) -> i64 {
    4 * shape + q
}
impl Grammar {
    fn kind(
        &mut self,
        definition: Option<&str>,
        rotation: usize,
        auxiliary: bool,
        jump: Option<usize>,
    ) -> usize {
        let id = self.kinds.len();
        self.kinds.push(Kind {
            definition: definition.map(str::to_owned),
            rotation,
            auxiliary,
            jump,
            lift: definition.is_some_and(|d| d.starts_with("Lift")),
        });
        id
    }
    fn row(&mut self, kind: usize, inputs: &[(usize, i64)], outputs: &[(usize, i64)]) {
        let mut row = vec![0; 13];
        row[0] = kind as i64;
        for &(d, t) in inputs {
            row[1 + d] = t;
        }
        for &(d, t) in outputs {
            row[7 + d] = t;
        }
        self.rows.push(row);
    }
    fn build() -> Self {
        let mut g = Self {
            kinds: Vec::new(),
            rows: Vec::new(),
        };
        g.kind(None, 0, false, None);
        g.row(0, &[], &[]);
        for r in 0..4 {
            let dir = |d| (d + r) % 4;
            for (name, out) in [
                ("BeltDefaultForwardInternalVariant", 0),
                ("BeltDefaultLeftInternalVariant", 3),
                ("BeltDefaultLeftInternalVariantMirrored", 1),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for shape in SHAPES {
                    for q in 1..=4 {
                        g.row(
                            k,
                            &[(dir(2), token(shape, q))],
                            &[(dir(out), token(shape, q))],
                        );
                    }
                }
            }
            for (name, a, b) in [
                ("RotatorOneQuadInternalVariant", 12, 9),
                ("RotatorOneQuadCCWInternalVariant", 3, 9),
                ("RotatorOneQuadInternalVariant", 3, 6),
                ("RotatorOneQuadCCWInternalVariant", 12, 6),
                ("RotatorOneQuadInternalVariant", 8, 1),
                ("RotatorOneQuadCCWInternalVariant", 2, 1),
                ("RotatorHalfInternalVariant", 4, 1),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for q in 1..=2 {
                    g.row(k, &[(dir(2), token(a, q))], &[(dir(0), token(b, q))]);
                }
            }
            for (name, sides) in [
                ("Merger2To1LInternalVariant", &[2, 3][..]),
                ("Merger2To1LInternalVariantMirrored", &[2, 1][..]),
                ("Merger3To1InternalVariant", &[2, 3, 1][..]),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for shape in SHAPES {
                    for a in 1..=3 {
                        for b in 1..=3 {
                            for c in if sides.len() == 3 { 1..=2 } else { 0..=0 } {
                                let q = a + b + c;
                                if q > 4 {
                                    continue;
                                }
                                let rates = [a, b, c];
                                let input = sides
                                    .iter()
                                    .enumerate()
                                    .map(|(i, &d)| (dir(d), token(shape, rates[i])))
                                    .collect::<Vec<_>>();
                                g.row(k, &input, &[(dir(0), token(shape, q))]);
                            }
                        }
                    }
                }
            }
            for (name, side) in [
                ("Splitter1To2LInternalVariant", 3),
                ("Splitter1To2LInternalVariantMirrored", 1),
                ("SplitterTShapeInternalVariant", 4),
            ] {
                let k = g.kind(Some(name), r, false, None);
                for shape in SHAPES {
                    for a in 1..=3 {
                        for b in 1..=4 - a {
                            let ds = if side == 4 { [3, 1] } else { [0, side] };
                            g.row(
                                k,
                                &[(dir(2), token(shape, a + b))],
                                &[(dir(ds[0]), token(shape, a)), (dir(ds[1]), token(shape, b))],
                            );
                        }
                    }
                }
            }
            for (mirror, (name, side)) in [
                ("CutterDefaultInternalVariant", 3),
                ("CutterDefaultInternalVariantMirrored", 1),
            ]
            .into_iter()
            .enumerate()
            {
                for (recipe, (a, l, b)) in
                    [(15, 12, 3), (9, 8, 1), (6, 4, 2)].into_iter().enumerate()
                {
                    let transfer = 80 + ((r * 2 + mirror) * 3 + recipe) as i64;
                    let main = g.kind(Some(name), r, false, None);
                    g.row(
                        main,
                        &[(dir(2), token(a, 1))],
                        &[(dir(0), token(b, 1)), (dir(side), transfer)],
                    );
                    let aux = g.kind(None, r, true, None);
                    g.row(
                        aux,
                        &[((dir(side) + 2) % 4, transfer)],
                        &[(dir(0), token(l, 1))],
                    );
                }
            }
            for n in 2..=5 {
                let j = r * 4 + n - 2;
                let sender = g.kind(Some("BeltPortSenderInternalVariant"), r, false, Some(j));
                let receiver = g.kind(
                    Some("BeltPortReceiverInternalVariant"),
                    r,
                    false,
                    Some(16 + j),
                );
                for shape in SHAPES {
                    for q in 1..=4 {
                        g.row(sender, &[(dir(2), token(shape, q))], &[]);
                        g.row(receiver, &[], &[(dir(0), token(shape, q))]);
                    }
                }
            }
            for (turn, (suffix, out)) in [
                ("Forward", 0),
                ("Left", 3),
                ("LeftMirrored", 1),
                ("Backward", 2),
            ]
            .into_iter()
            .enumerate()
            {
                for (up, axis) in [(true, 4), (false, 5)] {
                    let code = r * 8 + turn * 2 + usize::from(up);
                    for length in 1..=2 {
                        let name = format!(
                            "Lift{length}{}{}InternalVariant{}",
                            if up { "Up" } else { "Down" },
                            suffix.trim_end_matches("Mirrored"),
                            if suffix.ends_with("Mirrored") {
                                "Mirrored"
                            } else {
                                ""
                            }
                        );
                        let main = g.kind(Some(&name), r, false, None);
                        let end = g.kind(None, r, true, None);
                        g.kinds[end].lift = true;
                        let middle = (length == 2).then(|| {
                            let k = g.kind(None, r, true, None);
                            g.kinds[k].lift = true;
                            k
                        });
                        for shape in SHAPES {
                            for q in 1..=4 {
                                let a = 1000
                                    + ((if length == 1 { 10 } else { 42 }) + code as i64) * 128
                                    + token(shape, q);
                                let b = 1000 + (74 + code as i64) * 128 + token(shape, q);
                                g.row(main, &[(dir(2), token(shape, q))], &[(axis, a)]);
                                g.row(
                                    end,
                                    &[(opposite(axis), if length == 1 { a } else { b })],
                                    &[(dir(out), token(shape, q))],
                                );
                                if let Some(m) = middle {
                                    g.row(m, &[(opposite(axis), a)], &[(axis, b)]);
                                }
                            }
                        }
                    }
                }
            }
        }
        g
    }
}
#[derive(Clone, Copy)]
struct Edge {
    wire: Var,
    shape: Var,
    rate: Var,
    active: Var,
    normal: Var,
    internal: Var,
}
struct Tile {
    kind: Var,
    ports: [Edge; 12],
    jumps: [Var; 32],
    shapes: [Var; 3],
}
struct Grid {
    model: Model,
    grammar: Grammar,
    tiles: BTreeMap<Point, Tile>,
    served: Vec<Var>,
    sources: Vec<(Point, usize, Edge)>,
    targets: Vec<(Point, usize, Edge)>,
    width: i32,
    height: i32,
    layers: i16,
}
fn operation(k: &Kind) -> i64 {
    match k.definition.as_deref() {
        Some("RotatorOneQuadInternalVariant") => 1,
        Some("RotatorOneQuadCCWInternalVariant") => 2,
        Some("RotatorHalfInternalVariant") => 3,
        Some(s) if s.starts_with("Cutter") => 4,
        _ => 0,
    }
}
fn shape_relations() -> Vec<Vec<i64>> {
    let mut rows = vec![vec![0, 0, 0, 0]];
    rows.extend(SHAPES.into_iter().map(|s| vec![0, s, s, s]));
    for (op, a, b, c) in [
        (1, 12, 9, 9),
        (1, 3, 6, 6),
        (1, 8, 1, 1),
        (2, 3, 9, 9),
        (2, 12, 6, 6),
        (2, 2, 1, 1),
        (3, 4, 1, 1),
        (4, 15, 3, 12),
        (4, 9, 1, 8),
        (4, 6, 2, 4),
    ] {
        rows.push(vec![op, a, b, c]);
    }
    rows
}
fn equal_if(model: &mut Model, on: Var, a: Var, b: Var, limit: i64) {
    model.bounded(
        Expr {
            terms: vec![(a, 1), (b, -1), (on, limit)],
            constant: 0,
        },
        i64::MIN,
        limit,
    );
    model.bounded(
        Expr {
            terms: vec![(a, 1), (b, -1), (on, -limit)],
            constant: 0,
        },
        -limit,
        i64::MAX,
    );
}
fn edge_value(e: Edge, s: &Solution) -> i64 {
    let wire = s.value(e.wire);
    let shape = s.value(e.shape);
    let rate = s.value(e.rate);
    if wire == 0 {
        0
    } else if wire == 1 {
        token(shape, rate)
    } else if wire >= 10 {
        1000 + wire * 128 + token(shape, rate)
    } else {
        80 + (wire - 2) * 3
            + match shape {
                12 => 0,
                8 => 1,
                4 => 2,
                _ => unreachable!("Native cutter side shape"),
            }
    }
}
fn build(args: &Args) -> Grid {
    let (width, height) = if args.cell { (5, 4) } else { (34, 14) };
    let layers = args.layers;
    let mut grammar = Grammar::build();
    let mut kinds = Vec::<Kind>::new();
    let remap: Vec<_> = grammar
        .kinds
        .iter()
        .map(|k| {
            if let Some(i) = kinds.iter().position(|other| {
                other.definition == k.definition
                    && other.rotation == k.rotation
                    && other.auxiliary == k.auxiliary
                    && other.jump == k.jump
                    && other.lift == k.lift
            }) {
                i
            } else {
                let i = kinds.len();
                kinds.push(k.clone());
                i
            }
        })
        .collect();
    let rows = grammar
        .rows
        .into_iter()
        .map(|mut row| {
            row[0] = remap[row[0] as usize] as i64;
            for v in &mut row[1..] {
                *v = if *v == 0 {
                    0
                } else if *v >= 1000 {
                    (*v - 1000) / 128
                } else if *v < 80 {
                    1
                } else {
                    2 + (*v - 80) / 3
                };
            }
            row
        })
        .collect::<std::collections::BTreeSet<_>>();
    grammar.rows = rows.into_iter().collect();
    grammar.kinds = kinds;
    let mut model = Model::default();
    let wiring = model.relation(grammar.rows.clone());
    let transforms = model.relation(shape_relations());
    let edge_types = model.relation(
        (0..=105)
            .map(|k| {
                vec![
                    k,
                    i64::from(k != 0),
                    i64::from(k == 1),
                    i64::from(k >= 2),
                    i64::from((2..=9).contains(&k)),
                ]
            })
            .collect(),
    );
    let mut shape_rows = vec![vec![0, 0]];
    shape_rows.extend(SHAPES.into_iter().map(|s| vec![1, s]));
    let edge_shapes = model.relation(shape_rows);
    let classes = model.relation(
        grammar
            .kinds
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let op = operation(k);
                let def = k.definition.as_deref().unwrap_or("");
                let capacity = if i == 0 {
                    0
                } else if k.auxiliary && !k.lift || op == 4 {
                    1
                } else if op != 0 {
                    2
                } else {
                    4
                };
                let delta = if def.starts_with("Splitter") {
                    1
                } else if def.starts_with("Merger3") {
                    -2
                } else if def.starts_with("Merger") {
                    -1
                } else {
                    0
                };
                let mut row = vec![0; 38];
                row[..6].copy_from_slice(&[
                    i as i64,
                    i64::from(i != 0),
                    i64::from(op == 4),
                    capacity,
                    op,
                    delta,
                ]);
                if let Some(j) = k.jump {
                    row[6 + j] = 1;
                }
                row
            })
            .collect(),
    );
    let mut cores = BTreeMap::<Point, (Vec<i64>, i64)>::new();
    if args.cores {
        for i in 0..16 {
            let r = if i < 8 { 3 } else { 1 };
            let origin = if i < 8 {
                (1 + 4 * (i % 8), 5)
            } else {
                (4 + 4 * (i % 8), 8)
            };
            for (x, y, definition, mask, aux, mirrored) in [
                (0, 2, "CutterDefaultInternalVariant", 15, false, false),
                (0, 1, "", 0, true, false),
                (1, 1, "RotatorOneQuadInternalVariant", 12, false, false),
                (1, 2, "RotatorOneQuadCCWInternalVariant", 3, false, false),
                (2, 1, "CutterDefaultInternalVariant", 9, false, false),
                (2, 0, "", 0, true, false),
                (2, 2, "CutterDefaultInternalVariantMirrored", 9, false, true),
                (2, 3, "", 0, true, true),
            ] {
                let (dx, dy) = if r == 3 { (y, -x) } else { (-y, x) };
                let p = (origin.0 + dx, origin.1 + dy);
                let mut allowed = vec![0];
                for row in &grammar.rows {
                    let k = &grammar.kinds[row[0] as usize];
                    if k.rotation != r || k.auxiliary != aux {
                        continue;
                    }
                    if aux {
                        if row[1..5].contains(&(2 + (r * 2 + usize::from(mirrored)) as i64)) {
                            allowed.push(row[0]);
                        }
                    } else if k.definition.as_deref() == Some(definition) {
                        allowed.push(row[0]);
                    }
                }
                allowed.sort_unstable();
                allowed.dedup();
                for z in 0..layers {
                    cores.insert((p.0, p.1, z), (allowed.clone(), mask));
                }
            }
        }
    }
    let open_kinds: Vec<_> = grammar
        .kinds
        .iter()
        .enumerate()
        .filter(|(_, k)| (!k.auxiliary || k.lift) && operation(k) != 4)
        .map(|(i, _)| vec![i as i64])
        .collect();
    let open_table = model.relation(open_kinds);
    let mut edges = BTreeMap::<(Point, usize), Edge>::new();
    let mut tiles = BTreeMap::new();
    let mut objective = Expr::default();
    let mut cuts = Expr::default();
    let mut stream_balance = Expr::default();
    let mut source_flow = Expr::default();
    for z in 0..layers {
        for y in 0..height {
            for x in 0..width {
                let p = (x, y, z);
                let kind = model.variable(0, grammar.kinds.len() as i64 - 1);
                let ports = std::array::from_fn(|i| {
                    let d = i % 6;
                    let key = if i < 6 {
                        (step(p, d, 1), opposite(d))
                    } else {
                        (p, d)
                    };
                    *edges.entry(key).or_insert_with(|| {
                        let wire = model.variable(0, 105);
                        let active = model.boolean();
                        let normal = model.boolean();
                        let internal = model.boolean();
                        let shape = model.variable(0, 15);
                        let rate = model.flow(active, 4);
                        let narrow = model.boolean();
                        model.allow(edge_types, vec![wire, active, normal, internal, narrow]);
                        model.allow(edge_shapes, vec![active, shape]);
                        model.bounded(
                            Expr {
                                terms: vec![(rate, 1), (narrow, 3)],
                                constant: 0,
                            },
                            i64::MIN,
                            4,
                        );
                        Edge {
                            wire,
                            shape,
                            rate,
                            active,
                            normal,
                            internal,
                        }
                    })
                });
                let used = model.boolean();
                let cut = model.boolean();
                let cap = model.variable(0, 4);
                let op = model.variable(0, 4);
                let delta = model.variable(-2, 1);
                let jumps = std::array::from_fn(|_| model.boolean());
                let shapes = std::array::from_fn(|_| model.variable(0, 15));
                let mut cls = vec![kind, used, cut, cap, op, delta];
                cls.extend(jumps);
                model.allow(classes, cls);
                let mut state = vec![kind];
                state.extend(ports.map(|e| e.wire));
                model.allow(wiring, state);
                model.allow(transforms, vec![op, shapes[0], shapes[1], shapes[2]]);
                let mut balance = Expr::from(cut);
                for (i, e) in ports.iter().enumerate() {
                    model.bounded(
                        Expr {
                            terms: vec![(e.rate, 1), (cap, -1)],
                            constant: 0,
                        },
                        i64::MIN,
                        0,
                    );
                    if i < 6 {
                        equal_if(&mut model, e.active, e.shape, shapes[0], 15);
                        balance.term(e.rate, 1);
                    } else {
                        equal_if(&mut model, e.normal, e.shape, shapes[1], 15);
                        equal_if(&mut model, e.internal, e.shape, shapes[2], 15);
                        balance.term(e.rate, -1);
                    }
                }
                let mut upper = balance.clone();
                let mut lower = balance;
                for j in jumps {
                    upper.term(j, -4);
                    lower.term(j, 4);
                }
                model.bounded(upper, i64::MIN, 0);
                model.bounded(lower, 0, i64::MAX);
                if args.cores {
                    if let Some((allowed, mask)) = cores.get(&p) {
                        let rel = model.relation(allowed.iter().map(|&k| vec![k]).collect());
                        model.allow(rel, vec![kind]);
                        if *mask != 0 {
                            let wanted = model.variable(*mask, *mask);
                            equal_if(&mut model, used, shapes[0], wanted, 15);
                        }
                    } else {
                        model.allow(open_table, vec![kind]);
                    }
                }
                objective.term(used, 1);
                cuts.term(cut, 1);
                stream_balance.term(delta, 1);
                model.hint(kind, 0);
                tiles.insert(
                    p,
                    Tile {
                        kind,
                        ports,
                        jumps,
                        shapes,
                    },
                );
            }
        }
    }
    for (&p, t) in &tiles {
        for r in 0..4 {
            let mut nearby = Expr::default();
            let mut count = 0;
            for n in 2..=5 {
                let j = r * 4 + (n - 2) as usize;
                let sender = t.jumps[j];
                let receiver = t.jumps[16 + j];
                if let Some(other) = tiles.get(&step(p, r, n)) {
                    model.equal(
                        Expr {
                            terms: vec![(sender, 1), (other.jumps[16 + j], -1)],
                            constant: 0,
                        },
                        0,
                    );
                    equal_if(
                        &mut model,
                        sender,
                        t.ports[(r + 2) % 4].shape,
                        other.ports[6 + r].shape,
                        15,
                    );
                    equal_if(
                        &mut model,
                        sender,
                        t.ports[(r + 2) % 4].rate,
                        other.ports[6 + r].rate,
                        4,
                    );
                    for k in 0..4 {
                        nearby.term(other.jumps[16 + r * 4 + k], 1);
                    }
                    count += 1;
                } else {
                    model.equal(sender, 0);
                }
                if !tiles.contains_key(&step(p, (r + 2) % 4, n)) {
                    model.equal(receiver, 0);
                }
            }
            if count > 0 {
                for k in 0..4 {
                    nearby.term(t.jumps[r * 4 + k], count - 1);
                }
                model.bounded(nearby, i64::MIN, count);
            }
        }
    }
    let source_ports = if args.cell {
        vec![((-1, 2, layers - 1), 0)]
    } else {
        (0..layers)
            .flat_map(|z| (5..9).map(move |x| ((x, -1, z), 1)))
            .collect()
    };
    let target_ports = if args.cell {
        vec![((4, 3, 0), 0)]
    } else {
        (0..layers)
            .flat_map(|z| {
                (5..9)
                    .flat_map(move |y| [((0, y, z), 2), ((33, y, z), 0)])
                    .chain(
                        [5, 25]
                            .into_iter()
                            .flat_map(move |x| (x..x + 4).map(move |x| ((x, 13, z), 1))),
                    )
            })
            .collect::<Vec<_>>()
    };
    let mut sources = Vec::new();
    let mut targets = Vec::new();
    let mut served = Vec::new();
    for (p, d) in source_ports {
        let e = edges[&(p, d)];
        model.equal(e.internal, 0);
        model.equal(
            Expr {
                terms: vec![(e.shape, 1), (e.active, -15)],
                constant: 0,
            },
            0,
        );
        if args.cell {
            model.bounded(e.rate.into(), 0, 1);
        }
        source_flow.term(e.rate, 1);
        stream_balance.term(e.active, 1);
        sources.push((p, d, e));
    }
    for (p, d) in target_ports {
        let e = edges[&(p, d)];
        model.equal(e.internal, 0);
        model.equal(
            Expr {
                terms: vec![(e.shape, 1), (e.active, -1)],
                constant: 0,
            },
            0,
        );
        model.equal(
            Expr {
                terms: vec![(e.rate, 1), (e.active, -4)],
                constant: 0,
            },
            0,
        );
        let yes = e.active;
        objective.term(yes, -10000);
        cuts.term(yes, -3);
        stream_balance.term(yes, 2);
        source_flow.term(yes, -1);
        served.push(yes);
        targets.push((p, d, e));
    }
    for (&(p, d), e) in &edges {
        let q = step(p, d, 1);
        if (!tiles.contains_key(&p) || !tiles.contains_key(&q))
            && !sources
                .iter()
                .chain(&targets)
                .any(|&(a, b, _)| a == p && b == d)
        {
            model.equal(e.wire, 0);
        }
    }
    model.equal(cuts, 0);
    model.equal(stream_balance, 0);
    model.equal(source_flow, 0);
    model.minimize(objective);
    Grid {
        model,
        grammar,
        tiles,
        served,
        sources,
        targets,
        width,
        height,
        layers,
    }
}
#[derive(Serialize, Deserialize)]
struct CellState {
    position: Point,
    kind: usize,
    ports: [i64; 12],
}
fn selected(grid: &Grid, solution: &Solution) -> Vec<CellState> {
    grid.tiles
        .iter()
        .filter_map(|(&p, t)| {
            let kind = solution.value(t.kind) as usize;
            (kind != 0).then(|| CellState {
                position: p,
                kind,
                ports: t.ports.map(|e| edge_value(e, solution)),
            })
        })
        .collect()
}
fn apply_hint(grid: &mut Grid, p: Point, ports: [i64; 12], kind: Option<usize>) -> Result<usize> {
    let decoded = ports.map(|value| {
        if value == 0 {
            (0, 0, 0)
        } else if value < 80 {
            (1, (value - 1) / 4, (value - 1) % 4 + 1)
        } else if value >= 1000 {
            let normal = (value - 1000) % 128;
            ((value - 1000) / 128, (normal - 1) / 4, (normal - 1) % 4 + 1)
        } else {
            (
                2 + (value - 80) / 3,
                [12, 8, 4][((value - 80) % 3) as usize],
                1,
            )
        }
    });
    let wires = decoded.map(|(w, _, _)| w);
    let shapes = shape_relations();
    let found = grid
        .grammar
        .rows
        .iter()
        .find_map(|row| {
            if row[1..] != wires || kind.is_some_and(|k| k != row[0] as usize) {
                return None;
            }
            let op = operation(&grid.grammar.kinds[row[0] as usize]);
            shapes
                .iter()
                .find(|s| {
                    s[0] == op
                        && decoded.iter().enumerate().all(|(i, &(w, shape, _))| {
                            w == 0
                                || shape
                                    == s[if i < 6 {
                                        1
                                    } else if w == 1 {
                                        2
                                    } else {
                                        3
                                    }]
                        })
                })
                .map(|s| (row[0], s.clone()))
        })
        .ok_or_else(|| anyhow::anyhow!("No matching native grammar state for hint at {p:?}"))?;
    let t = &grid.tiles[&p];
    grid.model.hint(t.kind, found.0);
    for (&v, &value) in t.shapes.iter().zip(&found.1[1..]) {
        grid.model.hint(v, value);
    }
    for (e, (wire, shape, rate)) in t.ports.iter().zip(decoded) {
        for (v, value) in [
            (e.wire, wire),
            (e.shape, shape),
            (e.rate, rate),
            (e.active, i64::from(wire != 0)),
            (e.normal, i64::from(wire == 1)),
            (e.internal, i64::from(wire >= 2)),
        ] {
            grid.model.hint(v, value);
        }
    }
    Ok(found.0 as usize)
}
pub fn run() -> Result<()> {
    let args = Args::parse();
    ensure!(
        (1..=3).contains(&args.layers),
        "One to three native layers are supported"
    );
    let mut grid = build(&args);
    grid.model.bounded(
        Expr::sum(grid.served.iter().copied()),
        args.minimum,
        grid.served.len() as i64,
    );
    if let Some(path) = &args.buildings_hint {
        let buildings: Vec<Building> = serde_json::from_slice(&fs::read(path)?)?;
        let lookup: BTreeMap<_, _> = buildings
            .iter()
            .map(|b| ((b.x - 3, b.y - 3, b.layer), b))
            .collect();
        let mut known = BTreeMap::new();
        for (&p, b) in &lookup {
            if !grid.tiles.contains_key(&p) {
                continue;
            }
            let r = b.rotation.index();
            let jump = if b.definition.starts_with("BeltPort") {
                let sender = b.definition.contains("Sender");
                let d = if sender { r } else { opposite(r) };
                let distance = (2..=5)
                    .find(|&n| {
                        lookup.get(&step(p, d, n)).is_some_and(|other| {
                            other.rotation.index() == r
                                && other.definition.contains(if sender {
                                    "Receiver"
                                } else {
                                    "Sender"
                                })
                        })
                    })
                    .ok_or_else(|| anyhow::anyhow!("Unpaired native jump at {p:?}"))?;
                Some(if sender { 0 } else { 16 } + r * 4 + (distance - 2) as usize)
            } else {
                None
            };
            let kind = grid
                .grammar
                .kinds
                .iter()
                .position(|k| {
                    k.definition.as_deref() == Some(&b.definition)
                        && k.rotation == r
                        && k.jump == jump
                })
                .ok_or_else(|| anyhow::anyhow!("Unsupported native hint {}", b.definition))?;
            known.insert(p, kind);
            if b.definition.starts_with("Cutter") {
                let side = (r + if b.definition.ends_with("Mirrored") {
                    1
                } else {
                    3
                }) % 4;
                let aux = grid
                    .grammar
                    .kinds
                    .iter()
                    .position(|k| k.auxiliary && !k.lift && k.rotation == r)
                    .unwrap();
                known.insert(step(p, side, 1), aux);
            } else if b.definition.starts_with("Lift") {
                let length = b.definition.as_bytes()[4] - b'0';
                let axis = if b.definition.contains("Up") { 4 } else { 5 };
                let aux = grid
                    .grammar
                    .kinds
                    .iter()
                    .position(|k| k.auxiliary && k.lift && k.rotation == r)
                    .unwrap();
                for n in 1..=length {
                    known.insert(step(p, axis, i32::from(n)), aux);
                }
            }
        }
        for (&p, t) in &grid.tiles {
            let kind = known.get(&p).copied().unwrap_or(0) as i64;
            grid.model.hint(t.kind, kind);
            if args.lock_layout {
                grid.model.equal(t.kind, kind);
            }
        }
    }
    if let Some(path) = &args.unit_hint {
        ensure!(!args.cell, "Unit placement hint is for the platform");
        let states: Vec<CellState> = serde_json::from_slice(&fs::read(path)?)?;
        let mut placed = Vec::new();
        for state in states {
            let mut ports = [0; 12];
            for (i, value) in state.ports.into_iter().enumerate() {
                ports[(if i % 6 < 4 { (i % 6 + 2) % 4 } else { i % 6 }) + 6 * (i / 6)] =
                    if (80..104).contains(&value) {
                        80 + (value - 80 + 12) % 24
                    } else {
                        value
                    };
            }
            for z in 0..grid.layers {
                placed.push(((4 - state.position.0, 9 - state.position.1, z), ports));
            }
        }
        for y in 0..=7 {
            let mut ports = [0; 12];
            ports[3] = 61;
            ports[6 + if y == 7 { 2 } else { 1 }] = 61;
            for z in 0..grid.layers {
                placed.push(((5, y, z), ports));
            }
        }
        for (p, ports) in placed {
            apply_hint(&mut grid, p, ports, None)?;
        }
    }
    if let Some(path) = &args.hint {
        let states: Vec<CellState> = serde_json::from_slice(&fs::read(path)?)?;
        let mut known = BTreeMap::new();
        for s in &states {
            let kind = if args.infer_hint {
                let direction = s.ports[..4].iter().position(|&v| v > 0 && v < 80);
                let receiver = s.ports[6..10].iter().position(|&v| v > 0 && v < 80);
                let jump = if let Some(input) =
                    direction.filter(|_| s.ports[6..].iter().all(|&v| v == 0))
                {
                    let d = opposite(input);
                    (2..=5)
                        .find(|&n| {
                            states.iter().any(|b| {
                                b.position == step(s.position, d, n)
                                    && b.ports[..6].iter().all(|&v| v == 0)
                                    && b.ports[6 + d] == s.ports[input]
                            })
                        })
                        .map(|n| d * 4 + (n - 2) as usize)
                } else if let Some(d) = receiver.filter(|_| s.ports[..6].iter().all(|&v| v == 0)) {
                    (2..=5)
                        .find(|&n| {
                            states.iter().any(|b| {
                                b.position == step(s.position, opposite(d), n)
                                    && b.ports[6..].iter().all(|&v| v == 0)
                                    && b.ports[opposite(d)] == s.ports[6 + d]
                            })
                        })
                        .map(|n| 16 + d * 4 + (n - 2) as usize)
                } else {
                    None
                };
                jump.and_then(|j| grid.grammar.kinds.iter().position(|k| k.jump == Some(j)))
            } else {
                Some(s.kind)
            };
            let kind = apply_hint(&mut grid, s.position, s.ports, kind)?;
            known.insert(s.position, kind);
        }
        if let [x0, y0, x1, y1] = args.window[..] {
            for (&p, t) in &grid.tiles {
                if p.0 < x0 || p.0 >= x1 || p.1 < y0 || p.1 >= y1 {
                    grid.model
                        .equal(t.kind, known.get(&p).copied().unwrap_or(0) as i64);
                }
            }
        }
    }
    let (vars, rows) = grid.model.counts();
    eprintln!(
        "Finite grid: {vars} variables, {rows} constraints, {} legal states",
        grid.grammar.rows.len()
    );
    let solution = grid.model.solve_cp_sat(&Limits {
        seconds: args.seconds,
        workers: args.workers,
        log: !args.quiet,
    })?;
    let complete = if solution.feasible() {
        grid.served.iter().map(|&v| solution.value(v)).sum::<i64>()
    } else {
        0
    };
    let path = |s: &str| PathBuf::from(format!("{}{s}", args.output.display()));
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path(".status.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"status":solution.status,"seconds":solution.seconds,"completed":complete,"required":grid.served.len(),"variables":vars,"constraints":rows}),
        )?,
    )?;
    eprintln!(
        "{:?}: {complete}/{} complete outputs in {:.2}s",
        solution.status,
        grid.served.len(),
        solution.seconds
    );
    if solution.feasible() {
        let state = selected(&grid, &solution);
        fs::write(path(".json"), serde_json::to_vec_pretty(&state)?)?;
        let buildings: Vec<_> = state
            .iter()
            .filter_map(|s| {
                let k = &grid.grammar.kinds[s.kind];
                k.definition.as_ref().map(|d| {
                    Building::new(
                        d,
                        s.position.0,
                        s.position.1,
                        s.position.2,
                        Rotation::new(k.rotation),
                    )
                })
            })
            .collect();
        fs::write(
            path(".buildings.json"),
            serde_json::to_vec_pretty(&buildings)?,
        )?;
        if args.native {
            ensure!(
                complete == grid.served.len() as i64,
                "Native acceptance requires complete output flow"
            );
            native(&args, &grid, &solution, buildings)?;
        }
    }
    Ok(())
}
fn native(args: &Args, grid: &Grid, solution: &Solution, base: Vec<Building>) -> Result<()> {
    use shapez_blueprint::{
        Blueprint, BlueprintContent, BuildingBlueprint, Island, IslandBlueprint, encode_blueprint,
    };
    use shapez_runtime::{LabPaths, RunDirectory, run_suite};
    let paths = LabPaths::discover();
    let run = RunDirectory::create(
        &paths.root,
        if args.cell {
            "quarter-grid"
        } else {
            "quarter-cut"
        },
    )?;
    let (offset, layers) = if args.cell {
        ((8, 4), 1)
    } else {
        ((3, 3), if grid.layers == 1 { 3 } else { 1 })
    };
    let mut buildings = Vec::new();
    for z in 0..layers {
        for b in &base {
            let mut b = b.clone();
            b.x += offset.0;
            b.y += offset.1;
            b.layer += z;
            buildings.push(b);
        }
    }
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    for z in 0..layers {
        for &(p, d, v) in &grid.sources {
            ensure!(solution.value(v.rate) > 0, "Unused required source");
            let p = (p.0 + offset.0, p.1 + offset.1, p.2 + z);
            inputs.push(p);
            buildings.push(Building::new(
                if args.cell {
                    "BeltDefaultForwardInternalVariant"
                } else {
                    "BeltPortReceiverInternalVariant"
                },
                p.0,
                p.1,
                p.2,
                Rotation::new(d),
            ));
        }
        for &(p, d, _) in &grid.targets {
            let p = step(p, d, 1);
            let p = (p.0 + offset.0, p.1 + offset.1, p.2 + z);
            outputs.push(p);
            buildings.push(Building::new(
                if args.cell {
                    "BeltDefaultForwardInternalVariant"
                } else {
                    "BeltPortSenderInternalVariant"
                },
                p.0,
                p.1,
                p.2,
                Rotation::new(d),
            ));
        }
    }
    let count = buildings.len();
    let mergers = buildings
        .iter()
        .filter(|b| b.definition.starts_with("Merger"))
        .count();
    let mut island = Island::new(
        if args.cell {
            "Foundation_2x2"
        } else {
            "Foundation_1x2"
        },
        0,
        0,
        0,
        Rotation::new(0),
    );
    island.buildings = Some(BuildingBlueprint::new(buildings));
    let mut bp = Blueprint {
        game_version: 1138,
        content: BlueprintContent::Islands(IslandBlueprint::new(vec![island])),
        icon: None,
        source_format_version: None,
    };
    bp.icon = Some(shapez_blueprint::BlueprintIcon {
        components: [
            Some(shapez_blueprint::IconComponent::Shape("CuCuCuCu".into())),
            Some(shapez_blueprint::IconComponent::Shape("Cu------".into())),
            Some(shapez_blueprint::IconComponent::Shape("Cu------".into())),
            Some(shapez_blueprint::IconComponent::Shape("Cu------".into())),
        ],
    });
    let blueprint = run.path.join(if args.cell {
        "grid.spz2bp"
    } else {
        "quarter-cut-1x2.spz2bp"
    });
    fs::write(&blueprint, encode_blueprint(&bp)?)?;
    eprintln!(
        "Blueprint {} ({}x{})",
        blueprint.display(),
        grid.width,
        grid.height
    );
    if args.cell {
        let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":"grid","blueprint":blueprint,"expected_buildings":count,"inputs":inputs,"outputs":outputs,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterWeaveExperiment"}}]});
        let file = run.path.join("suite.json");
        fs::write(&file, serde_json::to_vec_pretty(&suite)?)?;
        run_suite(&paths, &file, std::time::Duration::from_secs(180))?;
    } else {
        use shapez_model::{Direction, FoundationGeometry, FoundationSocket};
        ensure!(
            grid.layers == 1 || grid.layers == 3,
            "Factory acceptance requires three resulting layers"
        );
        let geometry = FoundationGeometry::strip(2, 3);
        let sockets = [
            (Direction::North, 0, false, "input"),
            (Direction::West, 0, true, "west"),
            (Direction::East, 0, true, "east"),
            (Direction::South, 0, true, "sw"),
            (Direction::South, 1, true, "se"),
        ]
        .map(|(side, segment, output, role)| FoundationSocket {
            side,
            segment,
            output,
            role,
        });
        let mut harness = bp.clone();
        if let BlueprintContent::Islands(islands) = &mut harness.content {
            islands
                .entries
                .extend(shapez_blueprint::foundation_harness(&geometry, &sockets));
        }
        let test = run.path.join("harness.spz2bp");
        fs::write(&test, encode_blueprint(&harness)?)?;
        let suite = serde_json::json!({"projects":["${REPO}/tasks/quarter-cut/interop/QuarterCutTask.csproj"],"cases":[{"name":"factory","blueprint":test,"ports":geometry.ports(&sockets),"expected_buildings":count,"expected_mergers":mergers,"export_graph":true,"experiment":{"assembly":"${LAB}/bin/QuarterCutTask/Release/netstandard2.1/ShapezLab.QuarterCut.dll","type":"QuarterFactoryExperiment"}},{"name":"library-icon","operation":"inspect-blueprint","blueprint":blueprint,"expected_format_version":5,"expected_library_warning":false,"expected_icon":bp.icon.as_ref().unwrap().components}]});
        let file = run.path.join("suite.json");
        fs::write(&file, serde_json::to_vec_pretty(&suite)?)?;
        run_suite(&paths, &file, std::time::Duration::from_secs(180))?;
        run.publish(&paths.root)?;
    }
    Ok(())
}
