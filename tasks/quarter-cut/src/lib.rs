//! Portable reconstruction and visualization of the delivered factory.
mod assembly;
mod visualization;
use anyhow::Result;
pub use assembly::{assemble_banks, factory_sockets};
use shapez_blueprint::{Blueprint, Building, encode_blueprint};
use std::{fs, path::Path};

/// Reconstruct from versioned bank solutions, without a solver or game installation.
pub fn delivered_blueprint() -> Result<Blueprint> {
    let left: Vec<Building> =
        serde_json::from_str(include_str!("../search/bank-left.buildings.json"))?;
    let right: Vec<Building> =
        serde_json::from_str(include_str!("../search/bank-right.buildings.json"))?;
    assemble_banks(&left, &right)
}

/// Write a freshly assembled blueprint and its interactive offline visualization.
pub fn rebuild(destination: &Path) -> Result<()> {
    let blueprint = delivered_blueprint()?;
    let scene = visualization::scene(&blueprint)?;
    let html = shapez_viz::render_html(&scene)?;
    fs::create_dir_all(destination)?;
    fs::write(
        destination.join("quarter-cut-1x2.spz2bp"),
        encode_blueprint(&blueprint)?,
    )?;
    fs::write(destination.join("quarter-cut-1x2.html"), html)?;
    println!(
        "Rebuilt {} buildings from committed bank solutions",
        blueprint.islands()[0].buildings().len()
    );
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
        let scene = visualization::scene(&rebuilt).unwrap();
        assert_eq!(scene.tiles.iter().filter(|t| !t.auxiliary).count(), 1096);
        assert_eq!(scene.regions.len(), 4);
        assert!(scene.connections.iter().any(|c| c.from[2] != c.to[2]));
    }
}
