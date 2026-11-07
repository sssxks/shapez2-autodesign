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
    public void Configure(JObject args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;
    private sealed class Source
    {
        internal readonly IItemReceiver Lane;
        internal IBeltItem Item;
        internal double Credit;
        internal Source(IItemReceiver lane, string shape)
        {
            Lane = lane;
            Item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve(shape));
        }
        internal void Feed(double delta, double rate)
        {
            Credit = Math.Min(1, Credit + delta * rate);
            if (Credit >= 1 && Lane.CanAcceptItem(Item))
            {
                Lane.HandOverItem(Item, Ticks.Zero);
                Credit -= 1;
            }
        }
    }
    private sealed class Meter : IItemReceiver
    {
        internal readonly Dictionary<string, long> Counts = new Dictionary<string, long>();
        internal bool Blocked;
        public Steps MaxStep_S => LaneConstants.ItemSpacing;
        public bool CanAcceptItem(IBeltItem item) => !Blocked;
        public void HandOverItem(IBeltItem item, Ticks remainingTicks)
        {
            var key = item.ToString();
            Counts.TryGetValue(key, out var count);
            Counts[key] = count + 1;
        }
    }
    public object Run(LabWorld world, JObject args, string root)
    {
        int checkedMergers = 0;
        foreach (var sim in ((ISimulator)world.Simulator).Simulations)
        {
            if (sim.Simulation.GetType().Name.Contains("Blocked") || sim.Simulation.GetType().Name.Contains("Disabled"))
                throw new InvalidOperationException("Disabled transport: " + sim.Simulation.GetType().Name);
            if (sim.Simulation is MergerSimulation merger && sim is IConnectableSimulation connectors)
            {
                if (merger.NumItemReceivers != 2)
                    throw new InvalidOperationException("Non-two-input merger");
                for (int i = 0; i < connectors.NumConnectors; i++)
                    if (!world.Simulator.TryGetConnectedSimulation(sim, i, out _))
                        throw new InvalidOperationException("Unconnected merger socket at " + ((ILocalizedTileSimulation)sim).GetOccupiedTile(0));
                checkedMergers++;
            }
        }
        if (checkedMergers != 144)
            throw new InvalidOperationException("Merger count: " + checkedMergers);
        var sources = new List<Source>();
        var meters = new Dictionary<string, Meter>();
        foreach (var localized in ((ISimulator)world.Simulator).Simulations)
        {
            if (!(localized is ILocalizedTileSimulation tile))
                continue;
            var p = tile.GetOccupiedTile(0);
            if (p.y == 2 && p.x >= 28 && p.x <= 31 && localized.Simulation is SpaceBeltPortReceiverSimulation receiver)
                sources.Add(new Source(receiver.InputLane, "CuCuCuCu"));
            if (localized.Simulation is SpaceBeltPortSenderSimulation sender &&
                ((p.y == 2 || p.y == 17) && (p.x >= 8 && p.x <= 11 || p.x >= 48 && p.x <= 51)))
            {
                var meter = new Meter();
                sender.PathLane.NextLane = meter;
                meters.Add($"{p.x}:{p.y}:{p.z}", meter);
            }
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
            world.Advance(180, 120, Feed);
            foreach (var m in meters.Values)
                m.Counts.Clear();
            world.Advance(120, 120, Feed);
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
            connected_two_input_mergers = checkedMergers,
            unused_merger_connectors = 0,
            phases
        };
        File.WriteAllText(Path.Combine(root, "quarter-cut-measurements.json"), JsonConvert.SerializeObject(report, Formatting.Indented));
        return report;
    }
}








