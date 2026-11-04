use anyhow::{Result, ensure};
use serde_json::{Value, json};
use shapez_core::{
    Blueprint, encode_blueprint,
    geometry::{ChunkPos, Direction, GridBounds},
    routing::{SearchOptions, TransportNet, TransportProblem, solve_transport},
    space::space_island,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
};
fn n(v: &Value, k: &str) -> i32 {
    v[k].as_i64().unwrap_or(0) as i32
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
            blocked.insert(ChunkPos(
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
            blocked.insert(ChunkPos(x, -2, 0));
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
        for (kind, label) in labels.iter().enumerate() {
            let x = start + kind as i32;
            let fluid = kind >= 5;
            islands.push(json!({"T":if fluid{"Layout_TrainUnloader_Fluids"}else{"Layout_TrainUnloader_Shapes"},"X":x,"Y":-7,"R":2}));
            blocked.insert(ChunkPos(x, -7, 0));
            blocked.insert(ChunkPos(x, -6, 0));
            sources.push(json!({"chunk":[x,-7,0],"item":label}));
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
                ends.push((ChunkPos(col + j * 5, y, 0), Direction::ALL[d]));
            }
            nets.push(TransportNet {
                root: ChunkPos(x, -5, 0),
                heading: Direction::South,
                ends,
                fluid,
            });
        }
    }
    for x in -20..=6 {
        if blocked.contains(&ChunkPos(x, -7, 0)) {
            continue;
        }
        islands.push(json!({"T":if x== -19||x== -4{"Layout_TrainQuickStation"}else{"Rail_Forward"},"X":x,"Y":-7,"R":2,"S":"AQAAAAA="}));
        blocked.insert(ChunkPos(x, -7, 0));
    }
    for j in 0..5 {
        for (port, col, y, d) in [(0, -18, 5, 3), (1, -16, 1, 1), (2, -15, 1, 1)] {
            nets.push(TransportNet {
                root: ChunkPos(-17 + 5 * j + port, -1, 0),
                heading: Direction::South,
                ends: vec![(ChunkPos(col + 5 * j, y, 0), Direction::ALL[d])],
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
                    reserved.insert(ChunkPos(p.0, p.1, z), id).is_none(),
                    "Shared endpoint at {p:?}"
                );
            }
        }
    }
    let problem = TransportProblem {
        nets,
        blocked,
        reserved,
        bounds: GridBounds {
            min: ChunkPos(-24, -9, 0),
            max: ChunkPos(12, 7, 2),
        },
    };
    let nodes = solve_transport(
        &problem,
        SearchOptions {
            attempts: 80,
            seed: 1,
        },
    )?;
    let mut routing = vec![];
    let ordered: BTreeMap<_, _> = nodes.iter().collect();
    for (&p, node) in ordered {
        islands.push(space_island(p, node, problem.nets[node.owner].fluid));
        routing.push(json!({"p":p,"net":node.owner,"input":node.input.index(),"out_z":node.zout,"outputs":node.outputs.iter().map(|d|d.index()).collect::<Vec<_>>()}));
    }
    let mut bp = original.value;
    bp["BP"]["Entries"] = json!(islands);
    let result = Blueprint { value: bp };
    fs::create_dir_all(out)?;
    let path = out.join("CMAM compact full-rate.spz2bp");
    fs::write(&path, encode_blueprint(&result.value)?)?;
    let previous = out.parent().unwrap().join("adapter/CMAM connected.spz2bp");
    let previous = if previous.exists() {
        Some(Blueprint::read(previous)?)
    } else {
        None
    };
    let manifest = json!({"sources":sources,"targets":targets,"cut_islands":cuts,"space_ratio":4,"production_buildings":production_buildings,"space_nodes":nodes.len(),"routing":routing,"summary":result.summary(),"bounds":bounds(&result),"previous":previous.as_ref().map(|p|json!({"summary":p.summary(),"bounds":bounds(p)}))});
    fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::write(
        out.join("review.html"),
        include_str!("compact_review.html").replace("__DATA__", &serde_json::to_string(&manifest)?),
    )?;
    Ok(manifest)
}

fn bounds(bp: &Blueprint) -> Value {
    let mut points = vec![];
    for i in bp.islands() {
        let (x, y) = (n(i, "X"), n(i, "Y"));
        points.push((x, y));
        if let Some(bs) = i["B"]["Entries"].as_array() {
            for b in bs {
                points.push((
                    (x * 20 + n(b, "X")).div_euclid(20),
                    (y * 20 + n(b, "Y")).div_euclid(20),
                ));
            }
        }
    }
    let x0 = points.iter().map(|p| p.0).min().unwrap();
    let x1 = points.iter().map(|p| p.0).max().unwrap();
    let y0 = points.iter().map(|p| p.1).min().unwrap();
    let y1 = points.iter().map(|p| p.1).max().unwrap();
    json!({"min_x":x0,"min_y":y0,"width":x1-x0+1,"height":y1-y0+1,"area":(x1-x0+1)*(y1-y0+1)})
}
