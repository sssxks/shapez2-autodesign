//! Typed factory domain models.
/// Island and tile coordinates and cardinal directions.
mod geometry;
/// Typed physical factory inputs and outputs.
mod interface;
/// Recipe DAGs and native verification traces.
mod recipe;
/// Validated game shape codes and quadrant decoding.
mod shape;
/// Island-grid transport problems and route plans.
mod transport;
pub use geometry::{ChunkPos, Direction, GridBounds, TilePos, numeric_direction};
pub use interface::{DemandPort, FactoryInterface, Payload, RateManifest, SupplyPort};
pub use recipe::{
    Node, Operation, OutputRef, Recipe, RecipeDto, RecipeNode, RecipeNodeDto, RecipeReport,
    RecipeTrace, VerifiedNode, VerifiedRecipe,
};
pub use shape::{Color, Layer, Material, Part, Shape};
pub use transport::{
    RouteNode, RoutePlan, SearchOptions, TransportEndpoint, TransportNet, TransportProblem,
};

/// Physical foundation tiles, ports and route constraints.
mod tile;
pub use tile::{
    ExchangeGroup, FoundationGeometry, TileNet, TilePoint, TileResource, TileRoute, TileRouteStep,
    TileRoutingProblem, tile_resources,
};

pub use tile::{FactoryBeltPort, FoundationSocket};
