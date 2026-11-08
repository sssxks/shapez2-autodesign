//! Typed build reports shared by commands, export and native acceptance.
use serde::Serialize;
use shapez_blueprint::Blueprint;
use shapez_model::{ChunkPos, RateManifest};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Serialize)]
pub struct Summary {
    pub version: u32,
    pub islands: usize,
    pub buildings: usize,
    pub types: BTreeMap<String, usize>,
}
impl From<&Blueprint> for Summary {
    fn from(bp: &Blueprint) -> Self {
        let summary = bp.summary();
        Self {
            version: summary.version,
            islands: summary.islands,
            buildings: summary.buildings,
            types: summary
                .types
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        }
    }
}
#[derive(Serialize)]
pub struct Bounds {
    pub min_x: i32,
    pub min_y: i32,
    pub width: i32,
    pub height: i32,
    pub area: i32,
}
#[derive(Serialize)]
pub struct Previous {
    pub summary: Summary,
    pub bounds: Bounds,
}
#[derive(Serialize)]
pub struct Route {
    pub p: ChunkPos,
    pub net: usize,
    pub input: usize,
    pub out_z: i32,
    pub outputs: Vec<usize>,
}
#[derive(Serialize)]
pub struct CompactReport {
    #[serde(flatten)]
    pub boundary: RateManifest,
    pub space_ratio: usize,
    pub production_buildings: usize,
    pub space_nodes: usize,
    pub routing: Vec<Route>,
    pub summary: Summary,
    pub bounds: Bounds,
    pub production: shapez_runtime::ProductionConfig,
    pub previous: Option<Previous>,
}
#[derive(Serialize)]
pub struct Connection {
    pub chunk: [i32; 2],
    pub net: usize,
    pub input: usize,
    pub outputs: BTreeSet<usize>,
}
#[derive(Serialize)]
pub struct AdapterReport {
    pub blueprint: PathBuf,
    pub generated_platforms: usize,
    pub summary: Summary,
    pub connections: Vec<Connection>,
}
