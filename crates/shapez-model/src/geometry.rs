//! Typed factory domain models.
use serde::{Deserialize, Serialize};

/// An island-grid position. Kept distinct from building tile coordinates.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ChunkPos(pub i32, pub i32, pub i32);

/// Building tile coordinates (x, y, layer).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TilePos(pub i32, pub i32, pub i32);

impl TilePos {
    /// Map tiles to chunks using Euclidean division, including negative coordinates. Chunk dimensions must be positive.
    pub fn chunk(self, tiles_per_chunk: i32, layers_per_chunk: i32) -> ChunkPos {
        ChunkPos(
            self.0.div_euclid(tiles_per_chunk),
            self.1.div_euclid(tiles_per_chunk),
            self.2.div_euclid(layers_per_chunk),
        )
    }
}

/// Cardinal heading in clockwise order starting east.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    /// Positive x heading.
    East,
    /// Positive y heading.
    South,
    /// Negative x heading.
    West,
    /// Negative y heading.
    North,
}

impl Direction {
    /// All cardinal headings in clockwise order.
    pub const ALL: [Self; 4] = [Self::East, Self::South, Self::West, Self::North];
    /// Return the clockwise zero-based heading index.
    pub fn index(self) -> usize {
        self as usize
    }
    /// Return the heading rotated by 180 degrees.
    pub fn opposite(self) -> Self {
        Self::ALL[(self.index() + 2) % 4]
    }
    /// Return the unit tile displacement for this heading.
    pub fn delta(self) -> (i32, i32) {
        [(1, 0), (0, 1), (-1, 0), (0, -1)][self.index()]
    }
}

/// Inclusive minimum and maximum island-grid coordinates.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GridBounds {
    /// Inclusive lower coordinate bound.
    pub min: ChunkPos,
    /// Inclusive upper coordinate bound.
    pub max: ChunkPos,
}

impl GridBounds {
    /// Whether all coordinates lie within the inclusive bounds.
    pub fn contains(self, p: ChunkPos) -> bool {
        p.0 >= self.min.0
            && p.0 <= self.max.0
            && p.1 >= self.min.1
            && p.1 <= self.max.1
            && p.2 >= self.min.2
            && p.2 <= self.max.2
    }
}
