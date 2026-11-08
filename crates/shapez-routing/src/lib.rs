//! Transport and tile routing over the shared Shapez domain model.

mod solver;
pub use solver::solve_transport;

mod tile_negotiated;
pub use tile_negotiated::{
    assign_reachable_targets, reachable_targets, solve_tiles_negotiated, source_blockers,
};

mod tile_sequential;
pub use tile_sequential::solve_tiles_sequential;
