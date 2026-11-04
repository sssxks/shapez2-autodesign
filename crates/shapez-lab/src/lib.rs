// Compatibility facade for the command-line application.
pub use cmam_compact::{compact_cmam, connect_cmam};
pub use scaffold_recipe::plan_recipe;
pub use shapez_core::{Blueprint, decode_blueprint, encode_blueprint};
pub use shapez_runtime::{Host, LabPaths, prepare_host};
