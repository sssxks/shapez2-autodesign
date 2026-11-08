using System;
using System.IO;
using Newtonsoft.Json;

internal static class RateScenario
{
    internal static RateReport Run(LabWorld world, RateOptions options, string root)
    {
        var probe = new RateProbe(world.Simulator, options.Manifest);
        world.Advance(options.WarmupSeconds, 10, probe.BeforeTick, probe.AfterTick);
        probe.ResetMeters();
        world.Advance(options.Seconds, 10, probe.BeforeTick, probe.AfterTick);
        var report = probe.Report(options.Seconds);
        var json = JsonConvert.SerializeObject(report, Formatting.Indented);
        File.WriteAllText(Path.Combine(root, "rate-report.json"), json);
        if (report.MinShapeRatio < options.MinimumRatio || report.MinFluidRatio < options.MinimumRatio)
            throw new InvalidOperationException("Full-rate test failed: " + json);
        return report;
    }
}
