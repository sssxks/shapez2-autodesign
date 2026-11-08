//! Portable reconstruction and visualization of the delivered factory.
mod assembly;
pub mod design;
#[cfg(feature = "search")]
pub mod grid;
#[cfg(feature = "search")]
pub mod search;
mod visualization;
#[cfg(feature = "search")]
pub mod weave;
use anyhow::Result;
pub use assembly::{assemble_banks, assemble_original_halves, factory_sockets, original_sockets};
use shapez_blueprint::{Blueprint, Building, encode_blueprint};
use std::{fs, path::Path};

/// Physical interface arrangement, shared by reconstruction and the flow viewer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    WestInput,
    Original,
}

/// Reconstruct from versioned bank solutions, without a solver or game installation.
pub fn delivered_blueprint() -> Result<Blueprint> {
    let left: Vec<Building> =
        serde_json::from_str(include_str!("../search/bank-left.buildings.json"))?;
    let right: Vec<Building> =
        serde_json::from_str(include_str!("../search/bank-right.buildings.json"))?;
    assemble_banks(&left, &right)
}

/// Reconstruct the original north-input design from the two committed regional solutions.
pub fn original_blueprint() -> Result<Blueprint> {
    let left: Vec<Building> =
        serde_json::from_str(include_str!("../search/original/left.buildings.json"))?;
    let right: Vec<Building> =
        serde_json::from_str(include_str!("../search/original/right.buildings.json"))?;
    assemble_original_halves(&left, &right)
}

/// Write both verified interface variants and their interactive offline visualizations.
pub fn rebuild(destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for (blueprint, layout, name) in [
        (
            original_blueprint()?,
            Layout::Original,
            "quarter-cut-1x2-original",
        ),
        (delivered_blueprint()?, Layout::WestInput, "quarter-cut-1x2"),
    ] {
        let html = shapez_viz::render_html(&visualization::scene(&blueprint, layout)?)?;
        fs::write(
            destination.join(format!("{name}.spz2bp")),
            encode_blueprint(&blueprint)?,
        )?;
        fs::write(destination.join(format!("{name}.html")), html)?;
        println!(
            "Rebuilt {name}: {} buildings from committed regional solutions",
            blueprint.islands()[0].buildings().len()
        );
    }
    println!(
        "Blueprint and visualization: {}",
        destination.canonicalize()?.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconstruction_matches_the_native_verified_template() {
        let rebuilt = delivered_blueprint().unwrap();
        let expected =
            shapez_blueprint::decode_blueprint(include_str!("../templates/quarter-cut-1x2.spz2bp"))
                .unwrap();
        assert_eq!(
            rebuilt.islands()[0].buildings(),
            expected.islands()[0].buildings()
        );
        assert_eq!(rebuilt.icon, expected.icon);
        let scene = visualization::scene(&rebuilt, Layout::WestInput).unwrap();
        assert_eq!(scene.tiles.iter().filter(|t| !t.auxiliary).count(), 1096);
        assert_eq!(scene.regions.len(), 4);
        assert!(scene.connections.iter().any(|c| c.from[2] != c.to[2]));
    }

    #[test]
    fn original_interfaces_reconstruct_the_native_verified_factory() {
        let rebuilt = original_blueprint().unwrap();
        let expected = shapez_blueprint::decode_blueprint(include_str!(
            "../templates/quarter-cut-1x2-original.spz2bp"
        ))
        .unwrap();
        assert_eq!(
            rebuilt.islands()[0].buildings(),
            expected.islands()[0].buildings()
        );
        assert_eq!(rebuilt.icon, expected.icon);
        let scene = visualization::scene(&rebuilt, Layout::Original).unwrap();
        assert_eq!(scene.tiles.iter().filter(|t| !t.auxiliary).count(), 1134);
        assert_eq!(scene.regions.len(), 2);
        assert_eq!(scene.ports.iter().find(|p| p.input).unwrap().from, [5, -1]);
        assert!(!scene.connections.iter().any(|c| c.from[2] != c.to[2]));
    }
}
