using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

/// Exercise real ports, circuitry, belts, and backpressure in a disposable native world.
public sealed class PriorityExperiment : IBlueprintExperiment
{
    public void Configure(JObject args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;

    private sealed class Source
    {
        internal readonly IItemReceiver Lane;
        internal readonly IBeltItem Item;
        internal double Credit;
        internal Source(IItemReceiver lane, string shape)
        {
            Lane = lane;
            Item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve(shape));
        }
        internal void Feed(double delta, double rate)
        {
            Credit = Math.Min(1, Credit + delta * rate);
            if (rate > 0 && Credit >= 1 && Lane.CanAcceptItem(Item))
            {
                Lane.HandOverItem(Item, Ticks.Zero);
                Credit -= 1;
            }
        }
    }
    private sealed class Meter : IItemReceiver
    {
        private readonly string primary;
        private readonly string secondary;
        internal Meter(int lane)
        {
            primary = Shape(lane, true);
            secondary = Shape(lane, false);
        }
        internal readonly Dictionary<string, long> Counts = new Dictionary<string, long>();
        internal bool Blocked;
        public Steps MaxStep_S => LaneConstants.ItemSpacing;
        public bool CanAcceptItem(IBeltItem item) => !Blocked;
        public void HandOverItem(IBeltItem item, Ticks remainingTicks)
        {
            var key = item.ToString();
            if (key != primary && key != secondary)
                throw new InvalidOperationException("Crossed output lane: " + key);
            Counts.TryGetValue(key, out var count);
            Counts[key] = count + 1;
        }
    }

    private static string Shape(int lane, bool primary)
    {
        char part = primary ? 'C' : 'R';
        return $"{part}{"rgbcmyw"[lane % 7]}{part}{"rgbcmyw"[lane / 7]}{part}u{part}u";
    }

    public object Run(LabWorld world, JObject args, string root)
    {
        var a = new Dictionary<int, Source>();
        var b = new Dictionary<int, Source>();
        var sinks = new Dictionary<int, Meter>();
        foreach (var localized in ((ISimulator)world.Simulator).Simulations)
        {
            if (!(localized is ILocalizedTileSimulation tile) || !(localized.Simulation is IItemSimulation belt))
                continue;
            var p = tile.GetOccupiedTile(0);
            if (p.x == 2 && p.y >= 8 && p.y <= 11)
                a.Add(p.z * 4 + p.y - 8, new Source(((SpaceBeltPortReceiverSimulation)belt).InputLane, Shape(p.z * 4 + p.y - 8, true)));
            if (p.y == 2 && p.x >= 8 && p.x <= 11)
                b.Add(p.z * 4 + p.x - 8, new Source(((SpaceBeltPortReceiverSimulation)belt).InputLane, Shape(p.z * 4 + p.x - 8, false)));
            if (p.x == 17 && p.y >= 8 && p.y <= 11)
            {
                var meter = new Meter(p.z * 4 + p.y - 8);
                ((SpaceBeltPortSenderSimulation)belt).PathLane.NextLane = meter;
                sinks.Add(p.z * 4 + p.y - 8, meter);
            }
        }
        if (a.Count != 12 || b.Count != 12 || sinks.Count != 12)
            throw new InvalidOperationException($"Port counts: A={a.Count}, B={b.Count}, output={sinks.Count}; " + string.Join(";", ((ISimulator)world.Simulator).Simulations.Take(12).Select(s => s.Simulation.GetType().Name)));
        var speed = GameBindings.Mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        double rate = (speed.StepsPerTick * Ticks.OneSecond).Value / (double)LaneConstants.ItemSpacing.Value;
        var reports = new List<object>();
        foreach (var phase in new[] { ("full", 1.0, 1.0), ("half", 0.5, 1.0), ("A-only", 1.0, 0.0), ("B-only", 0.0, 1.0), ("restart-A", 1.0, 1.0), ("mixed", -1.0, 1.0) })
        {
            void Feed(Ticks start, Ticks delta)
            {
                foreach (var pair in a)
                    pair.Value.Feed(delta.FloatSeconds, rate * (phase.Item2 < 0 ? (pair.Key % 4) / 3.0 : phase.Item2));
                foreach (var source in b.Values)
                    source.Feed(delta.FloatSeconds, rate * phase.Item3);
            }
            world.Advance(30, 120, Feed);
            foreach (var sink in sinks.Values)
                sink.Counts.Clear();
            world.Advance(120, 120, Feed);
            foreach (var pair in sinks)
            {
                double fraction = phase.Item2 < 0 ? (pair.Key % 4) / 3.0 : phase.Item2;
                pair.Value.Counts.TryGetValue(Shape(pair.Key, true), out var primary);
                pair.Value.Counts.TryGetValue(Shape(pair.Key, false), out var secondary);
                if (Math.Abs(primary - 120 * rate * fraction) > 2 || primary + secondary < 120 * rate * 0.995 ||
                    fraction == 1 && secondary != 0)
                    throw new InvalidOperationException($"{phase.Item1} lane {pair.Key}: A={primary}, B={secondary}");
            }
            reports.Add(new
            {
                phase = phase.Item1,
                lanes = sinks.OrderBy(p => p.Key).Select(p => new { lane = p.Key, counts = new Dictionary<string, long>(p.Value.Counts), utilization = p.Value.Counts.Values.Sum() / (120 * rate) }).ToArray()
            });
        }
        // Retain queued goods while the downstream port is blocked, then recover.
        foreach (var sink in sinks.Values)
            sink.Blocked = true;
        world.Advance(15, 120, (_, delta) => { foreach (var s in a.Values.Concat(b.Values)) s.Feed(delta.FloatSeconds, rate); });
        foreach (var sink in sinks.Values)
            sink.Blocked = false;
        world.Advance(30, 120, (_, delta) => { foreach (var s in a.Values.Concat(b.Values)) s.Feed(delta.FloatSeconds, rate); });
        foreach (var sink in sinks.Values)
            sink.Counts.Clear();
        world.Advance(120, 120, (_, delta) => { foreach (var s in a.Values.Concat(b.Values)) s.Feed(delta.FloatSeconds, rate); });
        foreach (var pair in sinks)
        {
            pair.Value.Counts.TryGetValue(Shape(pair.Key, true), out var primary);
            pair.Value.Counts.TryGetValue(Shape(pair.Key, false), out var secondary);
            if (primary < 120 * rate * 0.995 || secondary != 0)
                throw new InvalidOperationException($"Blocked-output recovery failed at lane {pair.Key}");
        }
        var report = new
        {
            verified = true,
            blocked_output_recovery = true,
            belt_rate = rate,
            phases = reports
        };
        File.WriteAllText(Path.Combine(root, "priority-measurements.json"), JsonConvert.SerializeObject(report, Formatting.Indented));
        return report;
    }
}
