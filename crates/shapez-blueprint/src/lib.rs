//! Typed Shapez 2 layouts, placement helpers and envelope codecs.

/// Envelope codec and borrowed summaries.
mod blueprint;
/// Editable layouts and strict wire adapters.
mod model;
/// Transport junction encoding.
mod space;
pub use blueprint::{
    BLUEPRINT_FORMAT_VERSION, Blueprint, BlueprintSummary, LabelSummary, TrainInterfaceSummary,
    decode_blueprint, encode_blueprint,
};
pub use model::{
    BlueprintContent, BlueprintIcon, Building, BuildingBlueprint, Configuration, IconComponent,
    Island, IslandBlueprint, Rotation,
};
pub use space::space_island;

/// Tile route building encoding.
mod tile;
pub use tile::route_buildings;

/// Reusable cells and placement transforms.
mod cell;
pub use cell::{Cell, CellPort, Placement};
