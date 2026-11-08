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
