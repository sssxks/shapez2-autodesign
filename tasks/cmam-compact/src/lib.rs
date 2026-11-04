mod adapter;
mod compact;
pub use adapter::connect_cmam;
pub use compact::compact_cmam;
pub mod design;
pub use compact::{CompactBuild, build_compact};
pub mod commands;
