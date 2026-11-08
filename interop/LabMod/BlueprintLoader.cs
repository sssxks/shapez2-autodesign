using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
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

    /// Use the same island action and nested building action as player blueprint placement.
    internal static void Place(LabWorld world, IslandBlueprint blueprint,
        Func<ChunkPosition, GlobalTileCoordinate, bool> keepBuilding)
    {
        var session = (GameSessionOrchestrator)GameBindings.Session;
        var converter = new BlueprintRawDataConverter(session.DataSerializers, GameBindings.Field<ILogger>(session, "Logger"));
        using var player = new LabPlayerActions(world);
        var islands = new List<ActionModifyIsland.PlacePayload>();
        var checks = new List<(GlobalChunkCoordinate Chunk, PlaceBuildingPayload Building)>();
        int expectedBuildings = 0;
        var expected = new Dictionary<GlobalTileCoordinate, (IBuildingDefinition Definition, GridRotation Rotation)>();
        foreach (var entry in blueprint.Entries)
        {
            var chunk = new GlobalChunkCoordinate(entry.Chunk_L.x, entry.Chunk_L.y, entry.Chunk_L.z);
            var global = new GlobalChunkTransform(chunk, default(GridRotation));
            entry.Definition.TryCreateConfiguration(out var config);
            if (entry.Configuration != null && config != null)
                converter.DeserializeByteArrayIntoData(entry.Configuration, GameVersionEnvironment.CurrentVersion, config.Sync);
            var buildings = new List<PlaceBuildingPayload>();
            if (entry.BuildingBlueprint != null)
                foreach (var building in entry.BuildingBlueprint.Entries)
                {
                    var position = new LocalTileTransform(building.Tile_L, building.Rotation).ToGlobal(global);
                    expected.Add(position.Position, (building.Definition, building.Rotation));
                    building.Definition.TryCreateConfiguration(out var buildingConfig);
                    if (building.AdditionalConfigData != null && buildingConfig != null)
                        converter.DeserializeByteArrayIntoData(building.AdditionalConfigData, GameVersionEnvironment.CurrentVersion, buildingConfig.Sync);
                    var local = new IslandTileTransform(position.Position.ToIslandCoordinate(chunk), building.Rotation);
                    buildings.Add(new PlaceBuildingPayload(default(IslandId), building.Definition, buildingConfig, local));
                    checks.Add((chunk, buildings.Last()));
                }
            expectedBuildings += buildings.Count;
            islands.Add(new ActionModifyIsland.PlacePayload(entry.Definition, config, chunk, entry.Rotation, buildings));
        }
        var action = new ActionModifyIsland(world.Model, player.Player, new ActionModifyIsland.Payload(islands));
        if (!action.TryExecute_INTERNAL(out _, player.Interaction))
            throw new InvalidOperationException("Player blueprint placement was rejected");
        // The island action can succeed while its nested building action rejects the entire payload.
        if (world.Model.IslandCount != islands.Count || world.Model.BuildingCount != expectedBuildings)
        {
            var rejected = new List<string>();
            var occupied = new HashSet<GlobalTileCoordinate>();
            foreach (var check in checks)
            {
                if (world.Model.TryGetIsland(check.Chunk, out var island) && !island.Buildings.Any() &&
                    !ActionModifyBuildings.CheckPlace(check.Building, occupied, player.Player, island.ToDescriptor(), player.Interaction, Array.Empty<BuildingId>()))
                {
                    var group = check.Building.Definition.CustomData.Get<IBuildingDefinitionGroup>();
                    rejected.Add(check.Building.Definition.Id + " at " + check.Building.Transform_I +
                        "; allowed=" + player.Interaction.AllowBuildingGroup(player.Player, group.Id, island.Definition.Id) +
                        "; buildable=" + group.PlayerBuildable + "; unlocked=" + GameBindings.Field<ResearchManager>(session, "Research").Progress.IsUnlocked(group.Id) +
                        "; requirements=" + string.Join("/", group.PlacementRequirements.Select(r => r.GetType().Name)));
                    break;
                }
            }
            throw new InvalidOperationException($"Incomplete player blueprint placement: islands {world.Model.IslandCount}/{islands.Count}, buildings {world.Model.BuildingCount}/{expectedBuildings}; rejected: {string.Join(", ", rejected)}");
        }
        foreach (var building in world.Model.Buildings)
            if (!expected.TryGetValue(building.Transform.Position, out var wanted) || wanted.Definition != building.Definition || !wanted.Rotation.Equals(building.Transform.Rotation))
                throw new InvalidOperationException("Placed building differs from blueprint at " + building.Transform.Position);
        // Diagnostic subgraphs are cut only after the entire blueprint passed native placement.
        var deletions = world.Model.Buildings.Where(b =>
        {
            var p = b.Island.Transform.Position;
            return !keepBuilding(new ChunkPosition(p.x, p.y, p.z), b.Transform.Position);
        }).Select(b => new DeleteBuildingPayload(b.Id)).ToArray();
        if (deletions.Length > 0)
        {
            var delete = new ActionModifyBuildings(world.Model, player.Player, new ModifyBuildingsPayload(deletions));
            if (!delete.TryExecute_INTERNAL(out _, player.Interaction))
                throw new InvalidOperationException("Cannot prepare diagnostic subgraph with native deletion");
        }
    }
}

