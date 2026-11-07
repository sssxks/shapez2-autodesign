using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json.Linq;

/// Measure the native cutter continuously with both outputs drained.
public sealed class QuarterCellExperiment : IBlueprintExperiment
{
    public void Configure(JObject args) { }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;
    private sealed class Meter : IItemReceiver
    {
        internal readonly Dictionary<string, long> Counts = new Dictionary<string, long>();
        public Steps MaxStep_S => LaneConstants.ItemSpacing;
        public bool CanAcceptItem(IBeltItem item) => true;
        public void HandOverItem(IBeltItem item, Ticks remainingTicks)
        {
            var key = item.ToString();
            Counts.TryGetValue(key, out var n);
            Counts[key] = n + 1;
        }
    }
    public object Run(LabWorld world, JObject args, string root)
    {
        var cutter = ((ISimulator)world.Simulator).Simulations.Select(s => s.Simulation).OfType<FullCutterSimulation>().First();
        var left = new Meter(); var right = new Meter();
        var output = ((ISimulator)world.Simulator).Simulations.OfType<ILocalizedTileSimulation>().Single(s => s.GetOccupiedTile(0).x == 12 && s.GetOccupiedTile(0).y == 9);
        ((MergerSimulation)output.Simulation).OutputLane.NextLane = left;
        var item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve("CuCuCuCu"));
        void Feed(Ticks start, Ticks delta)
        {
            if (cutter.InputLane.CanAcceptItem(item)) cutter.InputLane.HandOverItem(item, Ticks.Zero);
        }
        world.Advance(30, 120, Feed);
        left.Counts.Clear(); right.Counts.Clear();
        world.Advance(120, 120, Feed);
        return new { left = left.Counts, right = right.Counts };
    }
}


