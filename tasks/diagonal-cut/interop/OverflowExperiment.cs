using System;
using System.Collections.Generic;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json.Linq;
/// Verify all physical input/output ports against the native simulation.
public sealed class OverflowExperiment : IBlueprintExperiment
{
    public void Configure(JObject args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate p) => true;

    public object Run(LabWorld w, JObject args, string root)
    {
        var sources = new List<IItemReceiver>();
        var sinks = new List<BeltMeter>();
        foreach (var s in ((ISimulator)w.Simulator).Simulations)
        {
            if (!(s is ILocalizedTileSimulation tile))
                continue;
            var p = tile.GetOccupiedTile(0);
            if (p.y == 2 && p.x >= 8 && p.x <= 11)
                sources.Add(((SpaceBeltPortReceiverSimulation)s.Simulation).InputLane);
            if (p.y == 17 && ((p.x >= 8 && p.x <= 11) || (p.x >= 28 && p.x <= 31)))
            {
                var m = new BeltMeter();
                ((SpaceBeltPortSenderSimulation)s.Simulation).PathLane.NextLane = m;
                sinks.Add(m);
            }
        }
        if (sources.Count != 12 || sinks.Count != 24)
            throw new InvalidOperationException($"ports {sources.Count}/{sinks.Count}");
        var item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve("CgCbCgCb"));
        void Feed(Ticks t, Ticks dt)
        {
            foreach (var s in sources)
                if (s.CanAcceptItem(item))
                    s.HandOverItem(item, Ticks.Zero);
        }
        BeltMeasurement.Run(w, 120, 120, sinks, Feed);
        string expected = "Cg--Cg-- or --Cb--Cb";
        var speed = GameBindings.Mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        double rate = (speed.StepsPerTick * Ticks.OneSecond).Value / (double)LaneConstants.ItemSpacing.Value;
        var counts = sinks.Select(s => s.Counts).ToArray();
        foreach (var s in sinks)
            if (s.Counts.Count != 1 || !(s.Counts.TryGetValue("Cg--Cg--", out var n) || s.Counts.TryGetValue("--Cb--Cb", out n)) || n < 120 * rate * 0.49)
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




