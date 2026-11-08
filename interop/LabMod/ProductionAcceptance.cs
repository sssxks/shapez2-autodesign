using System;
using System.Collections.Generic;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;

/// Nominal-rate production and bounded recovery through the same continuously running world.
public static class ProductionAcceptance
{
    public static ExperimentResult Run(LabWorld world, IReadOnlyList<BeltSource> sources,
        IReadOnlyList<(string Name, string Group, BeltMeter Meter, string Shape)> outputs,
        bool independentOverflow = false, IReadOnlyList<double>? inputRates = null)
    {
        if (sources.Count == 0 || outputs.Count == 0)
            throw new ArgumentException("Production ports required");
        var speed = GameBindings.Mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        double rate = (speed.StepsPerTick * Ticks.OneSecond).Value / LaneConstants.ItemSpacing.Value;
        var rates = inputRates ?? sources.Select(_ => rate).ToArray();
        if (rates.Count != sources.Count || rates.Any(r => double.IsNaN(r) || r <= 0 || r > rate))
            throw new ArgumentException("Every input requires a positive rate no greater than belt capacity");
        bool supply = true;
        void Feed(Ticks t, Ticks dt)
        {
            for (int i = 0; i < sources.Count; i++)
                sources[i].Feed(dt.FloatSeconds, supply ? rates[i] : 0);
        }
        void Clear()
        {
            foreach (var output in outputs)
                output.Meter.Counts.Clear();
        }
        bool Meets((string Name, string Group, BeltMeter Meter, string Shape) output, int seconds)
        {
            if (output.Meter.Counts.Keys.Any(shape => shape != output.Shape))
                throw new InvalidOperationException("Wrong output shape at " + output.Name + ": expected " + output.Shape +
                    ", actual " + Newtonsoft.Json.JsonConvert.SerializeObject(output.Meter.Counts));
            long count = output.Meter.Counts.Values.Sum();
            return output.Meter.Blocked ? count == 0 : Math.Abs(count - seconds * rate) <= 1;
        }
        object Snapshot(string phase, int seconds) => new
        {
            phase,
            seconds,
            lanes = outputs.Select(o => new
            {
                port = o.Name,
                blocked = o.Meter.Blocked,
                expected = o.Shape,
                count = o.Meter.Counts.Values.Sum()
            }).ToArray()
        };
        var phases = new List<object>();
        void Steady(string phase)
        {
            Clear();
            var before = sources.Select(s => s.AcceptedCount).ToArray();
            world.Advance(120, 120, Feed);
            var snapshot = new
            {
                output = Snapshot(phase, 120),
                inputs = sources.Select((s, i) =>
                new
                {
                    lane = i,
                    expected = 120 * rates[i],
                    count = s.AcceptedCount - before[i]
                }).ToArray()
            };
            if (outputs.Any(o => !Meets(o, 120)) || sources.Where((s, i) => Math.Abs(s.AcceptedCount - before[i] - 120 * rates[i]) > 1).Any())
                throw new InvalidOperationException("Nominal throughput failed: " + Newtonsoft.Json.JsonConvert.SerializeObject(snapshot));
            phases.Add(snapshot);
        }
        void Recover(string phase)
        {
            var samples = new List<object>();
            int stable = 0;
            for (int elapsed = 10; elapsed <= 120; elapsed += 10)
            {
                Clear();
                world.Advance(10, 120, Feed);
                samples.Add(Snapshot(phase, 10));
                stable = outputs.All(o => Meets(o, 10)) ? stable + 1 : 0;
                if (stable == 3)
                {
                    phases.Add(new
                    {
                        phase,
                        recovery_seconds = elapsed,
                        samples
                    });
                    Steady(phase + "-sustained");
                    return;
                }
            }
            throw new InvalidOperationException("Recovery deadline exceeded: " + Newtonsoft.Json.JsonConvert.SerializeObject(new
            {
                phase,
                samples
            }));
        }
        world.Advance(120, 120, Feed);
        Steady("nominal");
        supply = false;
        world.Advance(120, 120, Feed);
        Clear();
        world.Advance(30, 120, Feed);
        if (outputs.Any(o => o.Meter.Counts.Values.Sum() != 0))
            throw new InvalidOperationException("Factory did not drain after supply stopped");
        supply = true;
        Recover("supply-restart");
        var groups = outputs.Select(o => o.Group).Distinct().ToArray();
        foreach (var blocked in new[] { "all", "alternating" }.Concat(groups.Length > 1 ? groups : Array.Empty<string>()))
        {
            for (int i = 0; i < outputs.Count; i++)
                outputs[i].Meter.Blocked = blocked == "all" || (blocked == "alternating" ? i % 2 == 0 : outputs[i].Group == blocked);
            world.Advance(120, 120, Feed);
            Clear();
            world.Advance(30, 120, Feed);
            if (outputs.Any(o => o.Meter.Blocked && o.Meter.Counts.Values.Sum() != 0))
                throw new InvalidOperationException("Backpressure violation");
            if (independentOverflow && outputs.Any(o => !Meets(o, 30)))
                throw new InvalidOperationException("Blocked outputs starved an independent overflow output");
            phases.Add(Snapshot("blocked-" + blocked, 30));
            foreach (var output in outputs)
                output.Meter.Blocked = false;
            Recover("release-" + blocked);
        }
        return ExperimentResult.Accepted(new
        {

            belt_rate = rate,
            input_lanes = sources.Count,
            output_lanes = outputs.Count,
            recovery_deadline_seconds = 120,
            stable_recovery_windows = 3,
            phases
        });
    }
}
