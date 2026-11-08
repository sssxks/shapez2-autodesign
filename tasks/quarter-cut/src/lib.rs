//! Typed in-memory solver results, factory assembly and visualization.
mod assembly;
mod checkpoint;
pub mod design;
pub mod grid;
pub mod search;
mod visualization;
pub mod weave;
use anyhow::Result;
pub use assembly::{assemble_banks, assemble_original_halves, factory_sockets, original_sockets};
use shapez_blueprint::{Blueprint, BlueprintContent, Building, BuildingBlueprint};
use std::{fs, path::Path};

/// Physical interface arrangement used by assembly and the viewer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    WestInput,
    Original,
}

/// One cell's six incoming and six outgoing material-flow tokens.
#[derive(Clone)]
pub struct FlowState {
    pub position: [i32; 3],
    pub ports: [i64; 12],
}

/// Buildings and flow belong to the same solver invocation.
pub struct RegionSolution {
    pub buildings: Vec<Building>,
    pub flow: Vec<FlowState>,
}

/// The two solved regions consumed together by assembly and visualization.
pub struct FactorySolution {
    pub layout: Layout,
    pub left: RegionSolution,
    pub right: RegionSolution,
}
impl FactorySolution {
    /// Assemble the game blueprint from the same result consumed by the viewer.
    pub fn blueprint(&self) -> Result<Blueprint> {
        match self.layout {
            Layout::Original => {
                assemble_original_halves(&self.left.buildings, &self.right.buildings)
            }
            Layout::WestInput => assemble_banks(&self.left.buildings, &self.right.buildings),
        }
    }
    /// Export final user artifacts directly from the in-memory solution.
    pub fn export(&self, destination: &Path) -> Result<()> {
        let blueprint = self.blueprint()?;
        let scene = visualization::scene(&blueprint, self)?;
        let html = shapez_viz::render_html(&scene)?;
        fs::create_dir_all(destination)?;
        blueprint.write(destination.join("factory.spz2bp"))?;
        fs::write(destination.join("factory.html"), html)?;
        println!("Factory and visualization: {}", destination.display());
        Ok(())
    }
}

/// Export a solved regional layout as an importable building blueprint.
pub fn export_buildings(buildings: &[Building], destination: &Path) -> Result<()> {
    let blueprint = Blueprint::new(
        BlueprintContent::Buildings(BuildingBlueprint::new(buildings.to_vec())),
        None,
    );
    if let Some(parent) = destination.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    blueprint.write(destination)?;
    println!("Blueprint: {}", destination.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shapez_blueprint::Rotation;
    fn region() -> RegionSolution {
        let buildings = [1, 2]
            .map(|x| {
                Building::new(
                    "BeltDefaultForwardInternalVariant",
                    x,
                    1,
                    0,
                    Rotation::new(0),
                )
            })
            .into();
        let mut ports = [0; 12];
        ports[6] = 64;
        RegionSolution {
            buildings,
            flow: vec![FlowState {
                position: [1, 1, 0],
                ports,
            }],
        }
    }
    #[test]
    fn assembly_and_viewer_share_region_offsets_layers_and_reflections() {
        for layout in [Layout::Original, Layout::WestInput] {
            let solved = FactorySolution {
                layout,
                left: region(),
                right: region(),
            };
            let blueprint = solved.blueprint().unwrap();
            let scene = visualization::scene(&blueprint, &solved).unwrap();
            let copies = if layout == Layout::Original { 6 } else { 12 };
            assert_eq!(scene.tiles.len(), 60 + copies * 2);
            assert_eq!(
                scene
                    .connections
                    .iter()
                    .filter(|c| c.from[0] == 1 || c.from[0] == 18)
                    .count(),
                copies
            );
            assert!(
                scene
                    .connections
                    .iter()
                    .any(|c| c.from == [18, 1, 2] && c.to == [19, 1, 2])
            );
            if layout == Layout::WestInput {
                assert!(scene.connections.iter().any(|c| c.from == [1, 12, 2]));
            }
            let html = shapez_viz::render_html(&scene).unwrap();
            assert!(!html.contains("__SCENE_JSON__"));
        }
    }
}
