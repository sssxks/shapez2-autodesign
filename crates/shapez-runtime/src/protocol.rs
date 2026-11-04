use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct Request<'a, T> {
    pub id: u64,
    pub operation: &'a str,
    pub args: T,
}

/// Transport success and operation-specific acceptance remain separate fields.
#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T = Value> {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub wall_seconds: Option<f64>,
}
impl<T> Response<T> {
    pub fn into_result(self) -> Result<T> {
        ensure!(
            self.ok,
            "{}",
            self.error.as_deref().unwrap_or("Native operation failed")
        );
        self.result
            .ok_or_else(|| anyhow::anyhow!("Native response omitted result"))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Suite {
    #[serde(default)]
    pub projects: Vec<PathBuf>,
    pub cases: Vec<TestCase>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    #[serde(default = "blueprint_operation")]
    pub operation: String,
    #[serde(default = "yes")]
    pub expected_ok: bool,
    /// Task-specific parameters belong to their task, not this runner.
    #[serde(flatten)]
    pub args: Map<String, Value>,
}
fn blueprint_operation() -> String {
    "blueprint".into()
}
fn yes() -> bool {
    true
}
impl Suite {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.cases.is_empty(), "Suite must contain cases");
        let mut names = std::collections::HashSet::new();
        for case in &self.cases {
            ensure!(
                !case.name.is_empty() && names.insert(&case.name),
                "Case names must be nonempty and unique"
            );
            ensure!(!case.operation.is_empty(), "Operation must be nonempty");
        }
        Ok(())
    }
}
