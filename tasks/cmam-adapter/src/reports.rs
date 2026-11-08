//! Typed build reports shared by commands, export and native acceptance.
use serde::Serialize;
use shapez_blueprint::Blueprint;
use shapez_model::{ChunkPos, RateManifest};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Serialize)]
/// Blueprint counts included in adapter build reports.
pub struct Summary {
    /// Encoded blueprint format version.
    pub version: u32,
    /// Number of platform entries.
    pub islands: usize,
    /// Total contained building count.
    pub buildings: usize,
    /// Building counts grouped by definition name.
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
/// Inclusive chunk-space rectangle enclosing a blueprint.
pub struct Bounds {
    /// Westernmost chunk coordinate.
    pub min_x: i32,
    /// Northernmost chunk coordinate.
    pub min_y: i32,
    /// Horizontal extent in chunks.
    pub width: i32,
    /// Vertical extent in chunks.
    pub height: i32,
    /// Bounding rectangle area in square chunks.
    pub area: i32,
}
#[derive(Serialize)]
/// Size and content of an earlier adapter used for comparison.
pub struct Previous {
    /// Blueprint format and object counts.
    pub summary: Summary,
    /// Occupied chunk bounding rectangle.
    pub bounds: Bounds,
}
#[derive(Serialize)]
/// One logical net routed through a compact platform node.
pub struct Route {
    /// Chunk coordinate of the routing node.
    pub p: ChunkPos,
    /// Logical supply-net index.
    pub net: usize,
    /// Cardinal side receiving this net.
    pub input: usize,
    /// Transport layer used for outgoing connections.
    pub out_z: i32,
    /// Cardinal sides carrying the outgoing branches.
    pub outputs: Vec<usize>,
}
#[derive(Serialize)]
/// Construction results and native production expectations for a compact adapter.
pub struct CompactReport {
    #[serde(flatten)]
    /// Expected material rates at external interfaces.
    pub boundary: RateManifest,
    /// Number of building lanes represented by one space transport connection.
    pub space_ratio: usize,
    /// Building count in the production template.
    pub production_buildings: usize,
    /// Number of routed space-level nodes.
    pub space_nodes: usize,
    /// Resolved per-node supply connections.
    pub routing: Vec<Route>,
    /// Blueprint format and object counts.
    pub summary: Summary,
    /// Occupied chunk bounding rectangle.
    pub bounds: Bounds,
    /// Native production measurement configuration.
    pub production: shapez_runtime::ProductionConfig,
    /// Optional baseline blueprint size and counts.
    pub previous: Option<Previous>,
}
#[derive(Serialize)]
/// Legacy adapter connection at one platform coordinate.
pub struct Connection {
    /// Horizontal chunk coordinates of the connection.
    pub chunk: [i32; 2],
    /// Logical supply-net index.
    pub net: usize,
    /// Cardinal side receiving this net.
    pub input: usize,
    /// Cardinal sides carrying the outgoing branches.
    pub outputs: BTreeSet<usize>,
}
#[derive(Serialize)]
/// Export location and generated connectivity for a legacy adapter.
pub struct AdapterReport {
    /// Path of the generated blueprint artifact.
    pub blueprint: PathBuf,
    /// Number of new routing platforms.
    pub generated_platforms: usize,
    /// Blueprint format and object counts.
    pub summary: Summary,
    /// Logical supply connections realized by those platforms.
    pub connections: Vec<Connection>,
}
