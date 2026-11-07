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
    public void Configure(JObject args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;

    public object Run(LabWorld world, JObject args, string root)
    {
        var cutter = ((ISimulator)world.Simulator).Simulations.Select(s => s.Simulation).OfType<FullCutterSimulation>().First();
        var left = new BeltMeter();
        var output = ((ISimulator)world.Simulator).Simulations.OfType<ILocalizedTileSimulation>().Single(s => s.GetOccupiedTile(0).x == 12 && s.GetOccupiedTile(0).y == 9);
        ((MergerSimulation)output.Simulation).OutputLane.NextLane = left;
        var item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve("CuCuCuCu"));
        void Feed(Ticks start, Ticks delta)
        {
            if (cutter.InputLane.CanAcceptItem(item))
                cutter.InputLane.HandOverItem(item, Ticks.Zero);
        }
        BeltMeasurement.Run(world, 30, 120, new[] { left }, Feed);
        if (left.Counts.Count != 1 || !left.Counts.TryGetValue("Cu------", out var count) || count != 240)
            throw new InvalidOperationException("Quarter cell did not produce 240 normalized quarters");
        return new
        {
            verified = true,
            output = left.Counts
        };
    }
}



