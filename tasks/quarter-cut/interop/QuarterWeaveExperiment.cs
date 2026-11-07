using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

/// Validate synthesized primitive networks with every merger socket connected.
public sealed class QuarterWeaveExperiment : IBlueprintExperiment
{
    public void Configure(JObject args) { }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;
    public object Run(LabWorld world, JObject args, string root)
    {
        var simulations = ((ISimulator)world.Simulator).Simulations.ToArray();
        int mergers = 0;
        foreach (var sim in simulations)
        {
            if (sim.Simulation.GetType().Name.Contains("Blocked") || sim.Simulation.GetType().Name.Contains("Disabled"))
                throw new InvalidOperationException("Disabled transport: " + sim.Simulation.GetType().Name);
            if (sim.Simulation is MergerSimulation && sim is IConnectableSimulation connectors)
            {
                for (int i = 0; i < connectors.NumConnectors; i++)
                    if (!world.Simulator.TryGetConnectedSimulation(sim, i, out _))
                        throw new InvalidOperationException("Unused merger socket: " + ((ILocalizedTileSimulation)sim).GetOccupiedTile(0));
                mergers++;
            }
        }
        IItemSimulation Find(JToken p) => (IItemSimulation)(simulations.OfType<ILocalizedTileSimulation>().SingleOrDefault(s => {
            for (int i = 0; i < s.NumOccupiedTiles; i++) {
                var t = s.GetOccupiedTile(i);
                if (t.x == (int)p[0]! && t.y == (int)p[1]! && t.z == (int)p[2]!) return true;
            }
            return false;
        }) ?? throw new InvalidOperationException("Missing test port " + p)).Simulation;
        var sources = args["inputs"]!.Select(p => new BeltSource(Find(p).GetItemReceiver(0), "CuCuCuCu")).ToArray();
        var meters = args["outputs"]!.Select(p => {
            var meter = new BeltMeter();
            Find(p).GetItemProvider(0).NextLane = meter;
            return meter;
        }).ToArray();
        var rates = args["input_rates"]?.Select(p => (double)p).ToArray() ?? sources.Select(_ => 0.5).ToArray();
        if (rates.Length != sources.Length) throw new InvalidOperationException("Input rate count does not match source count");
        void Feed(Ticks start, Ticks delta) { for (int i = 0; i < sources.Length; i++) sources[i].Feed(delta.FloatSeconds, rates[i]); }
        var phases = new List<object>();
        var outputFull = args["output_full"]?.Select(p => (bool)p).ToArray() ?? meters.Select(_ => false).ToArray();
        void Check(string expected)
        {
            for (int i = 0; i < meters.Length; i++) {
                var meter = meters[i];
                var shape = outputFull[i] ? string.Concat(Enumerable.Repeat(expected.Substring(0,2), 4)) : expected;
                if (meter.Counts.Count != 1 || !meter.Counts.TryGetValue(shape, out var n) || n < 239)
                    throw new InvalidOperationException("Weave output: " + JsonConvert.SerializeObject(meter.Counts));
            }
        }
        foreach (var shape in new[] { "CuCuCuCu", "RrRrRrRr" })
        {
            foreach (var source in sources) source.Item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve(shape));
            BeltMeasurement.Run(world, 120, 120, meters, Feed);
            var counts = meters.Select(m => new Dictionary<string,long>(m.Counts)).ToArray();
            phases.Add(new { shape, counts });
            File.WriteAllText(Path.Combine(root, "weave-measurements.json"), JsonConvert.SerializeObject(phases, Formatting.Indented));
            Check(shape.Substring(0,2) + "------");
        }
        foreach (var meter in meters) meter.Blocked = true;
        world.Advance(20, 120, Feed);
        foreach (var meter in meters) meter.Blocked = false;
        BeltMeasurement.Run(world, 120, 120, meters, Feed);
        Check("Rr------");
        return new { verified = true, phases, mergers, unused_merger_connectors = 0, blocked_output_recovery = true };
    }
}
