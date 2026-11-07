using System;
using System.IO;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

/// Measure saturated rotators independently before sharing them between cutting lanes.
public sealed class QuarterCapacityExperiment : IBlueprintExperiment
{
    public void Configure(JObject args) { }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;

    public object Run(LabWorld world, JObject args, string root)
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
        return report;
    }
}
