using System;
using System.Linq;
using Game.Core.Coordinates;
using Newtonsoft.Json.Linq;
/// Full-rate diagonal extraction with starvation and output-backpressure recovery.
public sealed class DiagonalExperiment : IBlueprintExperiment
{
    private BeltPortManifest ports = null!;
    public void Configure(JObject args) => ports = BeltPortManifest.Parse(args);
    public bool KeepBuilding(GlobalTileCoordinate p) => true;
    public object Run(LabWorld world, JObject args, string root)
    {
        var bound = ports.Bind(world);
        if (bound.Inputs.Count != 12 || bound.Outputs.Count != 24)
            throw new InvalidOperationException("Expected 12 inputs and 24 outputs");
        string expected = args.Value<string>("name") == "NE-SW" ? "Cu--Cu--" : "--Cu--Cu";
        var sources = bound.Inputs.Select(p => new BeltSource(p.Receiver, "CuCuCuCu")).ToArray();
        var outputs = bound.Outputs.Select(p =>
        {
            var meter = new BeltMeter(expected);
            p.Sender.PathLane.NextLane = meter;
            return (p.Position.ToString(), p.Role, meter, expected);
        }).ToArray();
        return ProductionAcceptance.Run(world, sources, outputs);
    }
}
