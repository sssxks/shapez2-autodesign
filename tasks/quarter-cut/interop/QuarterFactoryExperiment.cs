using Game.Content.BuildingPath.Simulation;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

/// End-to-end twelve-input, forty-eight-output production acceptance.
public sealed class QuarterFactoryExperiment : IBlueprintExperiment
{
    private BeltPortManifest ports = null!;
    public void Configure(JObject args) => ports = BeltPortManifest.Parse(args);
    public bool KeepBuilding(GlobalTileCoordinate position) => true;


    public object Run(LabWorld world, JObject args, string root)
    {
        int checkedMergers = 0;
        int twoInputMergers = 0, threeInputMergers = 0;
        foreach (var sim in ((ISimulator)world.Simulator).Simulations)
        {
            if (sim.Simulation.GetType().Name.Contains("Blocked") || sim.Simulation.GetType().Name.Contains("Disabled"))
                throw new InvalidOperationException("Disabled transport: " + sim.Simulation.GetType().Name);
            if (sim.Simulation is MergerSimulation merger && sim is IConnectableSimulation connectors)
            {
                if (merger.NumItemReceivers == 2) twoInputMergers++;
                else if (merger.NumItemReceivers == 3) threeInputMergers++;
                else throw new InvalidOperationException("Unexpected merger input count");
                for (int i = 0; i < connectors.NumConnectors; i++)
                    if (!world.Simulator.TryGetConnectedSimulation(sim, i, out _))
                        throw new InvalidOperationException("Unconnected merger socket at " + ((ILocalizedTileSimulation)sim).GetOccupiedTile(0));
                checkedMergers++;
            }
        }
        if (checkedMergers != args.Value<int>("expected_mergers"))
            throw new InvalidOperationException("Merger count: " + checkedMergers);
        var sources = new List<BeltSource>();
        var meters = new Dictionary<string, BeltMeter>();
        var bound = ports.Bind(world);
        foreach (var port in bound.Inputs)
            sources.Add(new BeltSource(port.Receiver, "CuCuCuCu"));
        foreach (var port in bound.Outputs)
        {
            var meter = new BeltMeter();
            port.Sender.PathLane.NextLane = meter;
            meters.Add(port.Position.ToString(), meter);
        }
        if (sources.Count != 12 || meters.Count != 48)
            throw new InvalidOperationException($"Port counts {sources.Count}/12, {meters.Count}/48");
        var speed = GameBindings.Mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        double rate = (speed.StepsPerTick * Ticks.OneSecond).Value / (double)LaneConstants.ItemSpacing.Value;
        void Feed(Ticks start, Ticks delta)
        {
            foreach (var s in sources)
                s.Feed(delta.FloatSeconds, rate);
        }
        var phases = new List<object>();
        foreach (var shape in new[] { "CuCuCuCu", "RrRrRrRr" })
        {
            foreach (var s in sources)
                s.Item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve(shape));
            BeltMeasurement.Run(world, 180, 120, meters.Values, Feed);
            var expected = shape.Substring(0, 2) + "------";
            var failures = new List<string>();
            foreach (var pair in meters)
            {
                pair.Value.Counts.TryGetValue(expected, out var n);
                if (n < 120 * rate * 0.995 || pair.Value.Counts.Keys.Any(k => k != expected))
                    failures.Add($"{pair.Key}: {JsonConvert.SerializeObject(pair.Value.Counts)}");
            }
            phases.Add(new
            {
                shape,
                expected,
                lanes = meters.Select(p => new { port = p.Key, counts = new Dictionary<string, long>(p.Value.Counts) }).ToArray()
            });
            File.WriteAllText(Path.Combine(root, "quarter-cut-measurements.json"), JsonConvert.SerializeObject(new
            {
                rate,
                phases,
                failures
            }, Formatting.Indented));
            if (failures.Count > 0)
                throw new InvalidOperationException(string.Join("; ", failures));
        }
        foreach (var m in meters.Values)
            m.Blocked = true;
        world.Advance(20, 120, Feed);
        foreach (var m in meters.Values)
            m.Blocked = false;
        world.Advance(120, 120, Feed);
        foreach (var m in meters.Values)
            m.Counts.Clear();
        world.Advance(120, 120, Feed);
        foreach (var pair in meters)
        {
            pair.Value.Counts.TryGetValue("Rr------", out var n);
            if (n < 120 * rate * 0.995)
                throw new InvalidOperationException($"Recovery {pair.Key}: {n}");
        }
        var report = new
        {
            verified = true,
            rate,
            input_lanes = sources.Count,
            output_lanes = meters.Count,
            blocked_output_recovery = true,
            connected_two_input_mergers = twoInputMergers,
            connected_three_input_mergers = threeInputMergers,
            unused_merger_connectors = 0,
            phases
        };
        File.WriteAllText(Path.Combine(root, "quarter-cut-measurements.json"), JsonConvert.SerializeObject(report, Formatting.Indented));
        return report;
    }
}








