use crate::{Blueprint, encode_blueprint};
use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet},
    fs,
    path::Path,
};
type P = (i32, i32, i32);
type State = (P, usize);
const D: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
fn n(v: &Value, k: &str) -> i32 {
    v[k].as_i64().unwrap_or(0) as i32
}
#[derive(Clone)]
struct Net {
    root: P,
    heading: usize,
    ends: Vec<(P, usize)>,
    fluid: bool,
}
#[derive(Clone)]
struct Node {
    owner: usize,
    input: usize,
    zout: i32,
    outputs: BTreeSet<usize>,
}
type Routed = HashMap<P, Node>;
fn route(
    netid: usize,
    net: &Net,
    end: (P, usize),
    blocked: &HashSet<P>,
    reserved: &HashMap<P, usize>,
    nodes: &mut Routed,
) -> Result<()> {
    let mut occupied = HashMap::new();
    for (&p, node) in nodes.iter() {
        for z in p.2.min(node.zout)..=p.2.max(node.zout) {
            occupied.insert((p.0, p.1, z), p);
        }
    }
    let allowed = |p: P| {
        p.0 >= -24
            && p.0 <= 12
            && p.1 >= -9
            && p.1 <= 7
            && p.2 >= 0
            && p.2 <= 2
            && !blocked.contains(&p)
            && reserved.get(&p).is_none_or(|&id| id == netid)
    };
    let mut heap = BinaryHeap::new();
    let mut cost = HashMap::<State, i32>::new();
    let mut prev = HashMap::<State, (State, usize, i32)>::new();
    let start = (net.root, net.heading);
    cost.insert(start, 0);
    heap.push(Reverse((0, start)));
    let mut finish = None;
    while let Some(Reverse((c, s))) = heap.pop() {
        if cost.get(&s) != Some(&c) {
            continue;
        }
        let (p, h) = s;
        if (p.0, p.1) == (end.0.0, end.0.1)
            && (p.2 != end.0.2 || end.1 != (h + 2) % 4)
            && (p.2.min(end.0.2)..=p.2.max(end.0.2)).all(|z| {
                allowed((p.0, p.1, z))
                    && occupied.get(&(p.0, p.1, z)).is_none_or(|base| {
                        *base == p
                            && nodes[base].owner == netid
                            && nodes[base].input == h
                            && nodes[base].zout == end.0.2
                    })
            })
        {
            finish = Some(s);
            break;
        }
        if !allowed(p) {
            continue;
        }
        if let Some(base) = occupied.get(&p) {
            if *base != p || nodes[base].owner != netid || nodes[base].input != h {
                continue;
            }
        }
        for z in 0..=2 {
            for d in 0..4 {
                if z == p.2 && d == (h + 2) % 4 {
                    continue;
                }
                if let Some(node) = nodes.get(&p) {
                    if node.zout != z || z != p.2 && !node.outputs.contains(&d) {
                        continue;
                    }
                }
                if (p.2.min(z)..=p.2.max(z)).any(|zz| {
                    !allowed((p.0, p.1, zz))
                        || occupied.get(&(p.0, p.1, zz)).is_some_and(|base| *base != p)
                }) {
                    continue;
                }
                let next = ((p.0 + D[d].0, p.1 + D[d].1, z), d);
                if !allowed(next.0) {
                    continue;
                }
                let step = if nodes.get(&p).is_some_and(|v| v.outputs.contains(&d)) {
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
    let mut steps = vec![(s, end.1, end.0.2)];
    while s != start {
        let &(parent, d, z) = prev.get(&s).unwrap();
        steps.push((parent, d, z));
        s = parent;
    }
    let mut seen = HashSet::new();
    for &(s, _, z) in &steps {
        for zz in s.0.2.min(z)..=s.0.2.max(z) {
            ensure!(seen.insert((s.0.0, s.0.1, zz)), "Self-crossing route");
        }
    }
    for ((p, h), d, z) in steps {
        let node = nodes.entry(p).or_insert_with(|| Node {
            owner: netid,
            input: h,
            zout: z,
            outputs: BTreeSet::new(),
        });
        node.outputs.insert(d);
    }
    Ok(())
}
fn space_island(p: P, node: &Node, fluid: bool) -> Value {
    let r = node.input;
    let out: Vec<usize> = node.outputs.iter().map(|d| (d + 4 - r) % 4).collect();
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
pub fn compact_cmam(input: &Path, rgb: &Path, out: &Path) -> Result<Value> {
    let original = Blueprint::read(input)?;
    let rgb = Blueprint::read(rgb)?;
    ensure!(original.islands().len() == 108, "Expected original CMAM");
    let mut islands = vec![];
    let mut blocked = HashSet::new();
    let mut cuts = vec![];
    for i in original.islands() {
        if n(i, "Y") < 1 {
            continue;
        }
        let Some(bs) = i["B"]["Entries"].as_array() else {
            continue;
        };
        if bs.iter().all(|v| {
            v["T"]
                .as_str()
                .is_some_and(|t| t.starts_with("Trash") || t.starts_with("Pipe"))
        }) {
            continue;
        }
        cuts.push(json!([n(i, "X"), n(i, "Y"), 0]));
        for b in bs {
            blocked.insert((
                (n(i, "X") * 20 + n(b, "X")).div_euclid(20),
                (n(i, "Y") * 20 + n(b, "Y")).div_euclid(20),
                0,
            ));
        }
        islands.push(i.clone());
    }
    let production_buildings: usize = islands
        .iter()
        .map(|i| i["B"]["Entries"].as_array().unwrap().len())
        .sum();
    for j in 0..5 {
        let mut i = rgb.islands()[0].clone();
        i["X"] = json!(-16 + 5 * j);
        i["Y"] = json!(-2);
        for x in -17 + 5 * j..=-15 + 5 * j {
            blocked.insert((x, -2, 0));
        }
        islands.push(i);
    }
    let mut sources = vec![];
    let mut nets = vec![];
    let mut targets = vec![];
    let labels = [
        "P-P-P-P-", "CuCuCuCu", "RuRuRuRu", "SuSuSuSu", "WuWuWuWu", "r", "g", "b",
    ];
    for group in 0..2 {
        let start = if group == 0 { -18 } else { -3 };
        for kind in 0..8 {
            let x = start + kind as i32;
            let fluid = kind >= 5;
            islands.push(json!({"T":if fluid{"Layout_TrainUnloader_Fluids"}else{"Layout_TrainUnloader_Shapes"},"X":x,"Y":-7,"R":2}));
            blocked.insert((x, -7, 0));
            blocked.insert((x, -6, 0));
            sources.push(json!({"chunk":[x,-7,0],"item":labels[kind]}));
            let mut ends = vec![];
            for j in if group == 0 { 0..3 } else { 3..5 } {
                let (col, y, d) = match kind {
                    0 => (-18, 1, 1),
                    1 => (-19, 5, 3),
                    2 => (-17, 5, 3),
                    3 => (-19, 1, 1),
                    4 => (-17, 1, 1),
                    5 => (-17, -3, 1),
                    6 => (-16, -3, 1),
                    _ => (-15, -3, 1),
                };
                ends.push(((col + j * 5, y, 0), d));
            }
            nets.push(Net {
                root: (x, -5, 0),
                heading: 1,
                ends,
                fluid,
            });
        }
    }
    for x in -20..=6 {
        if blocked.contains(&(x, -7, 0)) {
            continue;
        }
        islands.push(json!({"T":if x== -19||x== -4{"Layout_TrainQuickStation"}else{"Rail_Forward"},"X":x,"Y":-7,"R":2,"S":"AQAAAAA="}));
        blocked.insert((x, -7, 0));
    }
    for j in 0..5 {
        for (port, col, y, d) in [(0, -18, 5, 3), (1, -16, 1, 1), (2, -15, 1, 1)] {
            nets.push(Net {
                root: (-17 + 5 * j + port, -1, 0),
                heading: 1,
                ends: vec![((col + 5 * j, y, 0), d)],
                fluid: true,
            });
        }
        for z in 0..3 {
            for k in 0..4 {
                for (col, y, item) in [
                    (-19, 42, "SuSuSuSu"),
                    (-18, 42, "P-P-P-P-"),
                    (-17, 42, "WuWuWuWu"),
                    (-19, 97, "CuCuCuCu"),
                    (-17, 97, "RuRuRuRu"),
                ] {
                    targets.push(json!({"tile":[(col+5*j)*20+8+k,y,z],"item":item}));
                }
                for (col, y) in [(-16, 42), (-15, 42), (-18, 97)] {
                    targets.push(
                        json!({"tile":[(col+5*j)*20+8+k,y,z],"item":(["b","g","r"][z as usize])}),
                    );
                }
            }
        }
    }
    let mut reserved = HashMap::new();
    for (id, net) in nets.iter().enumerate() {
        for p in std::iter::once(net.root).chain(net.ends.iter().map(|e| e.0)) {
            for z in 0..=1 {
                ensure!(
                    reserved.insert((p.0, p.1, z), id).is_none(),
                    "Shared endpoint at {p:?}"
                );
            }
        }
    }
    // Deterministic order search: congestion is resolved by rerouting whole nets,
    // never by reducing transport width or splitting ordinary platform belts.
    let mut best: Option<Routed> = None;
    let mut last_error = String::new();
    for attempt in 0..80 {
        let mut order: Vec<usize> = (0..nets.len()).collect();
        let mut seed = attempt as u64 + 1;
        if attempt > 0 {
            for i in (1..order.len()).rev() {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                order.swap(i, (seed >> 32) as usize % (i + 1));
            }
        }
        let mut nodes = Routed::new();
        let mut good = true;
        for id in order {
            for &end in &nets[id].ends {
                if let Err(e) = route(id, &nets[id], end, &blocked, &reserved, &mut nodes) {
                    last_error = e.to_string();
                    if attempt < 8 {
                        eprintln!("attempt {attempt}: {last_error}");
                    }
                    good = false;
                    break;
                }
            }
            if !good {
                break;
            }
        }
        if good {
            eprintln!("routing attempt {attempt}: {} space nodes", nodes.len());
            best = Some(nodes);
            break;
        }
    }
    let nodes = best.ok_or_else(|| anyhow::anyhow!("Routing failed: {last_error}"))?;
    let mut routing = vec![];
    let ordered: BTreeMap<_, _> = nodes.iter().collect();
    for (&p, node) in ordered {
        islands.push(space_island(p, node, nets[node.owner].fluid));
        routing.push(json!({"p":p,"net":node.owner,"input":node.input,"out_z":node.zout,"outputs":node.outputs}));
    }
    let mut bp = original.value;
    bp["BP"]["Entries"] = json!(islands);
    let result = Blueprint { value: bp };
    fs::create_dir_all(out)?;
    let path = out.join("CMAM compact full-rate.spz2bp");
    fs::write(&path, encode_blueprint(&result.value)?)?;
    let manifest = json!({"sources":sources,"targets":targets,"cut_islands":cuts,"space_ratio":4,"production_buildings":production_buildings,"space_nodes":nodes.len(),"routing":routing,"summary":result.summary()});
    fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(manifest)
}
