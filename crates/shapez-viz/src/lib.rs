//! Self-contained interactive grid visualizations. Tasks supply geometry and semantics.
use anyhow::{Result, ensure};
use serde::Serialize;
use std::collections::BTreeSet;

/// Integer tile coordinate in a task-defined origin and layer convention.
pub type Point = [i32; 3];

/// One occupied tile; multi-tile buildings share the same owner identifier.
#[derive(Serialize)]
pub struct Tile {
    pub position: Point,
    pub owner: usize,
    pub category: usize,
    pub label: String,
    pub direction: usize,
    pub auxiliary: bool,
    pub description: String,
}

/// Legend category with a task-supplied name and glyph.
#[derive(Serialize)]
pub struct Category {
    pub name: String,
    pub glyph: String,
}

/// A real material connection, including jumps and changes of layer.
#[derive(Serialize)]
pub struct Connection {
    pub from: Point,
    pub to: Point,
    pub material: String,
    pub rate: f64,
    pub highlighted: bool,
    pub jump: bool,
}

/// A rectangular region spanning every layer. Bounds are half-open.
#[derive(Serialize)]
pub struct Region {
    pub bounds: [i32; 4],
    pub name: String,
    pub description: String,
}

/// A named physical boundary port, spanning the indicated line segment.
#[derive(Serialize)]
pub struct Port {
    pub from: [i32; 2],
    pub to: [i32; 2],
    pub name: String,
    pub input: bool,
}

/// Immutable scene data. No game definitions, solver, or runtime are required.
#[derive(Serialize)]
pub struct Scene {
    pub description: String,
    pub width: i32,
    pub height: i32,
    pub layers: i32,
    pub categories: Vec<Category>,
    pub regions: Vec<Region>,
    pub ports: Vec<Port>,
    pub tiles: Vec<Tile>,
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
