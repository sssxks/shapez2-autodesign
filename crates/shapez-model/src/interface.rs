use crate::{
    geometry::{ChunkPos, TilePos},
    shape::{Color, Shape},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Payload {
    Shape(Shape),
    Fluid(Color),
}
impl TryFrom<String> for Payload {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self> {
        if s.len() == 1 {
            Ok(Self::Fluid(Color::try_from(s.as_bytes()[0] as char)?))
        } else {
            Ok(Self::Shape(s.parse()?))
        }
    }
}
impl From<Payload> for String {
    fn from(item: Payload) -> String {
        match item {
            Payload::Shape(s) => s.to_string(),
            Payload::Fluid(c) => c.code().to_string(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SupplyPort {
    pub chunk: ChunkPos,
    pub item: Payload,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandPort {
    pub tile: TilePos,
    pub item: Payload,
}

/// Physical factory interfaces have no assumptions about module count or layout.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FactoryInterface {
    pub sources: Vec<SupplyPort>,
    pub targets: Vec<DemandPort>,
}

/// A demand experiment may retain only the receivers in selected islands.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateManifest {
    #[serde(flatten)]
    pub interface: FactoryInterface,
    pub cut_islands: Vec<ChunkPos>,
}
