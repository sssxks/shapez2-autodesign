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
use anyhow::{Context, Result};
pub use assembly::{assemble_banks, assemble_original_halves, factory_sockets, original_sockets};
use shapez_blueprint::{Blueprint, Building, encode_blueprint};
use std::{fs, path::Path};

/// Physical interface arrangement, shared by reconstruction and the flow viewer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    WestInput,
    Original,
}

/// Read a solver output at runtime, retaining its path in error messages.
fn read_solution<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let file = fs::File::open(path)
        .with_context(|| format!("Cannot open solver result {}", path.display()))?;
    serde_json::from_reader(std::io::BufReader::new(file))
        .with_context(|| format!("Cannot decode solver result {}", path.display()))
}

/// Reconstruct the west-input design from runtime bank solutions.
pub fn delivered_blueprint(input: &Path) -> Result<Blueprint> {
    let left: Vec<Building> = read_solution(&input.join("bank-left.buildings.json"))?;
    let right: Vec<Building> = read_solution(&input.join("bank-right.buildings.json"))?;
    assemble_banks(&left, &right)
}

/// Reconstruct the north-input design from runtime regional solutions.
pub fn original_blueprint(input: &Path) -> Result<Blueprint> {
    let left: Vec<Building> = read_solution(&input.join("original/left.buildings.json"))?;
    let right: Vec<Building> = read_solution(&input.join("original/right.buildings.json"))?;
    assemble_original_halves(&left, &right)
}

/// Write both verified interface variants and their interactive offline visualizations.
pub fn rebuild(input: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for (blueprint, layout, name) in [
        (
            original_blueprint(input)?,
            Layout::Original,
            "quarter-cut-1x2-original",
        ),
        (
            delivered_blueprint(input)?,
            Layout::WestInput,
            "quarter-cut-1x2",
        ),
    ] {
        let html = shapez_viz::render_html(&visualization::scene(&blueprint, layout, input)?)?;
        fs::write(
            destination.join(format!("{name}.spz2bp")),
            encode_blueprint(&blueprint)?,
        )?;
        fs::write(destination.join(format!("{name}.html")), html)?;
        println!(
            "Rebuilt {name}: {} buildings from runtime solver results",
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
    fn solver_results_are_read_at_runtime_with_path_diagnostics() {
        let directory = std::env::temp_dir().join(format!(
            "quarter-cut-runtime-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("result.json");
        let missing = read_solution::<Vec<i64>>(&path).unwrap_err();
        assert!(missing.to_string().contains(&path.display().to_string()));
        fs::write(&path, "[1]").unwrap();
        assert_eq!(read_solution::<Vec<i64>>(&path).unwrap(), [1]);
        fs::write(&path, "[2,3]").unwrap();
        assert_eq!(read_solution::<Vec<i64>>(&path).unwrap(), [2, 3]);
        fs::write(&path, "invalid JSON").unwrap();
        let malformed = read_solution::<Vec<i64>>(&path).unwrap_err();
        assert!(malformed.to_string().contains(&path.display().to_string()));
        fs::remove_dir_all(&directory).unwrap();
    }
}
