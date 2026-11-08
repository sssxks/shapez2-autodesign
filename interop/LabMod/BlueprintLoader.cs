using System;
using System.IO;
using Core.Logging;
using Game.Blueprints;
using Game.Core;
using Game.Core.Blueprint;
using Game.Core.Coordinates;
using Game.Orchestration;

internal static class BlueprintLoader
{
    internal static IslandBlueprint Read(string path)
    {
        if (!GameBindings.Importer.TryImport(File.ReadAllText(path), out var annotated, out _, out var error))
            throw error;
        return annotated.Blueprint as IslandBlueprint ?? throw new ArgumentException("Island blueprint required");
    }

    internal static void Place(LabWorld world, IslandBlueprint blueprint,
        Func<ChunkPosition, GlobalTileCoordinate, bool> keepBuilding)
    {
        var session = (GameSessionOrchestrator)GameBindings.Session;
        var converter = new BlueprintRawDataConverter(session.DataSerializers, GameBindings.Field<ILogger>(session, "Logger"));
        using (world.StartEdit())
        {
            foreach (var entry in blueprint.Entries)
            {
                var transform = new GlobalChunkTransform(new GlobalChunkCoordinate(entry.Chunk_L.x,
                    entry.Chunk_L.y, entry.Chunk_L.z), entry.Rotation);
                entry.Definition.TryCreateConfiguration(out var config);
                if (entry.Configuration != null && config != null)
                    converter.DeserializeByteArrayIntoData(entry.Configuration, GameVersionEnvironment.CurrentVersion, config.Sync);
                world.Model.CreateIsland(entry.Definition, transform, config);
            }
            foreach (var entry in blueprint.Entries)
            {
                if (entry.BuildingBlueprint == null)
                    continue;
                var chunk = new ChunkPosition(entry.Chunk_L.x, entry.Chunk_L.y, entry.Chunk_L.z);
                var transform = new GlobalChunkTransform(new GlobalChunkCoordinate(entry.Chunk_L.x,
                    entry.Chunk_L.y, entry.Chunk_L.z), default(GridRotation));
                foreach (var building in entry.BuildingBlueprint.Entries)
                {
                    var position = new LocalTileTransform(building.Tile_L, building.Rotation).ToGlobal(transform);
                    if (!keepBuilding(chunk, position.Position))
                        continue;
                    building.Definition.TryCreateConfiguration(out var config);
                    if (building.AdditionalConfigData != null && config != null)
                        converter.DeserializeByteArrayIntoData(building.AdditionalConfigData, GameVersionEnvironment.CurrentVersion, config.Sync);
                    CheckPlacement(world.Model, world.Model.GetIsland(transform.Position), building.Definition, position, config);
                    world.Model.CreateBuilding(building.Definition, position, config);
                }
            }
        }
    }

    /// Apply the native floor, layer, and notch queries used by ActionModifyBuildings.CheckPlace.
    /// CreateBuilding is a low-level mutation and does not enforce these player placement rules.
    private static void CheckPlacement(MapModel map, IslandModel island, IBuildingDefinition definition, GlobalTileTransform transform, IBuildingConfiguration? configuration)
    {
        var group = definition.CustomData.Get<IBuildingDefinitionGroup>();
        var maximumLayer = GameBindings.Mode.MaxBuildingLayer;
        var query = new IslandLayoutQuery(island.ToDescriptor(), maximumLayer);
        foreach (var tile in definition.CustomData.Get<IBuildingConnectorData>().Tiles)
        {
            var position = tile.ToGlobal(transform).ToIslandCoordinate(island.Transform.Position);
            string? reason = position.z < 0 || position.z > maximumLayer ? "building layer outside mode range" :
                !(group.AllowPlaceOnNonFilledTiles ? query.IsValidTile_I(position) : query.IsValidAndBuildableTile_I(position)) ? "outside buildable island floor" :
                !group.AllowPlaceOnNotch && query.GetNotchFlag_I(position).HasValue ? "building cannot occupy a notch" : null;
            if (reason != null)
                throw new InvalidOperationException($"Invalid placement: {definition.Id} at island-local {position}, island {island.Definition.Id} at {island.Transform.Position}: {reason}");
        }
        var descriptor = new BuildingDescriptor(definition, transform, configuration, new SimulationStateContainer());
        foreach (var requirement in group.PlacementRequirements)
            // Isolated lab worlds contain foundations only; the hub observer exists only in a player session.
            if (!(requirement is CatapultOnHubBorderRequirement) && !(requirement is BuildingNotOnHubChunkRequirement) && !requirement.Check(map, descriptor, island.ToDescriptor()))
                throw new InvalidOperationException($"Invalid placement requirement: {definition.Id} at {transform.Position}: {requirement.GetType().FullName}");
    }
}




