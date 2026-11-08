using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json;

/// Exercise real ports, circuitry, belts, and backpressure in a disposable native world.
public sealed class PriorityExperiment : IBlueprintExperiment
{
    private BeltPortManifest ports = null!;
    public void Configure(LabArguments args) => ports = BeltPortManifest.Parse(args);
    public bool KeepBuilding(GlobalTileCoordinate position) => true;

    private static string Shape(int lane, bool primary)
    {
        char part = primary ? 'C' : 'R';
        return $"{part}{"rgbcmyw"[lane % 7]}{part}{"rgbcmyw"[lane / 7]}{part}u{part}u";
    }

    public ExperimentResult Run(LabWorld world, LabArguments args, string root)
    {
        var a = new Dictionary<int, BeltSource>();
        var b = new Dictionary<int, BeltSource>();
        var sinks = new Dictionary<int, BeltMeter>();
        var bound = ports.Bind(world);
        foreach (var port in bound.Inputs)
        {
            bool primary = port.Role == "primary";
            (primary ? a : b).Add(port.Lane, new BeltSource(port.Receiver, Shape(port.Lane, primary)));
        }
        foreach (var port in bound.Outputs)
        {
            var meter = new BeltMeter(Shape(port.Lane, true), Shape(port.Lane, false));
            port.Sender.PathLane.NextLane = meter;
            sinks.Add(port.Lane, meter);
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
            BeltMeasurement.Run(world, 30, 120, sinks.Values, Feed);
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
        var recovery = ProductionAcceptance.Run(world, a.OrderBy(p => p.Key).Select(p => p.Value).ToArray(),
            sinks.OrderBy(p => p.Key).Select(p => (p.Key.ToString(), "output", p.Value, Shape(p.Key, true))).ToArray());
        var report = new
        {
            verified = true,
            blocked_output_recovery = recovery,
            belt_rate = rate,
            phases = reports
        };
        File.WriteAllText(Path.Combine(root, args.Name + "-measurements.json"), JsonConvert.SerializeObject(report, Formatting.Indented));
        return ExperimentResult.Accepted(report);
    }
}

