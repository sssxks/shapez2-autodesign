using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Map.Simulation;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

internal static class BlueprintRunner
{
    internal static object Run(LabWorld world, JObject args, string root)
    {
        var options = new BlueprintRunOptions(args);
        var experiment = BlueprintExperiment.Load(options.Experiment);
        experiment?.Configure(args);
        var blueprint = BlueprintLoader.Read(options.Path);
        var manifest = options.Rate?.Manifest;
        BlueprintLoader.Place(world, blueprint, (chunk, tile) =>
            (manifest == null || !manifest.CutIslands.Contains(chunk) ||
                manifest.Targets.ContainsKey(new TilePosition(tile.x, tile.y, tile.z))) &&
            (experiment == null || experiment.KeepBuilding(tile)));
        if (options.ExpectedBuildings is int expected && expected != world.Model.BuildingCount)
            throw new InvalidOperationException($"Building count mismatch: {world.Model.BuildingCount}/{expected}");

        object? probe = null;
        if (experiment != null)
            probe = experiment.Run(world, args, root);
        else if (options.Rate != null)
            probe = RateScenario.Run(world, options.Rate, root);
        else
            world.Advance(options.Seconds, 60);

        var export = options.ExportGraph
            ? GraphExporter.Write(world.Simulator, options.Name, root)
            : (((ISimulator)world.Simulator).Simulations.Count(), new List<object>(), (string?)null);
        return new BlueprintRunReport(world.Model.IslandCount, world.Model.BuildingCount,
            blueprint.BuildingCount, world.Simulator.SimulationTime.FloatSeconds,
            export.Item1, export.Item2, export.Item3, probe);
    }
}

internal sealed class BlueprintRunReport
{
    [JsonProperty("islands")]
    public int Islands
    {
        get;
    }
    [JsonProperty("buildings")]
    public int Buildings
    {
        get;
    }
    [JsonProperty("imported_buildings")]
    public int ImportedBuildings
    {
        get;
    }
    [JsonProperty("simulated_seconds")]
    public double SimulatedSeconds
    {
        get;
    }
    [JsonProperty("simulations")]
    public int Simulations
    {
        get;
    }
    [JsonProperty("open_connectors")]
    public IReadOnlyList<object> OpenConnectors
    {
        get;
    }
    [JsonProperty("graph")]
    public string? Graph
    {
        get;
    }
    [JsonProperty("probe")]
    public object? Probe
    {
        get;
    }

    internal BlueprintRunReport(int islands, int buildings, int importedBuildings, double seconds,
        int simulations, IReadOnlyList<object> openConnectors, string? graph, object? probe)
    {
        Islands = islands;
        Buildings = buildings;
        ImportedBuildings = importedBuildings;
        SimulatedSeconds = seconds;
        Simulations = simulations;
        OpenConnectors = openConnectors;
        Graph = graph;
        Probe = probe;
    }
}
