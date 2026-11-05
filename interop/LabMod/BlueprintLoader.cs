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
                    world.Model.CreateBuilding(building.Definition, position, config);
                }
            }
        }
    }
}
