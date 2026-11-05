using System;
using System.Collections.Generic;
using System.Linq;
using Core.Logging;
using Game.Blueprints;
using Game.Core;
using Game.Core.Map.Simulation;
using Game.Core.Map.Simulation.Clustering;
using Game.Core.Simulation;

public sealed class LabWorld : IDisposable
{
    private readonly List<IDisposable> resources = new List<IDisposable>();
    private readonly MapLayout layout;
    private readonly MapModel model;
    private readonly Simulator simulator;
    private bool disposed;

    public LabWorld()
    {
        var logger = GameBindings.Field<ILogger>(GameBindings.Session, "Logger");
        var mode = GameBindings.Mode;
        try
        {
            var generator = new DefaultMapGenerator(mode.Seed, mode.MapGenerationParameters,
                mode.Scenario.StartingLocation, mode.ShapesConfiguration, mode.ShapeColorScheme,
                GameBindings.Shapes, GameBindings.ShapeIds);
            var resourceMap = new GameResourcesMap(generator);
            layout = Own(new MapLayout(logger));
            var map = Own(new Map(layout, resourceMap, logger));
            var graph = Own(new SimulationGraph(Ticks.Zero, false, logger));
            var systems = GameBindings.CreateSimulationSystems(resourceMap, logger).Select(system =>
            {
                if (system is IDisposable resource)
                    resources.Add(resource);
                return system;
            }).ToArray();
            simulator = Own(new Simulator(graph, systems, layout, logger));
            model = Own(new MapModel(map, simulator,
                new MapLayers(mode.MaxBuildingLayer, mode.MinIslandLayer, mode.MaxIslandLayer), logger));
        }
        catch (Exception creationError)
        {
            try
            {
                Dispose();
            }
            catch (Exception cleanupError) { throw new AggregateException(creationError, cleanupError); }
            throw;
        }
    }

    private T Own<T>(T resource) where T : IDisposable
    {
        resources.Add(resource);
        return resource;
    }

    public Simulator Simulator => simulator;
    internal MapModel Model => model;
    internal IDisposable StartEdit() => layout.StartBunchEdit();

    public void Advance(int seconds, int ticksPerSecond, Action<Ticks, Ticks>? before = null,
        Action<Ticks, Ticks>? after = null, Func<bool>? stop = null) =>
        SimulationStepper.Advance(model, seconds, ticksPerSecond, before, after, stop);

    public void Dispose()
    {
        if (disposed)
            return;
        disposed = true;
        var errors = new List<Exception>();
        for (var i = resources.Count - 1; i >= 0; i--)
        {
            try
            {
                resources[i].Dispose();
            }
            catch (Exception error) { errors.Add(error); }
        }
        resources.Clear();
        if (errors.Count > 0)
            throw new AggregateException("Failed to dispose lab world", errors);
    }
}
