use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Debug, Serialize, Deserialize)]
pub struct RunDirectory {
    pub task: Box<str>,
    pub path: PathBuf,
}
impl RunDirectory {
    pub fn create(root: &Path, task: &str) -> Result<Self> {
        validate_task(task)?;
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let id = format!(
            "{stamp}-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let run = Self {
            task: task.into(),
            path: root.join("runs").join(id),
        };
        fs::create_dir_all(&run.path)?;
        fs::write(run.path.join("run.json"), serde_json::to_vec_pretty(&run)?)?;
        Ok(run)
    }
    pub fn snapshot(&self, source: &Path, name: &str) -> Result<PathBuf> {
        ensure!(
            Path::new(name).components().count() == 1 && name != "." && name != "..",
            "Snapshot name must be a filename"
        );
        let dir = self.path.join("inputs");
        fs::create_dir_all(&dir)?;
        let dest = dir.join(name);
        fs::copy(source, &dest)?;
        Ok(dest)
    }
    /// Publish only after the operation has written its complete deliverables.
    pub fn publish(&self, root: &Path) -> Result<()> {
        let dir = root.join("tasks").join(self.task.as_ref());
        fs::create_dir_all(&dir)?;
        let temp = dir.join(format!("latest-{}.tmp", std::process::id()));
        fs::write(&temp, serde_json::to_vec_pretty(self)?)?;
        fs::rename(temp, dir.join("latest.json"))?;
        Ok(())
    }
    pub fn latest(root: &Path, task: &str) -> Result<Self> {
        validate_task(task)?;
        let run: Self = serde_json::from_slice(&fs::read(
            root.join("tasks").join(task).join("latest.json"),
        )?)?;
        ensure!(
            run.task.as_ref() == task && run.path.is_dir(),
            "Invalid task artifact pointer"
        );
        Ok(run)
    }
}
fn validate_task(task: &str) -> Result<()> {
    ensure!(
        !task.is_empty()
            && task
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "Invalid task identifier"
    );
    Ok(())
}
