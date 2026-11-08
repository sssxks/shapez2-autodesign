using System;
using System.IO;
using Game.Core.Coordinates;
using Game.Core.Simulation;
using Newtonsoft.Json;

public sealed class CmamExperiment : IBlueprintExperiment
{
    private CmamOptions? options;
    // Import policy is configured before buildings are created.
    public bool KeepBuilding(GlobalTileCoordinate p) => !Options.Adapter || AdapterProbe.KeepBuilding(p);
    private CmamOptions Options => options ?? throw new InvalidOperationException("Configure the experiment before use");
    public ExperimentResult Run(LabWorld world, LabArguments args, string root)
    {
        var configured = Options;
        if (configured.Production != null)
        {
            var production = new ProductionProbe(world.Simulator, configured.Production);
            world.Advance(configured.Seconds, 10, production.BeforeTick);
            var report = production.Report();
            File.WriteAllText(Path.Combine(root, configured.Name + "-production.json"), JsonConvert.SerializeObject(report, Formatting.Indented));
            if (configured.MinimumProduced is long minimum && production.Produced < minimum)
                throw new InvalidOperationException("Production below required count: " + production.Produced);
            return ExperimentResult.Observed(report);
        }
        var probe = new AdapterProbe(world.Simulator);
        bool complete = false;
        world.Advance(configured.Seconds, 10, (_, _) => probe.BeforeTick(), (_, _) => complete = probe.AfterTick(), () => complete);
        var flow = probe.Report();
        File.WriteAllText(Path.Combine(root, "adapter-flow.json"), JsonConvert.SerializeObject(flow));
        if (!complete)
            throw new Exception("Adapter flow did not reach every inlet: " + JsonConvert.SerializeObject(flow));
        return ExperimentResult.Observed(flow);
    }
    public CmamExperiment()
    {
    }
    public void Configure(LabArguments args)
    {
        options = new CmamOptions(args);
    }
}
