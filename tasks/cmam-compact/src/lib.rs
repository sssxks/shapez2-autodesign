mod adapter;
mod commands;
mod compact;
mod design;
mod reports;
pub use adapter::connect_cmam;
pub use commands::{run_compact, run_legacy};
pub use compact::{CompactBuild, build_compact, compact_cmam};
pub use design::{CmamDesign, SupplyGroup};
