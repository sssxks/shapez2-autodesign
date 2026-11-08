using System;
using System.Collections.Generic;
using System.Linq;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;

/// Measure the native cutter continuously with both outputs drained.
public sealed class QuarterCellExperiment : IBlueprintExperiment
{
    public void Configure(LabArguments args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate position) => true;

    public ExperimentResult Run(LabWorld world, LabArguments args, string root)
    {
        var inputY = args.Mirrored == true ? 7 : 8;
        var cutter = (FullCutterSimulation)((ISimulator)world.Simulator).Simulations.OfType<ILocalizedTileSimulation>()
            .Single(s => s.Simulation is FullCutterSimulation && s.GetOccupiedTile(0).x == 8 && s.GetOccupiedTile(0).y == inputY).Simulation;
        var left = new BeltMeter();
        var outputY = 6 + (args.OutputRow ?? (args.Mirrored == true ? 0 : 3));
        var output = ((ISimulator)world.Simulator).Simulations.OfType<ILocalizedTileSimulation>().Single(s => s.GetOccupiedTile(0).x == 12 && s.GetOccupiedTile(0).y == outputY);
        ((MergerSimulation)output.Simulation).OutputLane.NextLane = left;
        var item = GameBindings.Shapes.GetItem(GameBindings.ShapeIds.Resolve("CuCuCuCu"));
        IItemReceiver input = cutter.InputLane;
        if (args.JumpInput == true)
        {
            var source = ((ISimulator)world.Simulator).Simulations.OfType<ILocalizedTileSimulation>().Single(s => s.GetOccupiedTile(0).x == 3 && s.GetOccupiedTile(0).y == 8);
            input = ((IItemSimulation)source.Simulation).GetItemReceiver(0);
        }
        void Feed(Ticks start, Ticks delta)
        {
            if (input.CanAcceptItem(item))
                input.HandOverItem(item, Ticks.Zero);
        }
        BeltMeasurement.Run(world, 30, 120, new[] { left }, Feed);
        if (left.Counts.Count != 1 || !left.Counts.TryGetValue("Cu------", out var count) || count != 240)
            throw new InvalidOperationException("Quarter cell output: " + Newtonsoft.Json.JsonConvert.SerializeObject(left.Counts));
        return ExperimentResult.Accepted(new
        {

            output = left.Counts
        });
    }
}



