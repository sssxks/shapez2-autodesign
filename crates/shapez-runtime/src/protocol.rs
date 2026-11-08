use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct Request<'a, T> {
    pub id: u64,
    pub operation: &'a str,
    pub args: T,
}

/// Transport success and operation-specific acceptance remain separate fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response<T = Box<RawValue>> {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<str>>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suite {
    #[serde(default)]
    pub projects: Vec<PathBuf>,
    pub cases: Vec<TestCase>,
}
/// Native acceptance parameters. JSON is only the transport representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestCase {
    pub name: Box<str>,
    pub operation: Box<str>,
    pub expected_ok: bool,
    /// Negative regressions must match the intended failure, not an unrelated host error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_error: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blueprint: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe: Option<shapez_model::Recipe>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiment: Option<Experiment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<shapez_model::FactoryBeltPort>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_buildings: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_mergers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_graph: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_format_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_library_warning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_icon: Option<[Option<shapez_blueprint::IconComponent>; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<(i32, i32, i16)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<(i32, i32, i16)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_rates: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_full: Option<Vec<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirrored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jump_input: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_row: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_manifest: Option<shapez_model::RateManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warmup_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_rate_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_manifest: Option<ProductionManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_produced: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_probe: Option<bool>,
}
impl Default for TestCase {
    fn default() -> Self {
        Self {
            name: Box::default(),
            operation: "blueprint".into(),
            expected_ok: true,
            expected_error: None,
            blueprint: None,
            recipe: None,
            experiment: None,
            ports: None,
            expected_buildings: None,
            expected_mergers: None,
            export_graph: None,
            expected_format_version: None,
            expected_library_warning: None,
            expected_icon: None,
            inputs: None,
            outputs: None,
            input_rates: None,
            output_full: None,
            mirrored: None,
            jump_input: None,
            output_row: None,
            seconds: None,
            rate_manifest: None,
            warmup_seconds: None,
            minimum_rate_ratio: None,
            production_manifest: None,
            target: None,
            min_produced: None,
            adapter_probe: None,
        }
    }
}
/// Native plugin entry point.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Experiment {
    pub assembly: PathBuf,
    #[serde(rename = "type")]
    pub type_name: Box<str>,
}
/// Construct a typed acceptance case, checking field names and types at compile time.
#[macro_export]
macro_rules! case {
    ($($field:ident : $value:expr),* $(,)?) => {
        $crate::TestCase { $($field: ($value).into(),)* ..Default::default() }
    };
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
            ensure!(
                case.expected_ok || case.expected_error.as_ref().is_some_and(|e| !e.is_empty()),
                "Negative case {} must specify expected_error",
                case.name
            );
        }
        Ok(())
    }
}

/// Production experiment configuration carried directly over the native protocol.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionConfig {
    pub output_chunk: shapez_model::ChunkPos,
    pub signal_receivers: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionManifest {
    #[serde(flatten)]
    pub interface: shapez_model::FactoryInterface,
    pub production: ProductionConfig,
}

/// Host readiness is decoded once at the protocol boundary.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Ready {
    pub kind: Box<str>,
    pub protocol: u32,
    pub pid: u32,
    pub data_path: PathBuf,
    pub version: Box<str>,
    pub startup_seconds: f64,
}
/// Empty operation arguments encode as an object rather than JSON null.
#[derive(Serialize)]
pub struct EmptyArgs {}

