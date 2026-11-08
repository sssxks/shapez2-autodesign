using System;
using System.Collections.Generic;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Game.Core.Simulation;
using Newtonsoft.Json.Linq;
/// Fault injection exercises failure detection while real native belts remain in the loop.
public sealed class AcceptanceExperiment : IBlueprintExperiment
{
    public void Configure(JObject args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate p) => true;
    private sealed class StickyReceiver : IItemReceiver
    {
        private readonly BeltMeter meter; private bool latched;
        internal StickyReceiver(BeltMeter meter)
        {
            this.meter = meter;
        }
        public Steps MaxStep_S => LaneConstants.ItemSpacing;
        public bool CanAcceptItem(IBeltItem item)
        {
            latched |= meter.Blocked;
            return !latched;
        }
        public void HandOverItem(IBeltItem item, Ticks ticks) => meter.HandOverItem(item, ticks);
    }
    public object Run(LabWorld world, JObject args, string root)
    {
        string name = args.Value<string>("name")!;
        if (name == "unverified")
            return new
            {
                observed = true
            };
        var sources = new List<BeltSource>();
        var outputs = new List<(string, string, BeltMeter, string)>();
        foreach (var s in ((ISimulator)world.Simulator).Simulations)
        {
            if (!(s is ILocalizedTileSimulation tile))
                continue;
            var p = tile.GetOccupiedTile(0);
            if (p.y == 2 && s.Simulation is SpaceBeltPortReceiverSimulation receiver)
                sources.Add(new BeltSource(receiver.InputLane, name == "wrong-shape" ? "RrRrRrRr" : "CuCuCuCu"));
            if (p.y == 17 && s.Simulation is SpaceBeltPortSenderSimulation sender)
            {
                var meter = new BeltMeter();
                sender.PathLane.NextLane = name == "latched-output" ? new StickyReceiver(meter) : (IItemReceiver)meter;
                outputs.Add((p.ToString(), "output", meter, "CuCuCuCu"));
            }
        }
        if (sources.Count != 12 || outputs.Count != 12)
            throw new InvalidOperationException("Regression ports missing");
        return ProductionAcceptance.Run(world, sources, outputs);
    }
}
