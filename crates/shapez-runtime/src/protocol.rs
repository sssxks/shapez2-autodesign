//! Native host execution and acceptance support.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::path::PathBuf;

/// Request envelope pairing a sequence number with typed operation arguments.
#[derive(Debug, Serialize)]
pub struct Request<'a, T> {
    /// Sequence or execution-order identifier.
    pub id: u64,
    /// Native operation dispatch name.
    pub operation: &'a str,
    /// Typed operation arguments.
    pub args: T,
}

/// Transport success and operation-specific acceptance remain separate fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response<T = Box<RawValue>> {
    /// Sequence or execution-order identifier.
    pub id: u64,
    /// Whether the native request completed successfully.
    pub ok: bool,
    /// Typed native result, if supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    /// Native failure message, if supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Box<str>>,
    /// Native wall-clock execution duration in seconds.
    pub wall_seconds: Option<f64>,
}
impl<T> Response<T> {
    /// Extract a successful payload or report the host error or missing payload.
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

/// Native acceptance cases and projects to build before execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suite {
    /// Projects built before the suite starts.
    #[serde(default)]
    pub projects: Vec<PathBuf>,
    /// Acceptance cases in execution order.
    pub cases: Vec<TestCase>,
}
/// Native acceptance parameters. JSON is only the transport representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TestCase {
    /// Human-readable name used to identify this record.
    pub name: Box<str>,
    /// Native operation dispatch name.
    pub operation: Box<str>,
    /// Required native transport success status.
    pub expected_ok: bool,
    /// Negative regressions must match the intended failure, not an unrelated host error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_error: Option<Box<str>>,
    /// Blueprint file passed to the native operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blueprint: Option<PathBuf>,
    /// Typed recipe submitted for native verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe: Option<shapez_model::Recipe>,
    /// Plugin entry point for a custom experiment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experiment: Option<Experiment>,
    /// Named boundary port segments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ports: Option<Vec<shapez_model::FactoryBeltPort>>,
    /// Required building count when specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_buildings: Option<usize>,
    /// Required merger count when specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_mergers: Option<usize>,
    /// Request native connection graph export.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_graph: Option<bool>,
    /// Required blueprint format version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_format_version: Option<u32>,
    /// Required blueprint library warning status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_library_warning: Option<bool>,
    /// Required four-component blueprint icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_icon: Option<[Option<shapez_blueprint::IconComponent>; 4]>,
    /// Ordered input references or coordinates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<(i32, i32, i16)>>,
    /// Native output values in port order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<(i32, i32, i16)>>,
    /// Supply rates in input-port order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_rates: Option<Vec<f64>>,
    /// Expected saturation flags in output-port order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_full: Option<Vec<bool>>,
    /// Select the mirrored native fixture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirrored: Option<bool>,
    /// Feed the fixture through a jump.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jump_input: Option<bool>,
    /// Fixture output row coordinate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_row: Option<i32>,
    /// Measurement duration in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
    /// Physical supply and demand rate experiment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_manifest: Option<shapez_model::RateManifest>,
    /// Warmup duration excluded from measurement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warmup_seconds: Option<u32>,
    /// Minimum accepted measured-to-requested rate ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_rate_ratio: Option<f64>,
    /// Physical production experiment specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_manifest: Option<ProductionManifest>,
    /// Requested final shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Box<str>>,
    /// Minimum accepted produced item count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_produced: Option<u64>,
    /// Request native adapter diagnostics.
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
    /// Path to the native plugin assembly.
    pub assembly: PathBuf,
    /// Fully qualified native plugin entry type.
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
    /// Reject inconsistent inputs before execution.
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
    /// Island containing the production output.
    pub output_chunk: shapez_model::ChunkPos,
    /// Number of signal receivers used by the experiment.
    pub signal_receivers: usize,
}
/// Physical interface and production experiment settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionManifest {
    /// Physical supply and demand declarations.
    #[serde(flatten)]
    pub interface: shapez_model::FactoryInterface,
    /// Production experiment configuration.
    pub production: ProductionConfig,
}

/// Host readiness is decoded once at the protocol boundary.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Ready {
    /// Readiness message discriminator.
    pub kind: Box<str>,
    /// Native protocol version.
    pub protocol: u32,
    /// Native host process identifier.
    pub pid: u32,
    /// Game data directory reported by the host.
    pub data_path: PathBuf,
    /// Game version reported by the host.
    pub version: Box<str>,
    /// Native startup wall time in seconds.
    pub startup_seconds: f64,
}
/// Empty operation arguments encode as an object rather than JSON null.
#[derive(Serialize)]
pub struct EmptyArgs {}
