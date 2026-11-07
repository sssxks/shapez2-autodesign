using System;
using System.Collections.Generic;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json.Linq;
/// Verify all physical input/output ports against the native simulation.
public sealed class DiagonalExperiment : IBlueprintExperiment
{
    private BeltPortManifest ports = null!;
    public void Configure(JObject args) => ports = BeltPortManifest.Parse(args);
    public bool KeepBuilding(GlobalTileCoordinate p) => true;

    public object Run(LabWorld w, JObject args, string root)
    {
        var sources = new List<IItemReceiver>();
        var sinks = new List<BeltMeter>();
        var bound = ports.Bind(w);
        sources.AddRange(bound.Inputs.Select(p => p.Receiver));
        foreach (var port in bound.Outputs)
        {
            var meter = new BeltMeter();
            port.Sender.PathLane.NextLane = meter;
            sinks.Add(meter);
        }
        if (sources.Count != 12 || sinks.Count != 24)
            throw new InvalidOperationException($"ports {sources.Count}/{sinks.Count}");
        var item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve("CuCuCuCu"));
        void Feed(Ticks t, Ticks dt)
        {
            foreach (var s in sources)
            if (s.CanAcceptItem(item))
                s.HandOverItem(item, Ticks.Zero);
        }
        BeltMeasurement.Run(w, 120, 120, sinks, Feed);
        string expected = args.Value<string>("name") == "NE-SW" ? "Cu--Cu--" : "--Cu--Cu";
        var speed = GameBindings.Mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        double rate = (speed.StepsPerTick * Ticks.OneSecond).Value / (double)LaneConstants.ItemSpacing.Value;
        var counts = sinks.Select(s => s.Counts).ToArray();
        foreach (var s in sinks)
        if (s.Counts.Count != 1 || !s.Counts.TryGetValue(expected, out var n) || n < 120 * rate * 0.99)
            throw new InvalidOperationException(Newtonsoft.Json.JsonConvert.SerializeObject(new
            {
                expected,
                rate,
                counts
            }));
        return new
        {
            expected,
            rate,
            counts,
            verified = true
        };
    }
}


