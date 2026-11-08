use shapez_model::{ChunkPos, GridBounds, SearchOptions};
use std::ops::Range;

/// All assumptions about this supplied CMAM live in its task.
pub struct SupplyGroup {
    pub start_x: i32,
    pub modules: Range<usize>,
}
pub struct CmamDesign {
    pub expected_islands: usize,
    pub module_offsets: Vec<i32>,
    pub supply_groups: Vec<SupplyGroup>,
    pub routing_bounds: GridBounds,
    pub search: SearchOptions,
    pub accepted_output: ChunkPos,
    pub signal_receivers: usize,
}
impl Default for CmamDesign {
    fn default() -> Self {
        Self {
            expected_islands: 108,
            module_offsets: vec![0, 5, 10, 15, 20],
            supply_groups: vec![
                SupplyGroup {
                    start_x: -18,
                    modules: 0..3,
                },
                SupplyGroup {
                    start_x: -3,
                    modules: 3..5,
                },
            ],
            routing_bounds: GridBounds {
                min: ChunkPos(-24, -9, 0),
                max: ChunkPos(12, 7, 2),
            },
            search: SearchOptions {
                attempts: 80,
                seed: 1,
            },
            accepted_output: ChunkPos(9, 4, 0),
            signal_receivers: 1,
        }
    }
}
