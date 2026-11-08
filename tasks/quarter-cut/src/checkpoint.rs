//! Explicit, versioned binary restart state; never part of the normal execution pipeline.
use anyhow::{Context, Result, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::{fs, path::Path};

pub(crate) fn save<T: Serialize>(path: &Path, kind: &str, value: &T) -> Result<()> {
    let mut bytes = format!("shapez-quarter-cut/{kind}/1\n").into_bytes();
    bytes.extend(postcard::to_allocvec(value)?);
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes).with_context(|| format!("Write checkpoint {}", path.display()))
}
pub(crate) fn load<T: DeserializeOwned>(path: &Path, kind: &str) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("Read checkpoint {}", path.display()))?;
    let header = format!("shapez-quarter-cut/{kind}/1\n");
    ensure!(
        bytes.starts_with(header.as_bytes()),
        "Wrong checkpoint kind or version: {}",
        path.display()
    );
    postcard::from_bytes(&bytes[header.len()..])
        .with_context(|| format!("Decode checkpoint {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_rejects_other_solver_and_truncated_data() {
        let path = std::env::temp_dir().join(format!(
            "quarter-checkpoint-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let expected = vec![(3_i32, -8_i64), (12, 64)];
        save(&path, "grid", &expected).unwrap();
        assert_eq!(load::<Vec<(i32, i64)>>(&path, "grid").unwrap(), expected);
        assert!(load::<Vec<(i32, i64)>>(&path, "weave").is_err());
        let mut bytes = fs::read(&path).unwrap();
        bytes.pop();
        fs::write(&path, bytes).unwrap();
        assert!(load::<Vec<(i32, i64)>>(&path, "grid").is_err());
        fs::remove_file(path).unwrap();
    }
}
