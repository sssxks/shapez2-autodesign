//! Self-contained interactive grid visualizations. Tasks supply geometry and semantics.
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::BTreeSet;

/// Integer tile coordinate in a task-defined origin and layer convention.
pub type Point = [i32; 3];

/// One occupied tile; multi-tile buildings share the same owner identifier.
#[derive(Serialize)]
pub struct Tile {
    /// Occupied tile coordinate.
    pub position: Point,
    /// Building identifier shared by all tiles of its footprint.
    pub owner: usize,
    /// Index into the scene legend categories.
    pub category: usize,
    /// Short display label.
    pub label: Box<str>,
    /// Clockwise orientation index starting east.
    pub direction: usize,
    /// Whether this tile is an auxiliary part of a building footprint.
    pub auxiliary: bool,
    /// Detailed text displayed by the viewer.
    pub description: Box<str>,
}

/// Legend category with a task-supplied name and glyph.
#[derive(Serialize)]
pub struct Category {
    /// Human-readable name used to identify this record.
    pub name: Box<str>,
    /// Compact legend symbol.
    pub glyph: Box<str>,
}

/// A real material connection, including jumps and changes of layer.
#[derive(Serialize)]
pub struct Connection {
    /// Connection or port segment start coordinate.
    pub from: Point,
    /// Connection or port segment end coordinate.
    pub to: Point,
    /// Shape code describing the transported material.
    pub material: Box<str>,
    /// Material flow rate in task-defined units.
    pub rate: f64,
    /// Whether to emphasize this connection.
    pub highlighted: bool,
    /// Whether this connection crosses a transport jump.
    pub jump: bool,
}

/// A rectangular region spanning every layer. Bounds are half-open.
#[derive(Serialize)]
pub struct Region {
    /// Half-open rectangle [min_x, min_y, max_x, max_y].
    pub bounds: [i32; 4],
    /// Human-readable name used to identify this record.
    pub name: Box<str>,
    /// Detailed text displayed by the viewer.
    pub description: Box<str>,
}

/// A named physical boundary port, spanning the indicated line segment.
#[derive(Serialize)]
pub struct Port {
    /// Connection or port segment start coordinate.
    pub from: [i32; 2],
    /// Connection or port segment end coordinate.
    pub to: [i32; 2],
    /// Human-readable name used to identify this record.
    pub name: Box<str>,
    /// Whether flow enters the factory.
    pub input: bool,
}

/// Immutable scene data. No game definitions, solver, or runtime are required.
#[derive(Serialize)]
pub struct Scene {
    /// Detailed text displayed by the viewer.
    pub description: Box<str>,
    /// Scene width in tiles.
    pub width: i32,
    /// Scene height in tiles.
    pub height: i32,
    /// Number of native building layers.
    pub layers: i32,
    /// Legend entries indexed by tiles.
    pub categories: Vec<Category>,
    /// Named rectangular regions.
    pub regions: Vec<Region>,
    /// Named boundary port segments.
    pub ports: Vec<Port>,
    /// Occupied tiles with building ownership.
    pub tiles: Vec<Tile>,
    /// Material flow edges.
    pub connections: Vec<Connection>,
}

/// Emit an offline HTML page with embedded data, CSS, and interaction logic.
pub fn render_html(scene: &Scene) -> Result<String> {
    ensure!(
        scene.width > 0 && scene.height > 0 && scene.layers > 0,
        "Empty scene bounds"
    );
    let mut positions = BTreeSet::new();
    for tile in &scene.tiles {
        ensure!(
            (0..scene.layers).contains(&tile.position[2]),
            "Tile outside layer range"
        );
        ensure!(
            tile.category < scene.categories.len(),
            "Unknown tile category"
        );
        ensure!(
            positions.insert(tile.position),
            "Duplicate occupied tile {:?}",
            tile.position
        );
    }
    let data = serde_json::to_string(scene)?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029");
    Ok(include_str!("viewer.html")
        .replace("__VIEWER_CSS__", include_str!("viewer.css"))
        .replace("__VIEWER_JS__", include_str!("viewer.js"))
        .replace("__SCENE_JSON__", &data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_labels_cannot_close_the_data_script() {
        let scene = Scene {
            description: "</script><script>alert(1)</script>".into(),
            width: 1,
            height: 1,
            layers: 1,
            categories: vec![],
            regions: vec![],
            ports: vec![],
            tiles: vec![],
            connections: vec![],
        };
        let html = render_html(&scene).unwrap();
        assert!(!html.contains("<script>alert(1)"));
        assert!(html.contains("\\u003c/script>"));
    }
}
