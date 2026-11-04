use crate::design::CmamDesign;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use shapez_core::interface::{DemandPort, FactoryInterface, RateManifest, SupplyPort};
use shapez_core::{
    Blueprint, encode_blueprint,
    geometry::{ChunkPos, Direction, TilePos},
    routing::{TransportEndpoint, TransportNet, TransportProblem, solve_transport},
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
pub struct CompactBuild {
    pub blueprint: Blueprint,
    pub manifest: Value,
}

pub fn build_compact(
    original: &Blueprint,
    rgb: &Blueprint,
    previous: Option<&Blueprint>,
    design: &CmamDesign,
) -> Result<CompactBuild> {
    ensure!(
        original.islands().len() == design.expected_islands,
        "Expected original CMAM"
    );
    ensure!(!rgb.islands().is_empty(), "RGB template requires an island");
    ensure!(
        design
            .supply_groups
            .iter()
            .all(|g| g.modules.end <= design.module_offsets.len()),
        "Supply group refers to missing module"
    );
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
        cuts.push(ChunkPos(n(i, "X"), n(i, "Y"), 0));
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
    for &offset in &design.module_offsets {
        let mut i = rgb.islands()[0].clone();
        i["X"] = json!(-16 + offset);
        i["Y"] = json!(-2);
        for x in -17 + offset..=-15 + offset {
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
    for group in &design.supply_groups {
        let start = group.start_x;
        for (kind, label) in labels.iter().enumerate() {
            let x = start + kind as i32;
            let fluid = kind >= 5;
            islands.push(json!({"T":if fluid{"Layout_TrainUnloader_Fluids"}else{"Layout_TrainUnloader_Shapes"},"X":x,"Y":-7,"R":2}));
            blocked.insert(ChunkPos(x, -7, 0));
            blocked.insert(ChunkPos(x, -6, 0));
            sources.push(SupplyPort {
                chunk: ChunkPos(x, -7, 0),
                item: (*label).to_owned().try_into()?,
            });
            let mut ends = vec![];
            for j in group.modules.clone() {
                let offset = design.module_offsets[j];
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
                ends.push(TransportEndpoint {
                    position: ChunkPos(col + offset, y, 0),
                    direction: Direction::ALL[d],
                });
            }
            nets.push(TransportNet {
                source: TransportEndpoint {
                    position: ChunkPos(x, -5, 0),
                    direction: Direction::South,
                },
                sinks: ends,
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
    for &offset in &design.module_offsets {
        for (port, col, y, d) in [(0, -18, 5, 3), (1, -16, 1, 1), (2, -15, 1, 1)] {
            nets.push(TransportNet {
                source: TransportEndpoint {
                    position: ChunkPos(-17 + offset + port, -1, 0),
                    direction: Direction::South,
                },
                sinks: vec![TransportEndpoint {
                    position: ChunkPos(col + offset, y, 0),
                    direction: Direction::ALL[d],
                }],
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
                    targets.push(DemandPort {
                        tile: TilePos((col + offset) * 20 + 8 + k, y, z),
                        item: item.to_owned().try_into()?,
                    });
                }
                for (col, y) in [(-16, 42), (-15, 42), (-18, 97)] {
                    targets.push(DemandPort {
                        tile: TilePos((col + offset) * 20 + 8 + k, y, z),
                        item: ["b", "g", "r"][z as usize].to_owned().try_into()?,
                    });
                }
            }
        }
    }
    let mut reserved = HashMap::new();
    for (id, net) in nets.iter().enumerate() {
        for p in std::iter::once(net.source.position).chain(net.sinks.iter().map(|e| e.position)) {
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
        bounds: design.routing_bounds,
    };
    let nodes = solve_transport(&problem, design.search)?;
    let mut routing = vec![];
    let ordered: BTreeMap<_, _> = nodes.iter().collect();
    for (&p, node) in ordered {
        islands.push(space_island(p, node, problem.nets[node.net].fluid));
        routing.push(json!({"p":p,"net":node.net,"input":node.incoming.index(),"out_z":node.output_layer,"outputs":node.outgoing.iter().map(|d|d.index()).collect::<Vec<_>>()}));
    }
    let mut bp = original.value.clone();
    bp["BP"]["Entries"] = json!(islands);
    let result = Blueprint { value: bp };
    let boundary = RateManifest {
        interface: FactoryInterface { sources, targets },
        cut_islands: cuts,
    };
    let manifest = json!({"sources":boundary.interface.sources,"targets":boundary.interface.targets,"cut_islands":boundary.cut_islands,"space_ratio":4,"production_buildings":production_buildings,"space_nodes":nodes.len(),"routing":routing,"summary":result.summary(),"bounds":bounds(&result),"production":{"output_chunk":design.accepted_output,"signal_receivers":design.signal_receivers},"previous":previous.map(|p|json!({"summary":p.summary(),"bounds":bounds(p)}))});
    Ok(CompactBuild {
        blueprint: result,
        manifest,
    })
}
impl CompactBuild {
    pub fn write(&self, out: &Path) -> Result<()> {
        fs::create_dir_all(out)?;
        fs::write(
            out.join("CMAM compact full-rate.spz2bp"),
            encode_blueprint(&self.blueprint.value)?,
        )?;
        fs::write(
            out.join("manifest.json"),
            serde_json::to_vec_pretty(&self.manifest)?,
        )?;
        fs::write(
            out.join("review.html"),
            include_str!("compact_review.html")
                .replace("__DATA__", &serde_json::to_string(&self.manifest)?),
        )?;
        Ok(())
    }
}
pub fn compact_cmam(
    input: &Path,
    rgb: &Path,
    out: &Path,
    previous: Option<&Path>,
) -> Result<Value> {
    let input = Blueprint::read(input)?;
    let rgb = Blueprint::read(rgb)?;
    let previous = previous.map(Blueprint::read).transpose()?;
    let build = build_compact(&input, &rgb, previous.as_ref(), &CmamDesign::default())?;
    build.write(out)?;
    Ok(build.manifest)
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
