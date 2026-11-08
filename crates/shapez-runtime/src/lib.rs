//! Native host execution and acceptance support.
/// Run directories and published task artifact pointers.
mod artifacts;
/// Path of the prepared local game clone.
mod host;
/// Typed request, response and acceptance-suite wire data.
mod protocol;
/// Suite execution and artifact reporting.
mod suite;
/// Native framed pipe transport.
mod transport;
pub use artifacts::RunDirectory;
pub use host::{Host, LabPaths, prepare_host};
pub use protocol::{
    EmptyArgs, Experiment, ProductionConfig, ProductionManifest, Request, Response, Suite, TestCase,
};
pub use suite::{CaseResult, expand_paths, run_suite, run_suite_file};
