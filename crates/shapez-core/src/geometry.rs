use serde::{Deserialize, Serialize};

/// An island-grid position. Kept distinct from building tile coordinates.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ChunkPos(pub i32, pub i32, pub i32);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TilePos(pub i32, pub i32, pub i32);

impl TilePos {
    pub fn chunk(self, tiles_per_chunk: i32) -> ChunkPos {
        ChunkPos(
            self.0.div_euclid(tiles_per_chunk),
            self.1.div_euclid(tiles_per_chunk),
            self.2,
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    East,
    South,
    West,
    North,
}

impl Direction {
    pub const ALL: [Self; 4] = [Self::East, Self::South, Self::West, Self::North];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn opposite(self) -> Self {
        Self::ALL[(self.index() + 2) % 4]
    }
    pub fn delta(self) -> (i32, i32) {
        [(1, 0), (0, 1), (-1, 0), (0, -1)][self.index()]
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GridBounds {
    pub min: ChunkPos,
    pub max: ChunkPos,
}

impl GridBounds {
    pub fn contains(self, p: ChunkPos) -> bool {
        p.0 >= self.min.0
            && p.0 <= self.max.0
            && p.1 >= self.min.1
            && p.1 <= self.max.1
            && p.2 >= self.min.2
            && p.2 <= self.max.2
    }
}
