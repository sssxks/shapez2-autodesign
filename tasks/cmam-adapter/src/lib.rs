//! Build compact and legacy adapters from typed blueprints.
mod adapter;
/// File-oriented build workflows and optional native acceptance.
mod commands;
/// Compact adapter construction and chunk-space reporting.
mod compact;
/// Production template configuration and supply grouping.
mod design;
mod reports;
pub use adapter::connect_cmam;
pub use commands::{run_compact, run_legacy};
pub use compact::{CompactBuild, build_compact, compact_cmam};
pub use design::{CmamDesign, SupplyGroup};
