using System;
using System.IO;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json;

/// Measure saturated rotators independently before sharing them between cutting lanes.
public sealed class QuarterCapacityExperiment : IBlueprintExperiment
{
    public void Configure(LabArguments args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;

    public ExperimentResult Run(LabWorld world, LabArguments args, string root)
    {
        var rotators = ((ISimulator)world.Simulator).Simulations
            .Select(s => s.Simulation).OfType<RotatorSimulation>().ToArray();
        var meters = rotators.Select(_ => new BeltMeter()).ToArray();
        var item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve("Cu------"));
        for (int i = 0; i < rotators.Length; i++)
            rotators[i].OutputLane.NextLane = meters[i];
        void Feed(Ticks start, Ticks delta)
        {
            foreach (var rotator in rotators)
                if (rotator.InputLane.CanAcceptItem(item))
                    rotator.InputLane.HandOverItem(item, Ticks.Zero);
        }
        BeltMeasurement.Run(world, 30, 120, meters, Feed);
        var report = meters.Select(m => new { count = m.Counts.Values.Sum(), rate = m.Counts.Values.Sum() / 120.0, shapes = m.Counts }).ToArray();
        File.WriteAllText(Path.Combine(root, "rotator-capacity.json"), JsonConvert.SerializeObject(report, Formatting.Indented));
        if (report.Any(m => m.count == 0))
            throw new InvalidOperationException("Rotator capacity probe produced no output");
        var cutters = ((ISimulator)world.Simulator).Simulations.Select(s => s.Simulation).OfType<FullCutterSimulation>().ToArray();
        var cutMeters = cutters.SelectMany(_ => new[] { new BeltMeter(), new BeltMeter() }).ToArray();
        for (int i = 0; i < cutters.Length; i++)
        {
            cutters[i].LeftOutputLane.NextLane = cutMeters[2 * i];
            cutters[i].RightOutputLane.NextLane = cutMeters[2 * i + 1];
        }
        var full = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve("CuCuCuCu"));
        void FeedCutters(Ticks start, Ticks delta)
        {
            foreach (var cutter in cutters)
                if (cutter.InputLane.CanAcceptItem(full))
                    cutter.InputLane.HandOverItem(full, Ticks.Zero);
        }
        BeltMeasurement.Run(world, 30, 120, cutMeters, FeedCutters);
        var cutterReport = cutMeters.Select(m => new { count = m.Counts.Values.Sum(), rate = m.Counts.Values.Sum() / 120.0, shapes = m.Counts }).ToArray();
        File.WriteAllText(Path.Combine(root, "cutter-capacity.json"), JsonConvert.SerializeObject(cutterReport, Formatting.Indented));
        if (cutterReport.Any(m => m.count != 60))
            throw new InvalidOperationException("Native full-cutter rate changed; update the capacity model");
        return ExperimentResult.Observed(new
        {
            rotators = report,
            cutters = cutterReport
        });
    }
}
