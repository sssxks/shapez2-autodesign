using System;
using System.Collections.Generic;
using Game.Content.BuildingPath.Simulation;
using Game.Core.Coordinates;
using Game.Core.Map.Simulation;
using Newtonsoft.Json.Linq;
/// Every physical lane, nominal supply, independent overflow and bounded recovery.
public sealed class OverflowExperiment : IBlueprintExperiment
{
    public void Configure(JObject args)
    {
    }
    public bool KeepBuilding(GlobalTileCoordinate p) => true;
    public object Run(LabWorld world, JObject args, string root)
    {
        var sources = new List<BeltSource>();
        var outputs = new List<(string, string, BeltMeter, string)>();
        foreach (var simulation in ((ISimulator)world.Simulator).Simulations)
        {
            if (!(simulation is ILocalizedTileSimulation tile))
                continue;
            var p = tile.GetOccupiedTile(0);
            if (p.y == 2 && p.x >= 8 && p.x <= 11 && simulation.Simulation is SpaceBeltPortReceiverSimulation receiver)
                sources.Add(new BeltSource(receiver.InputLane, "CgCbCgCb"));
            if (p.y == 17 && (p.x >= 8 && p.x <= 11 || p.x >= 28 && p.x <= 31) && simulation.Simulation is SpaceBeltPortSenderSimulation sender)
            {
                string expected = p.x < 20 ? "Cg--Cg--" : "--Cb--Cb";
                var meter = new BeltMeter(expected);
                sender.PathLane.NextLane = meter;
                outputs.Add((p.ToString(), p.x < 20 ? "left" : "right", meter, expected));
            }
        }
        if (sources.Count != 12 || outputs.Count != 24)
            throw new InvalidOperationException("Expected 12 inputs and 24 outputs");
        return ProductionAcceptance.Run(world, sources, outputs, true);
    }
}
