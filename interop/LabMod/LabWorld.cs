using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using Core.Dependency;
using Core.Logging;
using Game.Blueprints;
using Game.Core;
using Game.Core.Blueprint;
using Game.Core.Blueprint.Importer;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Map.Simulation.Clustering;
using Game.Core.Simulation;
using Game.Orchestration;
using Game.Content.Features.Signals.Channels;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

public sealed class LabWorld : IDisposable
{
    readonly MapLayout layout; readonly Map map; readonly MapModel model; readonly Simulator simulator;
    readonly SimulationGraph graph; readonly ISimulationSystem[] systems; readonly ProcessingUpdateStrategy strategy;
    readonly GameMode mode; readonly ILogger logger; readonly BlueprintRawDataConverter converter; readonly IBlueprintImporter importer;
    public LabWorld()
    {
        var session = GameBindings.Session;
        logger = GameBindings.Field<ILogger>(session, "Logger");
        mode = GameBindings.Mode;
        var deps = GameBindings.Field<DependencyContainer>(session, "DependencyContainer");
        var generator = new DefaultMapGenerator(mode.Seed, mode.MapGenerationParameters, mode.Scenario.StartingLocation, mode.ShapesConfiguration, mode.ShapeColorScheme, GameBindings.Shapes, GameBindings.ShapeIds);
        var resources = new GameResourcesMap(generator);
        // Construct a new system collection. Stateful systems must never be shared with the host map.
        var constructor = typeof(BuiltinSimulationSystems).GetConstructors().Single();
        var values = constructor.GetParameters().Select(p => p.Name switch
        {
            "initialSimulationTime" => (object)Ticks.Zero,
            "shapeOperations" => GameBindings.Field<object>(session, "ShapeOperations"),
            "mode" => mode,
            "shapeRegistry" => GameBindings.Field<object>(session, "ShapeRegistry"),
            "shapeIdManager" => GameBindings.Field<object>(session, "ShapeIdManager"),
            "resourcesMap" => resources,
            "fluidRegistry" => GameBindings.Field<object>(session, "FluidRegistry"),
            "packageItemSolver" => GameBindings.Field<object>(session, "FluidPackageItemSolver"),
            "signalChannelRegistry" => new SignalChannelRegistry(Enumerable.Empty<(SignalChannelId, IReadOnlySignalChannel)>()),
            "researchUnlockManager" => GameBindings.Field<ResearchManager>(session, "Research").UnlockManager,
            "logger" => logger,
            _ => throw new InvalidOperationException("Unknown dependency: " + p.Name)
        }).ToArray();
        systems = ((BuiltinSimulationSystems)constructor.Invoke(values)).CreateSimulationSystems().ToArray();
        layout = new MapLayout(logger);
        map = new Map(layout, resources, logger);
        graph = new SimulationGraph(Ticks.Zero, false, logger);
        simulator = new Simulator(graph, systems, layout, logger);
        model = new MapModel(map, simulator, new MapLayers(mode.MaxBuildingLayer, mode.MinIslandLayer, mode.MaxIslandLayer), logger);
        strategy = new ProcessingUpdateStrategy(SimulationConstants.MaxSimulationUpdateDelta, SimulationConstants.MaxSimulatorIncrement, new[] { Ticks.Zero });
        importer = LabBootstrap.Importer ?? deps.Resolve<IBlueprintImporter>();
        converter = new BlueprintRawDataConverter(((GameSessionOrchestrator)session).DataSerializers, logger);
    }
    public Simulator Simulator => simulator;
    public void Advance(int seconds, int ticksPerSecond, Action before = null, Action after = null, Func<bool> stop = null)
    {
        if (seconds < 0 || ticksPerSecond <= 0)
            throw new ArgumentException("Invalid simulation duration or step");
        var lods = model.Islands.SelectMany(i => i.Definition.Layout.GetChunkPositions().Select(c => SimulationLOD.Highest(c.ToGlobal(i.Transform)))).ToArray();
        var update = new SimulationUpdateConfiguration(lods, TimeSpan.Zero, 1);
        var delta = Ticks.OneSecond / ticksPerSecond;
        for (int i = 0; i < checked(seconds * ticksPerSecond); i++)
        {
            before?.Invoke();
            simulator.SynchronousUpdate(delta, update, strategy);
            after?.Invoke();
            if (stop?.Invoke() == true)
                break;
        }
    }
    public object Run(JObject args, string root)
    {
        var experiment = BlueprintExperiment.Load(args["experiment"] as JObject);
        experiment?.Configure(args);
        var path = (string)args["blueprint"];
        if (string.IsNullOrEmpty(path))
            throw new ArgumentException("blueprint path required");
        if (!importer.TryImport(File.ReadAllText(path), out var annotated, out var version, out var error))
            throw error;
        if (!(annotated.Blueprint is IslandBlueprint blueprint))
            throw new ArgumentException("Island blueprint required");
        var originalCount = blueprint.BuildingCount;
        var rateManifest = args["rate_manifest"] == null ? null : JObject.Parse(File.ReadAllText((string)args["rate_manifest"]));
        var rateProbe = rateManifest == null ? null : new RateProbe(rateManifest);
        using (layout.StartBunchEdit())
        {
            foreach (var entry in blueprint.Entries)
            {
                var transform = new GlobalChunkTransform(new GlobalChunkCoordinate(entry.Chunk_L.x, entry.Chunk_L.y, entry.Chunk_L.z), entry.Rotation);
                entry.Definition.TryCreateConfiguration(out var config);
                if (entry.Configuration != null && config != null)
                    converter.DeserializeByteArrayIntoData(entry.Configuration, GameVersionEnvironment.CurrentVersion, config.Sync);
                model.CreateIsland(entry.Definition, transform, config);
            }
            foreach (var entry in blueprint.Entries)
            {
                var transform = new GlobalChunkTransform(new GlobalChunkCoordinate(entry.Chunk_L.x, entry.Chunk_L.y, entry.Chunk_L.z), entry.Rotation);
                if (entry.BuildingBlueprint == null)
                    continue;
                foreach (var building in entry.BuildingBlueprint.Entries)
                {
                    building.Definition.TryCreateConfiguration(out var config);
                    if (building.AdditionalConfigData != null && config != null)
                        converter.DeserializeByteArrayIntoData(building.AdditionalConfigData, GameVersionEnvironment.CurrentVersion, config.Sync);
                    var t = new LocalTileTransform(building.Tile_L, building.Rotation).ToGlobal(new GlobalChunkTransform(transform.Position, default(GridRotation)));
                    if (rateProbe != null && rateProbe.CutIslands.Contains((entry.Chunk_L.x, entry.Chunk_L.y, entry.Chunk_L.z)) && !rateProbe.Targets.ContainsKey((t.Position.x, t.Position.y, t.Position.z)))
                        continue;
                    if (experiment != null && !experiment.KeepBuilding(t.Position))
                        continue;
                    model.CreateBuilding(building.Definition, t, config);
                }
            }
        }
        if (args["expected_buildings"] != null && (int)args["expected_buildings"] != model.BuildingCount)
            throw new Exception("Building count mismatch");
        var seconds = (int?)args["seconds"] ?? 1;
        object probeReport = null;
        if (experiment != null)
            probeReport = experiment.Run(this, args, root);
        else if (rateProbe != null)
            probeReport = RateScenario.Run(this, rateProbe, rateManifest, args, root);
        else
            Advance(seconds, 60);
        var export = (bool?)args["export_graph"] != false ? GraphExporter.Write(simulator, args, root) : (((ISimulator)simulator).Simulations.Count(), new List<object>(), (string)null);
        return new
        {
            islands = model.IslandCount,
            buildings = model.BuildingCount,
            imported_buildings = originalCount,
            simulated_seconds = simulator.SimulationTime.FloatSeconds,
            simulations = export.Item1,
            open_connectors = export.Item2,
            graph = export.Item3,
            probe = probeReport
        };
    }
    public void Dispose()
    {
        model.Dispose();
        simulator.Dispose();
        foreach (var s in systems)
        if (s is IDisposable disposable)
            disposable.Dispose();
        graph.Dispose();
        map.Dispose();
        layout.Dispose();
    }
}

