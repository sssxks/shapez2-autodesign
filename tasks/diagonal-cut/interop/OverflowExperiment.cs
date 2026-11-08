using System;
using System.Collections.Generic;
using System.Linq;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json.Linq;
/// Full belt supply, side-specific shapes, overflow isolation and recovery.
public sealed class OverflowExperiment : IBlueprintExperiment
{
    public void Configure(JObject args) { }
    public bool KeepBuilding(GlobalTileCoordinate p) => true;
    public object Run(LabWorld w, JObject args, string root)
    {
        var sources = new List<BeltSource>();
        var sinks = new List<(int X, BeltMeter Meter)>();
        foreach (var s in ((ISimulator)w.Simulator).Simulations)
        {
            if (!(s is ILocalizedTileSimulation tile)) continue;
            var p = tile.GetOccupiedTile(0);
            if (p.y == 2 && p.x >= 8 && p.x <= 11)
                sources.Add(new BeltSource(((SpaceBeltPortReceiverSimulation)s.Simulation).InputLane, "CgCbCgCb"));
            if (p.y == 17 && ((p.x >= 8 && p.x <= 11) || (p.x >= 28 && p.x <= 31)))
            {
                var meter = new BeltMeter(p.x < 20 ? "Cg--Cg--" : "--Cb--Cb");
                ((SpaceBeltPortSenderSimulation)s.Simulation).PathLane.NextLane = meter;
                sinks.Add((p.x, meter));
            }
        }
        if (sources.Count != 12 || sinks.Count != 24)
            throw new InvalidOperationException($"ports {sources.Count}/{sinks.Count}");
        var speed = GameBindings.Mode.Buildings.ForwardBelt.ConfigAs<IConveyorConfiguration>().ConveyorSpeed;
        double rate = (speed.StepsPerTick * Ticks.OneSecond).Value / (double)LaneConstants.ItemSpacing.Value;
        void Feed(Ticks t, Ticks dt)
        {
            foreach (var source in sources) source.Feed(dt.Value / (double)Ticks.OneSecond.Value, rate);
        }
        var phases = new List<object>();
        foreach (var phase in new[] { "full", "left-blocked", "right-blocked", "recovered" })
        {
            foreach (var sink in sinks)
                sink.Meter.Blocked = phase == "left-blocked" && sink.X < 20 || phase == "right-blocked" && sink.X >= 20;
            BeltMeasurement.Run(w, 120, 120, sinks.Select(s => s.Meter), Feed);
            var counts = sinks.Select(s => new { x = s.X, blocked = s.Meter.Blocked, count = s.Meter.Counts.Values.Sum() }).ToArray();
            if (counts.Any(s => s.blocked ? s.count != 0 : s.count < 120 * rate * 0.99 || s.count > 120 * rate * 1.01))
                throw new InvalidOperationException(Newtonsoft.Json.JsonConvert.SerializeObject(new { phase, rate, counts }));
            phases.Add(new { phase, counts });
        }
        return new { rate, phases, verified = true };
    }
}
